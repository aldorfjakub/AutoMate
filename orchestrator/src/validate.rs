use std::time::Duration;

use redis::{AsyncCommands, aio::MultiplexedConnection};
use shakmaty::{Chess, EnPassantMode, Outcome, Position, fen::Fen, uci::UciMove};
use sqlx::sqlite::SqlitePool;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    time::timeout,
};
use uuid::Uuid;

use crate::{
    bot::{kill_bot, prepare_bot, read_move},
    models::*,
};

async fn publish_validation_status(
    redis_conn: &mut MultiplexedConnection,
    job_id: &str,
    status: ValidationStatus,
) {
    let json = serde_json::to_string(&status).expect("serialize ValidationStatus");
    let _: Result<(), redis::RedisError> = redis_conn.set_ex(job_id, json, 600).await;
}

async fn fetch_bot_or_fail(
    redis_conn: &mut MultiplexedConnection,
    job_id: &str,
    db_pool: &SqlitePool,
    bot_id: Uuid,
) -> Result<BotInfo, ()> {
    match get_bot(db_pool, bot_id).await {
        Ok(Some(bot)) => Ok(bot),
        Ok(_) => {
            publish_validation_status(
                redis_conn,
                job_id,
                ValidationStatus::Failed {
                    reason: "No such bot".to_string(),
                },
            )
            .await;
            Err(())
        }
        _ => {
            publish_validation_status(
                redis_conn,
                job_id,
                ValidationStatus::Failed {
                    reason: "Internal error".to_string(),
                },
            )
            .await;
            Err(())
        }
    }
}

pub async fn validate_bot(
    bot_id: Uuid,
    job_id: String,
    db_pool: SqlitePool,
    mut redis_conn: MultiplexedConnection,
) {
    publish_validation_status(&mut redis_conn, &job_id, ValidationStatus::Running).await;

    let bot = match fetch_bot_or_fail(&mut redis_conn, &job_id, &db_pool, bot_id).await {
        Ok(bot) => bot,
        Err(_) => return,
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
    let mut invalid_reason = String::new();
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
            invalid_reason = "failed to write data for next move".to_string();
            break;
        }
        let _ = prepared.stdin.flush().await;

        let uci = match read_move(&mut prepared.reader).await {
            Ok(uci) => uci,
            Err(s) => {
                // TODO return the right error
                valid = false;
                invalid_reason = s;
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
                    invalid_reason = format!("invalid move from bot: {uci}");
                }
            },
            Err(_) => {
                println!("unparseable move from worker: {uci}");
                invalid_reason = format!("unparseable move from bot: {uci}");
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

            Err(e) => {
                println!("failed");
                publish_validation_status(
                    &mut redis_conn,
                    &job_id,
                    ValidationStatus::Failed {
                        reason: e.to_string(),
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
                reason: invalid_reason,
            },
        )
        .await;
    }
}
