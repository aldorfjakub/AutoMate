use std::{
    env::{self},
    process::Stdio,
    str::FromStr,
    time::Duration,
};

use dotenvy::dotenv;
use redis::{AsyncCommands, AsyncConnectionConfig, Client, aio::MultiplexedConnection};
use serde::{Deserialize, Serialize};
use shakmaty::{
    Chess, EnPassantMode, KnownOutcome, Outcome, Position, fen::Fen, san::San, uci::UciMove,
};
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStderr, ChildStdin, ChildStdout, Command},
    time::{Instant, sleep, timeout, timeout_at},
};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum ValidationStatus {
    Pending,
    Running,
    Validated,
    Failed { reason: String },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Job {
    Validate {
        bot_id: String,
        job_id: String,
    },
    Match {
        match_id: String,
        white_bot_id: String,
        black_bot_id: String,
    },
}

#[derive(Serialize, Deserialize)]
pub struct BotInfo {
    pub id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub source_code: Option<String>,
    pub is_active: bool,
    pub is_public: bool,
    pub is_valid: bool,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "status")]
pub enum MatchStatus {
    Pending,
    Running,
    Finished { winner: String },
    Failed { reason: String },
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    // Need to build a loop that pulls the jobs from redis and then calls the function for specific task
    let client = Client::open(env::var("REDIS_URL").expect("REDIS_URL must be set"))
        .expect("Failed to create redis client");

    let config = AsyncConnectionConfig::new()
        .set_connection_timeout(Some(Duration::from_secs(5)))
        .set_response_timeout(Some(Duration::from_secs(10)));
    let mut conn = client
        .get_multiplexed_async_connection_with_config(&config)
        .await
        .unwrap();
    let db_pool = init_pool().await;
    loop {
        let res: Result<Option<(String, String)>, redis::RedisError> =
            conn.brpop("job_queue", 2.0).await;

        match res {
            Ok(Some((_queue, payload))) => {
                let job: Job = match serde_json::from_str(&payload) {
                    Ok(j) => j,
                    Err(err) => {
                        eprintln!("Failed to deserialize job: {err}. Payload: {payload}");
                        continue;
                    }
                };

                match job {
                    Job::Validate { bot_id, job_id } => {
                        let bot_uuid = match Uuid::from_str(&bot_id) {
                            Ok(uuid) => uuid,
                            Err(err) => {
                                eprintln!("Invalid bot UUID '{bot_id}': {err}");
                                continue;
                            }
                        };

                        let pool_copy = db_pool.clone();
                        let conn_copy = conn.clone();

                        tokio::spawn(async move {
                            validate_bot(bot_uuid, job_id, pool_copy, conn_copy).await;
                        });
                    }
                    Job::Match {
                        match_id,
                        white_bot_id,
                        black_bot_id,
                    } => {
                        let white_bot_id = match Uuid::from_str(&white_bot_id) {
                            Ok(uuid) => uuid,
                            Err(err) => {
                                eprintln!("Invalid bot UUID '{white_bot_id}': {err}");
                                continue;
                            }
                        };

                        let black_bot_id = match Uuid::from_str(&black_bot_id) {
                            Ok(uuid) => uuid,
                            Err(err) => {
                                eprintln!("Invalid bot UUID '{black_bot_id}': {err}");
                                continue;
                            }
                        };
                        let conn_copy = conn.clone();
                        let pool_copy = db_pool.clone();

                        tokio::spawn(async move {
                            play_match(&match_id, white_bot_id, black_bot_id, pool_copy, conn_copy)
                                .await;
                        });
                    }
                    _ => {
                        eprintln!("Unhandled job variant");
                    }
                }
            }
            Ok(None) => {
                // Normal 5s timeout, no jobs available
                continue;
            }
            Err(err) => {
                eprintln!("Redis BRPOP error: {err}. Retrying in 2 seconds...");
                sleep(Duration::from_secs(2)).await;
            }
        }
    }
}

async fn init_pool() -> SqlitePool {
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .expect("Failed to create pool");

    pool
}

async fn publish_validation_status(
    redis_conn: &mut MultiplexedConnection,
    job_id: &str,
    status: ValidationStatus,
) {
    let json = serde_json::to_string(&status).expect("serialize ValidationStatus");
    let _: Result<(), redis::RedisError> = redis_conn.set_ex(job_id, json, 600).await;
}

async fn publish_match_status(
    redis_conn: &mut MultiplexedConnection,
    job_id: &str,
    status: MatchStatus,
) {
    let json = serde_json::to_string(&status).expect("serialize ValidationStatus");
    let _: Result<(), redis::RedisError> = redis_conn.set_ex(job_id, json, 600).await;
}

enum MatchEndReason {
    Checkmate,
    Draw,
    IllegalMove,
    Timeout,
    WriteError,
    InvalidOutput,
}

impl MatchEndReason {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Checkmate => "checkmate",
            Self::Draw => "draw",
            Self::IllegalMove => "illegal_move",
            Self::Timeout => "timeout",
            Self::WriteError => "write_error",
            Self::InvalidOutput => "invalid_output",
        }
    }
}

struct MatchConclusion {
    winner_color: Option<&'static str>,
    win_reason: &'static str,
    pgn: String,
    error_message: Option<String>,
}

async fn conclude_match(db_pool: &SqlitePool, match_uuid: Uuid, conclusion: &MatchConclusion) {
    let _ = sqlx::query!(
        r#"UPDATE matches SET
             match_status = 'finished',
             winner_color = ?,
             win_reason = ?,
             pgn = ?,
             error_message = ?,
             completed_at = CURRENT_TIMESTAMP
           WHERE id = ?"#,
        conclusion.winner_color,
        conclusion.win_reason,
        conclusion.pgn,
        conclusion.error_message,
        match_uuid,
    )
    .execute(db_pool)
    .await;
}

fn build_pgn(white_name: &str, black_name: &str, result: &str, moves: &[String]) -> String {
    let mut pgn = format!(
        "[Event \"?\"]\n[White \"{white_name}\"]\n[Black \"{black_name}\"]\n[Result \"{result}\"]\n\n"
    );
    for (idx, chunk) in moves.chunks(2).enumerate() {
        if idx > 0 {
            pgn.push('\n');
        }
        pgn.push_str(&format!("{}. {}", idx + 1, chunk.join(" ")));
    }
    pgn
}

fn result_for_color(winner_color: Option<&str>) -> &'static str {
    match winner_color {
        Some("white") => "1-0",
        Some("black") => "0-1",
        _ => "1/2-1/2",
    }
}

fn bot_failure_winner(
    current_bot_id: Option<Uuid>,
    white_bot_id: Uuid,
    black_bot_id: Uuid,
) -> (&'static str, Uuid) {
    if current_bot_id == Some(white_bot_id) {
        ("black", black_bot_id)
    } else {
        ("white", white_bot_id)
    }
}

async fn finish_match(
    db_pool: &SqlitePool,
    redis_conn: &mut MultiplexedConnection,
    match_id: &str,
    match_uuid: Uuid,
    redis_winner: String,
    conclusion: MatchConclusion,
) {
    publish_match_status(
        redis_conn,
        match_id,
        MatchStatus::Finished {
            winner: redis_winner,
        },
    )
    .await;
    conclude_match(db_pool, match_uuid, &conclusion).await;
}

struct PreparedBot {
    bot_id: Option<Uuid>,
    docker_name: String,
    stdin: ChildStdin,
    reader: BufReader<ChildStdout>,
    stderr_reader: BufReader<ChildStderr>,
    process: Child,
}
/// Spawn the python-runner container, upload the bot's source code, and wait
/// for the worker to signal READY. Returns a [`PreparedBot`] usable for
/// validation or a match, or `Err(reason)` if anything during setup fails.
async fn prepare_bot(source_code: &str, bot_id: Option<Uuid>) -> Result<PreparedBot, String> {
    let docker_name = format!("bot-{}", Uuid::new_v4());
    let mut process = Command::new("docker")
        .args(&[
            "run",
            "-i",
            "--rm",
            "--name",
            &docker_name,
            "--read-only",
            "--tmpfs",
            "/tmp:rw,noexec,nosuid,nodev,size=64m",
            "--cap-drop=ALL",
            "--security-opt=no-new-privileges:true",
            "--network=none",
            "--memory=256m",
            "--memory-swap=256m",
            "--cpus=0.5",
            "--pids-limit=64",
            "python-runner",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| format!("failed to spawn container: {e}"))?;

    let mut stdin = match process.stdin.take() {
        Some(s) => s,
        None => {
            let _ = process.kill().await;
            return Err("container has no stdin pipe".to_string());
        }
    };
    let stdout = process.stdout.take().expect("missing stdout pipe");
    let stderr = process.stderr.take().expect("missing stderr pipe");
    let mut reader = BufReader::new(stdout);
    let stderr_reader = BufReader::new(stderr);

    // Upload the source code, terminated by the sentinel the wrapper reads.
    let source_bytes = source_code.as_bytes();
    if stdin.write_all(source_bytes).await.is_err()
        || stdin.write_all(b"\n").await.is_err()
        || stdin
            .write_all(b"===END_OF_WORKER_CODE===\n")
            .await
            .is_err()
    {
        kill_bot(&mut PreparedBot {
            bot_id,
            docker_name,
            stdin,
            reader,
            stderr_reader,
            process,
        })
        .await;
        return Err("failed to send code to worker".to_string());
    }
    let _ = stdin.flush().await;

    // TODO check if this container can't get stuck by keep printing stuff and stuck in loop
    // Wait for the worker to finish loading its code and print READY.
    let mut line = String::new();
    loop {
        line.clear();
        match timeout(Duration::from_secs(10), reader.read_line(&mut line)).await {
            Ok(Ok(0)) => {
                return Err("worker exited before READY".to_string());
            }
            Ok(Ok(_)) => (),
            Ok(Err(e)) => {
                return Err(format!("read error while loading worker: {e}"));
            }
            Err(_) => {
                return Err("worker code took too long to load".to_string());
            }
        }
        let trimmed = line.trim();
        if trimmed == "READY" {
            break;
        }
        if trimmed.starts_with("ERROR") {
            println!("{}", trimmed);
            return Err("worker failed to load".to_string());
        }
    }

    Ok(PreparedBot {
        bot_id,
        docker_name,
        stdin,
        reader,
        stderr_reader,
        process,
    })
}

async fn kill_bot(prepared: &mut PreparedBot) {
    if let Ok(mut child) = Command::new("docker")
        .args(["kill", &prepared.docker_name])
        .spawn()
    {
        let _ = timeout(Duration::from_secs(5), child.wait()).await;
    }
    let _ = Command::new("docker")
        .args(["rm", "--force", &prepared.docker_name])
        .spawn();

    let _ = prepared.process.kill().await;
    let _ = timeout(Duration::from_secs(2), prepared.process.wait()).await;
}

async fn play_match(
    match_id: &str,
    white_bot_id: Uuid,
    black_bot_id: Uuid,
    db_pool: SqlitePool,
    mut redis_conn: MultiplexedConnection,
) {
    let match_uuid = Uuid::from_str(&match_id).expect("Failed to make uuid out of match_id");
    let match_id = format!("match_{}", match_id);
    let white_bot_info = match sqlx::query_as!(BotInfo,r#"SELECT id as "id: uuid::Uuid", name, description, is_active, is_public, source_code, is_valid FROM bots WHERE id = ?"#, &white_bot_id).fetch_optional(&db_pool).await
    {
        Ok(Some(bot1)) => bot1,
        Ok(_) => {
            // TODO proper error message to show which bot is the cause, it is very niche error that will probably won't happen but still would be better.
            publish_match_status(&mut redis_conn, &match_id, MatchStatus::Failed {reason: "No such bot".to_string()}).await;
            return;
        }
        _ => {
            publish_match_status(&mut redis_conn, &match_id, MatchStatus::Failed {reason: "Internal error".to_string()}).await;
            return;
        }
    };
    let white_bot_source_code = match white_bot_info.source_code {
        Some(code) => code,
        None => {
            publish_match_status(
                &mut redis_conn,
                &match_id,
                MatchStatus::Failed {
                    reason: "Bot has no source code".to_string(),
                },
            )
            .await;
            return;
        }
    };

    let black_bot_info: BotInfo = match sqlx::query_as!(BotInfo,r#"SELECT id as "id: uuid::Uuid", name, description, is_active, is_public, source_code, is_valid FROM bots WHERE id = ?"#, &black_bot_id).fetch_optional(&db_pool).await
    {
        Ok(Some(bot2)) => bot2,
        Ok(_) => {
            // TODO proper error message to show which bot is the cause, it is very niche error that will probably won't happen but still would be better.
            publish_match_status(&mut redis_conn, &match_id, MatchStatus::Failed {reason: "No such bot".to_string()}).await;
            return;
        }
        _ => {
            publish_match_status(&mut redis_conn, &match_id, MatchStatus::Failed {reason: "Internal error".to_string()}).await;
            return;
        }
    };
    let black_bot_source_code = match black_bot_info.source_code {
        Some(code) => code,
        None => {
            publish_match_status(
                &mut redis_conn,
                &match_id,
                MatchStatus::Failed {
                    reason: "Bot has no source code".to_string(),
                },
            )
            .await;
            return;
        }
    };

    println!(
        "Creating docker for bot 1 of size: {}",
        white_bot_source_code.len()
    );

    let mut white_bot = match prepare_bot(&white_bot_source_code, white_bot_info.id).await {
        Ok(bot) => bot,
        Err(reason) => {
            let _ = sqlx::query!(
                "UPDATE matches SET match_status = 'failed', error_message = ? WHERE id = ?",
                format!("Bot: {} failed to load: {}", white_bot_info.name, reason),
                match_uuid
            )
            .execute(&db_pool)
            .await;
            publish_match_status(&mut redis_conn, &match_id, MatchStatus::Failed { reason }).await;
            return;
        }
    };

    let mut black_bot = match prepare_bot(&black_bot_source_code, black_bot_info.id).await {
        Ok(bot) => bot,
        Err(reason) => {
            kill_bot(&mut white_bot).await;
            let _ = sqlx::query!(
                "UPDATE matches SET match_status = 'failed', error_message = ? WHERE id = ?",
                format!("Bot: {} failed to load: {}", black_bot_info.name, reason),
                match_uuid
            )
            .execute(&db_pool)
            .await;
            publish_match_status(&mut redis_conn, &match_id, MatchStatus::Failed { reason }).await;
            return;
        }
    };

    let _ = sqlx::query!(
        "UPDATE matches SET match_status = 'playing' WHERE id = ?",
        match_uuid
    )
    .execute(&db_pool)
    .await;

    let mut pos = Chess::default();
    let mut moves: Vec<String> = Vec::new();

    let mut current_bot = &mut white_bot;
    loop {
        match pos.outcome() {
            Outcome::Known(KnownOutcome::Decisive { winner }) => {
                if winner.is_white() {
                    let pgn = build_pgn(&white_bot_info.name, &black_bot_info.name, "1-0", &moves);
                    finish_match(
                        &db_pool,
                        &mut redis_conn,
                        &match_id,
                        match_uuid,
                        white_bot_id.to_string(),
                        MatchConclusion {
                            winner_color: Some("white"),
                            win_reason: MatchEndReason::Checkmate.as_str(),
                            pgn,
                            error_message: None,
                        },
                    )
                    .await;
                } else {
                    let pgn = build_pgn(&white_bot_info.name, &black_bot_info.name, "0-1", &moves);
                    finish_match(
                        &db_pool,
                        &mut redis_conn,
                        &match_id,
                        match_uuid,
                        black_bot_id.to_string(),
                        MatchConclusion {
                            winner_color: Some("black"),
                            win_reason: MatchEndReason::Checkmate.as_str(),
                            pgn,
                            error_message: None,
                        },
                    )
                    .await;
                }
                break;
            }
            Outcome::Known(KnownOutcome::Draw) => {
                let pgn = build_pgn(
                    &white_bot_info.name,
                    &black_bot_info.name,
                    "1/2-1/2",
                    &moves,
                );
                finish_match(
                    &db_pool,
                    &mut redis_conn,
                    &match_id,
                    match_uuid,
                    String::new(),
                    MatchConclusion {
                        winner_color: None,
                        win_reason: MatchEndReason::Draw.as_str(),
                        pgn,
                        error_message: None,
                    },
                )
                .await;
                break;
            }
            Outcome::Unknown => (),
        }
        let fen = Fen::from_position(&pos, EnPassantMode::Legal).to_string();
        if current_bot.stdin.write_all(fen.as_bytes()).await.is_err()
            || current_bot.stdin.write_all(b"\n").await.is_err()
        {
            let (winner_color, redis_winner) =
                bot_failure_winner(current_bot.bot_id, white_bot_id, black_bot_id);
            let pgn = build_pgn(
                &white_bot_info.name,
                &black_bot_info.name,
                result_for_color(Some(winner_color)),
                &moves,
            );
            finish_match(
                &db_pool,
                &mut redis_conn,
                &match_id,
                match_uuid,
                redis_winner.to_string(),
                MatchConclusion {
                    winner_color: Some(winner_color),
                    win_reason: MatchEndReason::WriteError.as_str(),
                    pgn,
                    error_message: Some("failed to write position to bot".to_string()),
                },
            )
            .await;
            break;
            // Bot has failed
        }
        let _ = current_bot.stdin.flush().await;

        let uci = match read_move(&mut current_bot.reader).await {
            Ok(u) => u,
            Err(reason) => {
                let (winner_color, redis_winner) =
                    bot_failure_winner(current_bot.bot_id, white_bot_id, black_bot_id);
                let pgn = build_pgn(
                    &white_bot_info.name,
                    &black_bot_info.name,
                    result_for_color(Some(winner_color)),
                    &moves,
                );
                finish_match(
                    &db_pool,
                    &mut redis_conn,
                    &match_id,
                    match_uuid,
                    redis_winner.to_string(),
                    MatchConclusion {
                        winner_color: Some(winner_color),
                        win_reason: MatchEndReason::Timeout.as_str(),
                        pgn,
                        error_message: Some(reason),
                    },
                )
                .await;
                break;
            }
        };

        let uci = uci.trim();
        match uci.parse::<UciMove>() {
            Ok(uci_move) => match uci_move.to_move(&pos) {
                Ok(m) => {
                    let san = San::from_move(&pos, m).to_string();
                    pos.play_unchecked(m);
                    moves.push(san);
                }
                Err(_) => {
                    println!("invalid move from worker: {uci}");
                    let (winner_color, redis_winner) =
                        bot_failure_winner(current_bot.bot_id, white_bot_id, black_bot_id);
                    let pgn = build_pgn(
                        &white_bot_info.name,
                        &black_bot_info.name,
                        result_for_color(Some(winner_color)),
                        &moves,
                    );
                    finish_match(
                        &db_pool,
                        &mut redis_conn,
                        &match_id,
                        match_uuid,
                        redis_winner.to_string(),
                        MatchConclusion {
                            winner_color: Some(winner_color),
                            win_reason: MatchEndReason::IllegalMove.as_str(),
                            pgn,
                            error_message: Some(format!("illegal move: {uci}")),
                        },
                    )
                    .await;
                    break;
                }
            },
            Err(_) => {
                println!("unparseable move from worker: {uci}");
                let (winner_color, redis_winner) =
                    bot_failure_winner(current_bot.bot_id, white_bot_id, black_bot_id);
                let pgn = build_pgn(
                    &white_bot_info.name,
                    &black_bot_info.name,
                    result_for_color(Some(winner_color)),
                    &moves,
                );
                finish_match(
                    &db_pool,
                    &mut redis_conn,
                    &match_id,
                    match_uuid,
                    redis_winner.to_string(),
                    MatchConclusion {
                        winner_color: Some(winner_color),
                        win_reason: MatchEndReason::InvalidOutput.as_str(),
                        pgn,
                        error_message: Some(format!("unparseable move: {uci}")),
                    },
                )
                .await;
                break;
            }
        }

        if current_bot.bot_id == Some(white_bot_id) {
            current_bot = &mut black_bot;
        } else {
            current_bot = &mut white_bot;
        }
    }

    kill_bot(&mut white_bot).await;
    kill_bot(&mut black_bot).await;

    let _ = (&white_bot, &black_bot);
}

async fn read_move(reader: &mut BufReader<ChildStdout>) -> Result<String, String> {
    let mut line = String::new();
    let deadline = Instant::now() + Duration::from_millis(1200);
    for _ in 0..10 {
        line.clear();
        match timeout_at(deadline, reader.read_line(&mut line)).await {
            Ok(Ok(0)) => {
                return Err("Bot exited".into());
            }
            Ok(Ok(_)) => (),
            Ok(Err(_)) => {
                return Err("failed to read a line".into());
            }
            Err(_) => {
                return Err("took too long to respond".into());
            }
        }

        if line.len() > 1_000_000 {
            return Err("line too large".into());
        }

        let trimmed = line.trim();
        if let Some(uci) = trimmed.strip_prefix("move:") {
            return Ok(uci.into());
        }

        if trimmed.starts_with("ERROR") {
            println!("{}", trimmed);
            return Err(trimmed.into());
        }
    }
    Err("Bot has outputed too many lines".into())
}

async fn validate_bot(
    bot_id: Uuid,
    job_id: String,
    db_pool: SqlitePool,
    mut redis_conn: MultiplexedConnection,
) {
    publish_validation_status(&mut redis_conn, &job_id, ValidationStatus::Running).await;

    let bot = match sqlx::query_as!(BotInfo,r#"SELECT id as "id: uuid::Uuid", name, description, is_active, is_public, source_code, is_valid FROM bots WHERE id = ?"#, &bot_id).fetch_optional(&db_pool).await
    {
        Ok(Some(bot)) => bot,
        Ok(_) => {
            publish_validation_status(&mut redis_conn, &job_id, ValidationStatus::Failed {reason: "No such bot".to_string()}).await;
            return;
        }
        _ => {
            publish_validation_status(&mut redis_conn, &job_id, ValidationStatus::Failed {reason: "Internal error".to_string()}).await;
            return;
        }
    };
    let source_code = match bot.source_code {
        Some(code) => code,
        None => {
            publish_validation_status(
                &mut redis_conn,
                &job_id,
                ValidationStatus::Failed {
                    reason: "Bot has no source code".to_string(),
                },
            )
            .await;
            return;
        }
    };
    println!("Executing code of size: {}", source_code.len());

    let mut prepared = match prepare_bot(&source_code, None).await {
        Ok(bot) => bot,
        Err(reason) => {
            publish_validation_status(
                &mut redis_conn,
                &job_id,
                ValidationStatus::Failed { reason },
            )
            .await;
            return;
        }
    };

    let mut valid = true;
    let mut timed_out = false;

    // Play out a match of up to 20 moves, verifying the worker's
    // moves are legal in the current position.
    let mut pos = Chess::new();
    for _ in 0..20 {
        if timed_out || !valid || pos.outcome() != Outcome::Unknown {
            break;
        }

        let fen = Fen::from_position(&pos, EnPassantMode::Legal).to_string();
        if prepared.stdin.write_all(fen.as_bytes()).await.is_err()
            || prepared.stdin.write_all(b"\n").await.is_err()
        {
            valid = false;
            break;
        }
        let _ = prepared.stdin.flush().await;

        let uci = match read_move(&mut prepared.reader).await {
            Ok(uci) => uci,
            Err(s) => {
                // TODO return the right error
                valid = false;
                break;
            }
        };

        let uci = uci.trim();
        match uci.parse::<UciMove>() {
            Ok(uci_move) => match uci_move.to_move(&pos) {
                Ok(m) => {
                    pos.play_unchecked(m);
                }
                Err(_) => {
                    println!("invalid move from worker: {uci}");
                    valid = false;
                }
            },
            Err(_) => {
                println!("unparseable move from worker: {uci}");
                valid = false;
            }
        }
    }

    if timed_out {
        println!("Took too long");
        publish_validation_status(
            &mut redis_conn,
            &job_id,
            ValidationStatus::Failed {
                reason: "Bot took over 1s to return a move".to_string(),
            },
        )
        .await;
        kill_bot(&mut prepared).await;

        return;
    }

    let _ = prepared.stdin.write_all(b"kill\n").await;
    let _ = prepared.stdin.flush().await;

    // Drain the remaining output before the process is reaped.
    let mut stdout_rest = String::new();
    match timeout(
        Duration::from_secs(2),
        prepared.reader.read_to_string(&mut stdout_rest),
    )
    .await
    {
        Ok(_) => (),
        Err(_) => {
            // Some logging later
        }
    };
    let mut stderr_rest = String::new();
    match timeout(
        Duration::from_secs(2),
        prepared.stderr_reader.read_to_string(&mut stderr_rest),
    )
    .await
    {
        Ok(_) => (),
        Err(_) => {
            // Some logging later
        }
    };
    if !stdout_rest.is_empty() {
        print!("{}", stdout_rest);
    }
    if !stderr_rest.is_empty() {
        eprint!("{}", stderr_rest);
    }

    kill_bot(&mut prepared).await;

    if valid {
        // Update the bot with the result, also checking if the code hasn't changed during the process
        let res = sqlx::query!(
            "UPDATE bots SET is_valid = ? WHERE id = ? AND source_code = ?",
            true,
            bot_id,
            &source_code
        )
        .execute(&db_pool)
        .await;
        match res {
            Ok(res) => {
                if res.rows_affected() == 1 {
                    println!("validated");
                    publish_validation_status(
                        &mut redis_conn,
                        &job_id,
                        ValidationStatus::Validated,
                    )
                    .await;
                } else {
                    println!("failed no such bot");
                    publish_validation_status(
                        &mut redis_conn,
                        &job_id,
                        ValidationStatus::Failed {
                            reason: "No such bot exists".to_string(),
                        },
                    )
                    .await;
                }
            }

            Err(_) => {
                println!("failed");
                publish_validation_status(
                    &mut redis_conn,
                    &job_id,
                    ValidationStatus::Failed {
                        reason: "Something fail :( (good enough for now)".to_string(),
                    },
                )
                .await;
            }
        }
    } else {
        publish_validation_status(
            &mut redis_conn,
            &job_id,
            ValidationStatus::Failed {
                reason: "Something fail :( (good enough for now)".to_string(),
            },
        )
        .await;
    }
}
