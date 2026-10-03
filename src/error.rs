use axum::{
    Json,
    extract::multipart::MultipartError,
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
    #[error("file error")]
    File(#[from] MultipartError),
    #[error("bad request")]
    BadRequest(String),
    #[error("parse json")]
    ParseJson(#[from] serde_json::Error),
    #[error("reqwest")]
    Reqwest(#[from] reqwest::Error),
    #[error("redis")]
    Redis(#[from] redis::RedisError),
    #[error("redis pool")]
    Pool(#[from] r2d2::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        tracing::error!("{:?}", self);

        let (status, message) = match self {
            AppError::NotFound => (StatusCode::NOT_FOUND, self.to_string()),
            AppError::Conflict(_) => (StatusCode::CONFLICT, self.to_string()),
            AppError::Db(_)
            | AppError::Hash(_)
            | AppError::File(_)
            | AppError::Internal(_)
            | AppError::ParseJson(_)
            | AppError::Reqwest(_)
            | AppError::Redis(_)
            | AppError::Pool(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal server error".to_string(),
            ),
            AppError::Validation(str) => (StatusCode::BAD_REQUEST, str),
            AppError::Unauthorized(str) => (StatusCode::UNAUTHORIZED, str),
            AppError::Jwt(_) => (
                StatusCode::UNAUTHORIZED,
                "invalid or expired token".to_string(),
            ),
            AppError::BadRequest(str) => (StatusCode::BAD_REQUEST, str),
        };

        (status, Json(json!({ "error": message }))).into_response()
    }
}
