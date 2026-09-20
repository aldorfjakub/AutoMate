use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::get,
};
use redis::AsyncCommands;

use crate::extract::SessionUser;
use crate::state::AppState;
use crate::{
    error::{AppError, AppResult},
    models::dto::ValidationStatus,
};

async fn get_job_status(
    State(state): State<Arc<AppState>>,
    Path(job_id): Path<String>,
    _session: SessionUser,
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

pub fn job_routes() -> Router<Arc<AppState>> {
    Router::new().route("/{id}/status", get(get_job_status))
}
