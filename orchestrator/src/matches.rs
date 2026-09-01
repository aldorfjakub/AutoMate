use std::{str::FromStr, time::Duration};

use redis::{AsyncCommands, aio::MultiplexedConnection};
use shakmaty::{
    Chess, EnPassantMode, KnownOutcome, Outcome, Position, fen::Fen, san::San, uci::UciMove,
};
use sqlx::sqlite::SqlitePool;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

use crate::{
    bot::{kill_bot, prepare_bot, read_move},
    models::*,
};

async fn publish_match_status(
    redis_conn: &mut MultiplexedConnection,
    job_id: &str,
    status: MatchStatus,
) {
    let json = serde_json::to_string(&status).expect("serialize MatchStatus");
    let _: Result<(), redis::RedisError> = redis_conn.set_ex(job_id, json, 600).await;
}

// TODO error message should show which bot is the cause
async fn fetch_bot_or_fail(
    redis_conn: &mut MultiplexedConnection,
    match_id: &str,
    db_pool: &SqlitePool,
    bot_id: Uuid,
) -> Result<BotInfo, ()> {
    match get_bot(db_pool, bot_id).await {
        Ok(Some(bot)) => Ok(bot),
        Ok(_) => {
            publish_match_status(redis_conn, match_id, MatchStatus::Failed {
                reason: "No such bot".to_string(),
            })
            .await;
            Err(())
        }
        _ => {
            publish_match_status(redis_conn, match_id, MatchStatus::Failed {
                reason: "Internal error".to_string(),
            })
            .await;
            Err(())
        }
    }
}

struct MatchCtx {
    redis_conn: MultiplexedConnection,
    db_pool: SqlitePool,
    match_id: String,
    match_uuid: Uuid,
    white_bot_id: Uuid,
    black_bot_id: Uuid,
    white_bot_name: String,
    black_bot_name: String,
}
enum MatchEndReason {
    Checkmate,
    Draw,
    IllegalMove,
    Timeout,
    WriteError,
    InvalidOutput,
    FailedToLoadBot,
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
            Self::FailedToLoadBot => "failed_to_load_bot",
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

async fn fail_match(
    ctx: &mut MatchCtx,
    current_bot_id: Option<Uuid>,
    reason: MatchEndReason,
    error_message: String,
) {
    let bot_name = if current_bot_id == Some(ctx.white_bot_id) {
        &ctx.white_bot_name
    } else {
        &ctx.black_bot_name
    };
    let _ = sqlx::query!(
        "UPDATE matches SET match_status = 'failed', error_message = ? WHERE id = ?",
        format!(
            "Bot: {} failed to load: {} : {}",
            bot_name,
            reason.as_str(),
            error_message
        ),
        ctx.match_uuid
    )
    .execute(&ctx.db_pool)
    .await;
    publish_match_status(
        &mut ctx.redis_conn,
        &ctx.match_id,
        MatchStatus::Failed {
            reason: reason.as_str().to_string(),
        },
    )
    .await;
}

async fn finish_match(ctx: &mut MatchCtx, redis_winner: String, conclusion: MatchConclusion) {
    publish_match_status(
        &mut ctx.redis_conn,
        &ctx.match_id,
        MatchStatus::Finished {
            winner: redis_winner,
        },
    )
    .await;

    let event = MatchEvent::Finished {
        winner: conclusion.winner_color.unwrap_or("").to_string(),
        reason: conclusion.win_reason.to_string(),
    };
    let json = serde_json::to_string(&event).expect("failed to serialize match event");
    let _: Result<(), redis::RedisError> = ctx
        .redis_conn
        .publish(format!("matchStream_{}", ctx.match_uuid.to_string()), json)
        .await;
    conclude_match(&ctx.db_pool, ctx.match_uuid, &conclusion).await;
}

async fn finish_match_failure(
    ctx: &mut MatchCtx,
    current_bot_id: Option<Uuid>,
    moves: &[String],
    reason: MatchEndReason,
    error_message: String,
) {
    let (winner_color, redis_winner) =
        bot_failure_winner(current_bot_id, ctx.white_bot_id, ctx.black_bot_id);
    let pgn = build_pgn(
        &ctx.white_bot_name,
        &ctx.black_bot_name,
        result_for_color(Some(winner_color)),
        moves,
    );
    finish_match(
        ctx,
        redis_winner.to_string(),
        MatchConclusion {
            winner_color: Some(winner_color),
            win_reason: reason.as_str(),
            pgn,
            error_message: Some(error_message),
        },
    )
    .await;
}

pub async fn play_match(
    match_id: &str,
    white_bot_id: Uuid,
    black_bot_id: Uuid,
    db_pool: SqlitePool,
    mut redis_conn: MultiplexedConnection,
) {
    let match_uuid = Uuid::from_str(&match_id).expect("Failed to make uuid out of match_id");
    let match_id = format!("match_{}", match_id);

let white_bot_info = match fetch_bot_or_fail(&mut redis_conn, &match_id, &db_pool, white_bot_id).await {
        Ok(bot) => bot,
        Err(_) => return,
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

let black_bot_info = match fetch_bot_or_fail(&mut redis_conn, &match_id, &db_pool, black_bot_id).await {
        Ok(bot) => bot,
        Err(_) => return,
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

    let mut ctx = MatchCtx {
        redis_conn,
        db_pool,
        match_id,
        match_uuid,
        white_bot_id,
        black_bot_id,
        white_bot_name: white_bot_info.name,
        black_bot_name: black_bot_info.name,
    };

    let mut white_bot = match prepare_bot(&white_bot_source_code, white_bot_info.id).await {
        Ok(bot) => bot,
        Err(reason) => {
            fail_match(
                &mut ctx,
                white_bot_info.id,
                MatchEndReason::FailedToLoadBot,
                reason,
            )
            .await;
            return;
        }
    };

    let mut black_bot = match prepare_bot(&black_bot_source_code, black_bot_info.id).await {
        Ok(bot) => bot,
        Err(reason) => {
            kill_bot(&mut white_bot).await;
            fail_match(
                &mut ctx,
                black_bot_info.id,
                MatchEndReason::FailedToLoadBot,
                reason.clone(),
            )
            .await;
            let event: MatchEvent = MatchEvent::Failed { reason };
            let json = serde_json::to_string(&event).expect("failed to serialize match event");
            let _: Result<(), redis::RedisError> = ctx
                .redis_conn
                .publish(format!("matchStream_{}", ctx.match_uuid.to_string()), json)
                .await;
            return;
        }
    };

    let _ = sqlx::query!(
        "UPDATE matches SET match_status = 'playing' WHERE id = ?",
        ctx.match_uuid
    )
    .execute(&ctx.db_pool)
    .await;

    let mut pos = Chess::default();
    let mut moves: Vec<String> = Vec::new();

    let mut current_bot = &mut white_bot;
    loop {
        match pos.outcome() {
            Outcome::Known(KnownOutcome::Decisive { winner }) => {
                if winner.is_white() {
                    let pgn = build_pgn(&ctx.white_bot_name, &ctx.black_bot_name, "1-0", &moves);
                    finish_match(
                        &mut ctx,
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
                    let pgn = build_pgn(&ctx.white_bot_name, &ctx.black_bot_name, "0-1", &moves);
                    finish_match(
                        &mut ctx,
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
                let pgn = build_pgn(&ctx.white_bot_name, &ctx.black_bot_name, "1/2-1/2", &moves);
                finish_match(
                    &mut ctx,
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
            finish_match_failure(
                &mut ctx,
                current_bot.bot_id,
                &moves,
                MatchEndReason::WriteError,
                "failed to write position to bot".to_string(),
            )
            .await;
            break;
        }
        let _ = current_bot.stdin.flush().await;

        let uci = match read_move(&mut current_bot.reader).await {
            Ok(u) => u,
            Err(reason) => {
                finish_match_failure(
                    &mut ctx,
                    current_bot.bot_id,
                    &moves,
                    MatchEndReason::Timeout,
                    reason,
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
                    tokio::time::sleep(Duration::from_secs(1)).await;

                    pos.play_unchecked(m);
                    moves.push(san.clone());
                    let event = MatchEvent::Move {
                        san: san,
                        fen: Fen::from_position(&pos, EnPassantMode::Legal).to_string(),
                        move_number: moves.len() as u32,
                    };
                    let json =
                        serde_json::to_string(&event).expect("failed to serialize match event");
                    let _: Result<(), redis::RedisError> = ctx
                        .redis_conn
                        .publish(format!("matchStream_{}", ctx.match_uuid.to_string()), json)
                        .await;
                }
                Err(_) => {
                    println!("invalid move from worker: {uci}");
                    finish_match_failure(
                        &mut ctx,
                        current_bot.bot_id,
                        &moves,
                        MatchEndReason::IllegalMove,
                        format!("illegal move: {uci}"),
                    )
                    .await;
                    break;
                }
            },
            Err(_) => {
                println!("unparseable move from worker: {uci}");
                finish_match_failure(
                    &mut ctx,
                    current_bot.bot_id,
                    &moves,
                    MatchEndReason::InvalidOutput,
                    format!("unparseable move: {uci}"),
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
}
