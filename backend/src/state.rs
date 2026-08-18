use sqlx::SqlitePool;

#[derive(Clone)]
pub struct AppState {
    pub sqlite_pool: SqlitePool,
    pub redis_client: redis::Client,
    pub redis_con: redis::aio::MultiplexedConnection,
    pub oauth_client_id: String,
    pub oauth_client_secret: String,
    pub oauth_callback_url: String
}