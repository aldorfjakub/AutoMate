use std::env;

use dotenvy::dotenv;

use crate::routes::app_routes;

mod db;
mod error;
mod handlers;
mod models;
mod routes;
mod state;
#[tokio::main]
async fn main() {
    dotenv().ok();

    let db_pool = db::db::init_pool().await;
    let app_state = state::AppState {
        sqlite_pool: db_pool,
        oauth_client_id: env::var("OAUTH_CLIENT_ID").unwrap(),
        oauth_client_secret: env::var("OAUTH_CLIENT_SECRET").unwrap(),
        oauth_callback_url: env::var("OAUTH_CALLBACK").unwrap(),
    };

    let app = app_routes(app_state);
    // run it
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    println!("app running at {}", listener.local_addr().unwrap());
    let _ = axum::serve(listener, app).await;
}
