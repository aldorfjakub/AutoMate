use std::{str::FromStr, sync::Arc};

use axum::{Json, Router, extract::State, routing::get};
use axum_extra::extract::CookieJar;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::user::BasicUserInfo;
use crate::state::AppState;

async fn user_info(
    jar: CookieJar,
    State(state): State<Arc<AppState>>,
) -> AppResult<Json<BasicUserInfo>> {
    let session_cookie = jar
        .get("session_id")
        .ok_or(AppError::Unauthorized)?
        .value()
        .to_string();

    let session_id = Uuid::from_str(&session_cookie).map_err(|_| AppError::BadRequest("session_id is missing"))?.as_bytes().to_vec();
    let user = sqlx::query_as!(
        BasicUserInfo,
        r#"
                    SELECT 
                        u.display_name, 
                        u.avatar_url
                    FROM users u
                    JOIN sessions s ON u.id = s.user_id
                    WHERE s.id = ? 
                    AND s.expires_at > CURRENT_TIMESTAMP
                "#,
        session_id,
    )
    .fetch_optional(&state.sqlite_pool)
    .await?
    .ok_or(AppError::Unauthorized)?;

    Ok(Json(user))
}

pub fn user_routes() -> Router<Arc<AppState>> {
    Router::new().route("/me", get(user_info))
}
