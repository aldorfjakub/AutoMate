use std::{env, time::Duration};

use crate::routes::app_routes;
use dotenvy::dotenv;
use redis::Client;
use tokio::time::sleep;

mod db;
mod error;
mod extract;
mod handlers;
mod models;
mod routes;
mod state;
#[tokio::main]
async fn main() {
    dotenv().ok();

    let db_pool = db::db::init_pool().await;

    let redis_client = Client::open(env::var("REDIS_URL").expect("REDIS_URL must be set"))
        .expect("Failed to create Redis client");
    let redis_connection = redis_client
        .get_multiplexed_async_connection()
        .await
        .expect("Failed to connect to Redis");
    let app_state = state::AppState {
        sqlite_pool: db_pool.clone(),
        redis_client: redis_client,
        redis_con: redis_connection,
        oauth_client_id: env::var("OAUTH_CLIENT_ID").unwrap(),
        oauth_client_secret: env::var("OAUTH_CLIENT_SECRET").unwrap(),
        oauth_callback_url: env::var("OAUTH_CALLBACK").unwrap(),
    };

    let cleanup_pool = db_pool.clone();
    tokio::spawn(async move {
        loop {
            sleep(Duration::from_hours(2)).await;
            let _ = sqlx::query!("DELETE FROM sessions WHERE expires_at < CURRENT_TIMESTAMP")
                .execute(&cleanup_pool)
                .await;
        }
    });

    let app = app_routes(app_state);
    // run it
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    println!("app running at {}", listener.local_addr().unwrap());
    let _ = axum::serve(listener, app).await;
}
