use std::sync::Arc;

use axum::{Json, Router, extract::State, response::IntoResponse, routing::post};
use redis::AsyncCommands;
use reqwest::StatusCode;
use serde_json::json;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::{
    extract::SessionUser,
    models::dto::{BotSummary, Job, MatchRequest, MatchStatus},
};

pub async fn start_match(
    db_pool: SqlitePool,
    mut redis_conn: redis::aio::MultiplexedConnection,
    bot1: BotSummary,
    bot2: BotSummary,
    is_ranked: bool,
) -> AppResult<impl IntoResponse> {
    let match_uuid = Uuid::new_v4();
    let match_redis_id = format!("match_{}", match_uuid);
    // match_status will be pending, is_ranked will be false
    let white = if rand::random_bool(0.5) {
        bot1.id
    } else {
        bot2.id
    };
    let black = if white == bot1.id { bot2.id } else { bot1.id };

    let _ = sqlx::query!(
        "INSERT INTO matches (id, white_bot_id, black_bot_id, is_ranked) VALUES (?, ?, ?, ?)",
        match_uuid,
        &white,
        &black,
        is_ranked
    )
    .execute(&db_pool)
    .await?;
    let job: Job = Job::Match {
        match_id: match_uuid.to_string(),
        is_ranked,
        white_bot_id: white.unwrap_or_default().to_string(),
        black_bot_id: black.unwrap_or_default().to_string(),
    };

    let status = MatchStatus::Pending;

    let _: () = redis_conn
        .set_ex(
            &match_redis_id,
            serde_json::to_string(&status)
                .map_err(|_| AppError::Internal("Failed to serialize job_status to json"))?,
            600,
        )
        .await
        .map_err(|_| AppError::Internal("Redis failed"))?;
    let _: () = redis_conn
        .lpush(
            "job_queue",
            serde_json::to_string(&job)
                .map_err(|_| AppError::Internal("Failed to serialize job to json"))?,
        )
        .await
        .map_err(|_| AppError::Internal("Redis failed"))?;

    Ok((
        StatusCode::ACCEPTED,
        Json(json!({"match_id": &match_uuid.to_string()})),
    )
        .into_response())
}

async fn play_bot(
    State(state): State<Arc<AppState>>,
    session: SessionUser,
    Json(json): Json<MatchRequest>,
) -> AppResult<impl IntoResponse> {
    let user_bot = sqlx::query_as!(BotSummary, r#"SELECT id as "id: uuid::Uuid", user_id as "owner_id: uuid::Uuid", name, description, is_active, is_public, is_valid, rating, total_matches FROM bots WHERE id = ?"#, &json.player_bot_id).fetch_one(&state.sqlite_pool).await?;
    println!("User bot found");
    if user_bot.owner_id.unwrap_or_default() != session.user_id {
        return Err(AppError::Unauthorized);
    } else if !user_bot.is_valid {
        return Err(AppError::Conflict("Bot is not validated"));
    }

    let system_bot: BotSummary = sqlx::query_as!(BotSummary, r#"SELECT id as "id: uuid::Uuid", user_id as "owner_id: uuid::Uuid", name, description, is_active, is_public, is_valid, rating, total_matches FROM bots WHERE id = ? AND user_id is NULL"#, &json.opponent_bot_id).fetch_one(&state.sqlite_pool).await?;
    println!("System bot found");

    return start_match(
        state.sqlite_pool.clone(),
        state.redis_con.clone(),
        user_bot,
        system_bot,
        false,
    )
    .await;
}

pub fn play_routes() -> Router<Arc<AppState>> {
    Router::new().route("/", post(play_bot))
}
