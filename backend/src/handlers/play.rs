use std::sync::Arc;

use axum::{
    Json, Router, extract::{Path, State}, response::IntoResponse, routing::{get, post},
};
use redis::AsyncCommands;
use reqwest::StatusCode;
use serde_json::json;
use uuid::Uuid;

use crate::{extract::SessionUser, models::dto::{BotSummary, Job, MatchRequest, MatchStatus}};
use crate::state::AppState;
use crate::{
    error::{AppError, AppResult},
    models::dto::ValidationStatus,
};

async fn play_bot(
    State(state): State<Arc<AppState>>,
    session: SessionUser,
    Json(json): Json<MatchRequest>,
) -> AppResult<impl IntoResponse> {
    let user_bot = sqlx::query_as!(BotSummary, r#"SELECT id as "id: uuid::Uuid", user_id as "owner_id: uuid::Uuid", name, description, is_active, is_public, is_valid FROM bots WHERE id = ?"#, &json.player_bot_id).fetch_one(&state.sqlite_pool).await?;
    println!("User bot found");
    if user_bot.owner_id.unwrap_or_default() != session.user_id{
        return Err(AppError::Unauthorized)
    }
    else if !user_bot.is_valid {
        return Err(AppError::Conflict("Bot is not validated"));
    }

    let system_bot = sqlx::query_as!(BotSummary, r#"SELECT id as "id: uuid::Uuid", user_id as "owner_id: uuid::Uuid", name, description, is_active, is_public, is_valid FROM bots WHERE id = ? AND user_id is NULL"#, &json.opponent_bot_id).fetch_one(&state.sqlite_pool).await?;
    println!("System bot found");

    let match_uuid = Uuid::new_v4();
    let match_redis_id = format!("match_{}", match_uuid);
    // match_status will be pending, is_ranked will be false
    let white = if rand::random_bool(0.5) {
        json.player_bot_id
    } else {
        json.opponent_bot_id
    };
    let black = if white == json.player_bot_id {
        json.opponent_bot_id
    } else {
        json.player_bot_id
    };

    let _ = sqlx::query!(
        "INSERT INTO matches (id, white_bot_id, black_bot_id) VALUES (?, ?, ?)",
        match_uuid,
        &white,
        &black
    )
    .execute(&state.sqlite_pool)
    .await?;
    let job: Job = Job::Match {
        match_id: match_uuid.to_string(),
        white_bot_id: white.to_string(),
        black_bot_id: black.to_string(),
    };

    let status = MatchStatus::Pending;

    let mut redis_conn: redis::aio::MultiplexedConnection = state.redis_con.clone();
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

pub fn play_routes() -> Router<Arc<AppState>> {
    Router::new().route("/", post(play_bot))
}
