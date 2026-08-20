use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use redis::AsyncCommands;
use reqwest::StatusCode;
use serde_json::json;
use uuid::Uuid;

use crate::extract::SessionUser;
use crate::models::dto::{BotInfo, BotSummary, NewBotRequest};
use crate::state::AppState;
use crate::{
    error::{AppError, AppResult},
    models::dto::{Job, ValidationStatus},
};

const MIN_NAME_LEN: usize = 5;
const MAX_NAME_LEN: usize = 32;
const MAX_DESCRIPTION_LEN: usize = 810;
const MAX_SOURCE_BYTES: usize = 131072;
const MIN_SOURCE_BYTES: usize = 20;

fn validate_payload(payload: &NewBotRequest) -> AppResult<()> {
    if payload.name.len() < MIN_NAME_LEN {
        return Err(AppError::BadRequest(
            "Bot name has to be at least 5 characters.",
        ));
    }
    if payload.name.len() > MAX_NAME_LEN {
        return Err(AppError::BadRequest(
            "Bot name has to be less than 32 characters.",
        ));
    }
    if let Some(desc) = payload.description.as_ref() {
        if desc.len() > MAX_DESCRIPTION_LEN {
            return Err(AppError::BadRequest(
                "Bot description has to be less than 810 characters.",
            ));
        }
    }
    if payload.source_code.bytes().len() > MAX_SOURCE_BYTES {
        return Err(AppError::BadRequest("Source code has to be under 128 KiB"));
    }
    if payload.source_code.bytes().len() < MIN_SOURCE_BYTES {
        return Err(AppError::BadRequest("Source code can't be empty."));
    }
    Ok(())
}

// Update bot
async fn update_bot(
    State(state): State<Arc<AppState>>,
    Path(bot_id): Path<Uuid>,
    session: SessionUser,
    Json(payload): Json<NewBotRequest>,
) -> AppResult<()> {
    validate_payload(&payload)?;

    // If the code differs set is_valid to false for obvious reasons
    let original_bot = sqlx::query!(
        "SELECT source_code, is_valid FROM bots WHERE id = ? AND user_id = ?",
        &bot_id,
        &session.user_id
    )
    .fetch_one(&state.sqlite_pool)
    .await?;
    let is_same_code = original_bot.source_code == payload.source_code;
    let res = sqlx::query!(
        "UPDATE bots SET name = ?, description = ?, source_code = ?, is_active = ?, is_public = ?, is_valid = ? WHERE id = ? AND user_id = ?",
        &payload.name, &payload.description,
        &payload.source_code,
        payload.is_active,
        payload.is_public,
        (original_bot.is_valid && is_same_code),
        &bot_id,
        &session.user_id)
    .execute(&state.sqlite_pool).await?;

    match res.rows_affected() {
        0 => Err(AppError::NotFound),
        1 => Ok(()),
        _ => Err(AppError::Internal("Unexpected number of rows affected.")),
    }
}

//Get user's bots (send all but without source code, )
async fn get_user_bots(
    State(state): State<Arc<AppState>>,
    session: SessionUser,
) -> AppResult<Json<Vec<BotSummary>>> {
    let bots = sqlx::query_as!(
        BotSummary,
        r#"SELECT id as "id: uuid::Uuid", name, description, is_active, is_public, is_valid FROM bots WHERE user_id = ?"#,
        &session.user_id
    )
    .fetch_all(&state.sqlite_pool)
    .await?;

    Ok(Json(bots))
}

async fn delete_bot(
    State(state): State<Arc<AppState>>,
    Path(bot_id): Path<Uuid>,
    session: SessionUser,
) -> AppResult<()> {
    let res = sqlx::query!(
        "DELETE FROM bots WHERE id = ? AND user_id = ?",
        &bot_id,
        &session.user_id
    )
    .execute(&state.sqlite_pool)
    .await?;
    match res.rows_affected() {
        0 => Err(AppError::NotFound),
        1 => Ok(()),
        _ => Err(AppError::Internal("Unexpected number of rows affected.")),
    }
}

async fn get_bot_details(
    State(state): State<Arc<AppState>>,
    Path(bot_id): Path<Uuid>,
    session: SessionUser,
) -> AppResult<Json<BotInfo>> {
    let bot = sqlx::query_as!(BotInfo,
            r#"SELECT id as "id: uuid::Uuid", name, description, is_active, is_public, source_code, is_valid FROM bots WHERE id = ? AND user_id = ?"#, &bot_id, &session.user_id)
        .fetch_optional(&state.sqlite_pool)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(Json(bot))
}

async fn register_bot(
    State(state): State<Arc<AppState>>,
    session: SessionUser,
    Json(payload): Json<NewBotRequest>,
) -> AppResult<()> {
    validate_payload(&payload)?;

    let bot_id = Uuid::now_v7();
    let _ = sqlx::query!(
        "INSERT INTO bots (id, user_id, name, description, source_code, is_public) VALUES (?, ?, ?, ?, ?, ?)",
        &bot_id,
        &session.user_id,
        &payload.name,
        &payload.description,
        &payload.source_code,
        payload.is_public
    )
    .execute(&state.sqlite_pool)
    .await?;
    Ok(())
}

async fn validate_bot(
    State(state): State<Arc<AppState>>,
    Path(bot_id): Path<Uuid>,
    session: SessionUser,
) -> AppResult<Response> {
    let bot = sqlx::query_as!(BotInfo,r#"SELECT id as "id: uuid::Uuid", name, description, is_active, is_public, source_code, is_valid FROM bots WHERE id = ? AND user_id = ?"#, &bot_id, &session.user_id).fetch_optional(&state.sqlite_pool).await?.ok_or(AppError::NotFound)?;
    if bot.is_valid {
        return Err(AppError::BadRequest("Bot is already validated."));
    }

    let job_id = format!("job_{}", Uuid::now_v7().to_string());

    let job = Job::Validate {
        bot_id: bot_id.to_string(),
        job_id: job_id.clone(),
    };
    let mut redis_conn: redis::aio::MultiplexedConnection = state.redis_con.clone();
    let status = ValidationStatus::Pending;
    let _: () = redis_conn
        .set_ex(
            &job_id,
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

    Ok((StatusCode::ACCEPTED, Json(json!({"job_id": job_id}))).into_response())
}

async fn get_job_status(
    State(state): State<Arc<AppState>>,
    Path(job_id): Path<String>,
    session: SessionUser,
) -> AppResult<Json<ValidationStatus>> {
    let mut redis_conn: redis::aio::MultiplexedConnection = state.redis_con.clone();

    let res: String = redis_conn
        .get(job_id)
        .await
        .map_err(|_| AppError::NotFound)?;
    let status: ValidationStatus = serde_json::from_str(&res)
        .map_err(|_| AppError::Internal("Failed to parse validation status"))?;

    Ok(Json(status))
}

pub fn bots_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(get_user_bots).post(register_bot))
        .route(
            "/{id}",
            get(get_bot_details).put(update_bot).delete(delete_bot),
        )
        .route("/{id}/validate", post(validate_bot))
        .route("/{id}/status", get(get_job_status))
}
