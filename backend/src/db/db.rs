use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use std::env;


pub async fn init_pool() -> SqlitePool {
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");


    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .expect("Failed to create pool");

    // Run migrations automatically
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    pool
}

