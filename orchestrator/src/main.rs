use std::{
    env::{self},
    str::FromStr,
    time::Duration,
};

use dotenvy::dotenv;
use redis::{AsyncCommands, AsyncConnectionConfig, Client};

use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use tokio::{
    process::{Child, ChildStderr, ChildStdin, ChildStdout},
    time::{Instant, sleep, timeout_at},
};
use uuid::Uuid;

use crate::{matches::play_match, models::Job, validate::validate_bot};

mod bot;
mod matches;
mod models;
mod validate;

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
                        is_ranked,
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
                            play_match(&match_id, white_bot_id, black_bot_id, pool_copy, conn_copy, is_ranked)
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
                //eprintln!("Redis BRPOP error: {err}. Retrying in 2 seconds...");
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
