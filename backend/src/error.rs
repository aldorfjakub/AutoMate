use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

#[derive(Debug)]
pub enum AppError {
    BadRequest(&'static str),
    Unauthorized,
    NotFound,
    Conflict(&'static str),
    Database(sqlx::Error),
    MissingEnv(&'static str),
    External(String),
    Internal(&'static str),
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

impl AppError {
    fn status(&self) -> StatusCode {
        match self {
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::Unauthorized => StatusCode::UNAUTHORIZED,
            AppError::NotFound => StatusCode::NOT_FOUND,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::Database(_) | AppError::MissingEnv(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::External(_) => StatusCode::BAD_GATEWAY,
            AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn to_internal(e: impl std::fmt::Display) -> Self {
        eprintln!("{e}");
        AppError::Internal("internal error")
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status();

        let message = match self {
            AppError::Database(e) => {
                eprintln!("database error: {e}");
                "internal server error".to_string()
            }
            AppError::External(e) => {
                eprintln!("upstream error: {e}");
                "upstream service failed".to_string()
            }
            AppError::MissingEnv(e) => {
                eprintln!("missing env: {e}");
                "server misconfiguration".to_string()
            }
            AppError::BadRequest(m) | AppError::Conflict(m) | AppError::Internal(m) => {
                m.to_string()
            }
            AppError::Unauthorized => "unauthorized".to_string(),
            AppError::NotFound => "not found".to_string(),
        };

        (status, Json(ErrorBody { error: message })).into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        AppError::Database(e)
    }
}

impl From<axum::http::StatusCode> for AppError {
    fn from(_s: axum::http::StatusCode) -> Self {
        AppError::Internal("http status propagated")
    }
}

pub type AppResult<T> = Result<T, AppError>;
