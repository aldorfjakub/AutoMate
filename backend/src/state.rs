use sqlx::SqlitePool;

#[derive(Clone)]
pub struct AppState {
    pub sqlite_pool: SqlitePool,
    pub oauth_client_id: String,
    pub oauth_client_secret: String,
    pub oauth_callback_url: String
}