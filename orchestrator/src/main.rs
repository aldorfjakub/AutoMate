use std::{env, process::Stdio, str::FromStr, time::Duration};

use dotenvy::dotenv;
use redis::{AsyncCommands, AsyncConnectionConfig, Client, aio::MultiplexedConnection};
use serde::{Deserialize, Serialize};
use shakmaty::{Chess, EnPassantMode, Outcome, Position, fen::Fen, uci::UciMove};
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt},
    process::Command,
    time::sleep,
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
        bot1_id: String,
        bot2_id: String,
    },
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

async fn validate_bot(
    bot_id: Uuid,
    job_id: String,
    db_pool: SqlitePool,
    mut redis_conn: MultiplexedConnection,
) {
    //sleep(tokio::time::Duration::from_secs(5)).await;
    let a = r#"import chess
import random

number = 1

def get_chess_move(fen: str):
    global number
    board = chess.Board(fen)
    legal_moves = list(board.generate_legal_moves())
    captures = list(board.generate_legal_captures())

    for move in legal_moves:
        board.push(move)
        if board.is_checkmate():
            return move.uci()
        board.pop()
    for move in legal_moves:
        if "q" in move.uci().lower():
            return move.uci()
            
    if captures:
        return random.choice(captures).uci()
        
    return random.choice(legal_moves).uci()
"#
    .to_string();

    println!("Executing code of size: {}", a.len());

    let mut process = Command::new("docker")
        .args(&[
            "run",
            "-i",
            "--rm",
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
        .spawn()
        .unwrap();

    let a_bytes = a.as_bytes();

    let mut valid = true;
    if let Some(mut stdin) = process.stdin.take() {
        stdin.write_all(&a_bytes).await.unwrap();
        stdin.write_all(b"\n").await.unwrap();

        stdin
            .write_all(b"===END_OF_WORKER_CODE===\n")
            .await
            .unwrap();
        stdin.flush().await.unwrap();

        let stdout = process.stdout.take().unwrap();
        let stderr = process.stderr.take().unwrap();
        let mut reader = tokio::io::BufReader::new(stdout);
        let mut stderr_reader = tokio::io::BufReader::new(stderr);
        let mut line = String::new();

        // Wait for the worker to signal that it finished loading.
        loop {
            line.clear();
            let n = reader.read_line(&mut line).await.unwrap();
            if n == 0 {
                valid = false;
                break;
            }
            let trimmed = line.trim();
            if trimmed == "READY" {
                break;
            }
            if trimmed.starts_with("ERROR") {
                println!("{}", trimmed);
                valid = false;
                break;
            }
        }

        // Play out a match of up to 20 moves, verifying the worker's
        // moves are legal in the current position.
        let mut pos = Chess::new();
        for _ in 0..20 {
            if !valid || pos.outcome() != Outcome::Unknown {
                break;
            }

            let fen = Fen::from_position(&pos, EnPassantMode::Legal).to_string();
            stdin.write_all(fen.as_bytes()).await.unwrap();
            stdin.write_all(b"\n").await.unwrap();
            stdin.flush().await.unwrap();

            loop {
                line.clear();
                let n = reader.read_line(&mut line).await.unwrap();
                if n == 0 {
                    valid = false;
                    break;
                }
                let trimmed = line.trim();
                if let Some(uci) = trimmed.strip_prefix("move:") {
                    let uci = uci.trim();
                    match uci.parse::<UciMove>() {
                        Ok(uci_move) => match uci_move.to_move(&pos) {
                            Ok(m) => pos.play_unchecked(m),
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
                    break;
                }
                if trimmed.starts_with("ERROR") {
                    println!("{}", trimmed);
                    valid = false;
                    break;
                }
            }
        }

        let _ = stdin.write_all(b"kill\n").await;
        let _ = stdin.flush().await;
        drop(stdin);

        // Drain the remaining output before the process is reaped.
        let mut stdout_rest = String::new();
        let _ = reader.read_to_string(&mut stdout_rest).await;
        let mut stderr_rest = String::new();
        let _ = stderr_reader.read_to_string(&mut stderr_rest).await;

        if !stdout_rest.is_empty() {
            print!("{}", stdout_rest);
        }
        if !stderr_rest.is_empty() {
            eprint!("{}", stderr_rest);
        }
    } else {
        valid = false;
    }
    let _ = process.wait().await;

    if valid {
        let res = sqlx::query!("UPDATE bots SET is_valid = ? WHERE id = ?", true, bot_id)
            .execute(&db_pool)
            .await;
        match res {
            Ok(res) => {
                if res.rows_affected() == 1 {
                    println!("validated");
                    let _: Result<(), redis::RedisError> = redis_conn
                        .set_ex(
                            job_id,
                            serde_json::to_string(&ValidationStatus::Validated).unwrap(),
                            600,
                        )
                        .await;
                } else {
                    println!("failed no such bot");
                    let _: Result<(), redis::RedisError> = redis_conn
                        .set_ex(
                            job_id,
                            serde_json::to_string(&ValidationStatus::Failed {
                                reason: "No such bot exists".to_string(),
                            })
                            .unwrap(),
                            600,
                        )
                        .await;
                }
            }

            Err(_) => {
                println!("failed");

                let _: Result<(), redis::RedisError> = redis_conn
                    .set_ex(
                        job_id,
                        serde_json::to_string(&ValidationStatus::Failed {
                            reason: "Something fail :( (good enough for now)".to_string(),
                        })
                        .unwrap(),
                        600,
                    )
                    .await;
            }
        }
    } else {
        let _: Result<(), redis::RedisError> = redis_conn
            .set_ex(
                job_id,
                serde_json::to_string(&ValidationStatus::Failed {
                    reason: "Something fail :( (good enough for now)".to_string(),
                })
                .unwrap(),
                600,
            )
            .await;
    }
}
