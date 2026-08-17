use std::{str::FromStr, sync::Arc};

use axum::extract::FromRequestParts;
use axum_extra::extract::CookieJar;
use uuid::Uuid;

use crate::{error::AppError, state::AppState};
pub struct SessionUser {
    pub user_id: Uuid,
}

impl FromRequestParts<Arc<AppState>> for SessionUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_request_parts(parts, state)
            .await
            .expect("CookieJar extraction is infallible");
        let session_id = jar.get("session_id").ok_or(AppError::Unauthorized)?.value();
        let session_id = Uuid::from_str(session_id).map_err(|_| AppError::Unauthorized)?;
        let session_bytes = session_id.as_bytes().to_vec();
        let row = sqlx::query!(
            "SELECT user_id FROM sessions WHERE id = ? AND expires_at > CURRENT_TIMESTAMP",
            session_bytes
        )
        .fetch_optional(&state.sqlite_pool)
        .await?
        .ok_or(AppError::Unauthorized)?;

        Ok(SessionUser {
            user_id: Uuid::from_slice(&row.user_id).map_err(|_| AppError::Unauthorized)?,
        })
    }
}
