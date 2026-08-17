use std::sync::Arc;

use axum::{Json, Router, extract::State, routing::get};

use crate::error::{AppError, AppResult};
use crate::extract::SessionUser;
use crate::models::user::BasicUserInfo;
use crate::state::AppState;

async fn user_info(
    State(state): State<Arc<AppState>>,
    session: SessionUser,
) -> AppResult<Json<BasicUserInfo>> {
    let user = sqlx::query_as!(
        BasicUserInfo,
        r#"
                    SELECT 
                        display_name, 
                        avatar_url
                    FROM users
                    WHERE id = ? 
                "#,
        session.user_id,
    )
    .fetch_optional(&state.sqlite_pool)
    .await?
    .ok_or(AppError::Unauthorized)?;

    Ok(Json(user))
}

pub fn user_routes() -> Router<Arc<AppState>> {
    Router::new().route("/me", get(user_info))
}
