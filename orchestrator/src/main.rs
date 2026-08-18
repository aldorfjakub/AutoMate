use std::{env, result, str::FromStr, time::Duration};

use dotenvy::dotenv;
use redis::{AsyncCommands, AsyncConnectionConfig, Client, aio::MultiplexedConnection};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use tokio::time::sleep;
use uuid::Uuid;

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

    let config = AsyncConnectionConfig::new().set_connection_timeout(Some(Duration::from_secs(5))).set_response_timeout(Some(Duration::from_secs(10)));
    let mut conn = client.get_multiplexed_async_connection_with_config(&config).await.unwrap();
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

    let res = sqlx::query!("UPDATE bots SET is_valid = ? WHERE id = ?", true, bot_id)
        .execute(&db_pool)
        .await;

    match res {
        Ok(res) => {
            if (res.rows_affected() == 1){
                println!("validated");
                let _: Result<(), redis::RedisError> =
                    redis_conn.set_ex(job_id, "validated", 600).await;
            }
            else{
                println!("failed no such bot");
                let _: Result<(), redis::RedisError> =
                    redis_conn.set_ex(job_id, "failed", 600).await;
            }
            
        }

        Err(_) => {
            println!("failed");

            let _: Result<(), redis::RedisError> = redis_conn.set_ex(job_id, "failed", 600).await;
        }
    }
}
