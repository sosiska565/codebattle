use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use sea_orm::DbErr;
use serde_json::json;

#[derive(thiserror::Error, Debug)]
pub enum AppError {
    #[error("resource not found")]
    NotFound,
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("database error")]
    Db(#[from] DbErr),
    #[error("password hashing error")]
    Hash(#[from] bcrypt::BcryptError),
    #[error("internal error")]
    Internal(#[from] anyhow::Error),
    #[error("validation error")]
    Validation(String),
    #[error("unauthorized error")]
    Unauthorized(String),
    #[error("jwt error")]
    Jwt(#[from] jsonwebtoken::errors::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        tracing::error!("{:?}", self);

        let (status, message) = match self {
            AppError::NotFound => (StatusCode::NOT_FOUND, self.to_string()),
            AppError::Conflict(_) => (StatusCode::CONFLICT, self.to_string()),
            AppError::Db(_) | AppError::Hash(_) | AppError::Internal(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal server error".to_string(),
            ),
            AppError::Validation(str) => (StatusCode::BAD_REQUEST, str),
            AppError::Unauthorized(str) => (StatusCode::UNAUTHORIZED, str),
            AppError::Jwt(_) => (
                StatusCode::UNAUTHORIZED,
                "invalid or expired token".to_string(),
            ),
        };

        (status, Json(json!({ "error": message }))).into_response()
    }
}
