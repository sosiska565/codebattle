use std::os::linux::raw::stat;

use crate::models::user::{self, User};
use crate::repository::user_repository;
use axum::Json;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde_json::json;
use uuid::Uuid;

pub struct UserService {
    repo: user_repository::UserRepository,
}

impl UserService {
    pub fn new(repo: user_repository::UserRepository) -> Self {
        Self { repo }
    }
    pub async fn create(&self, user: User) -> impl IntoResponse {
        let user = self
            .repo
            .create(user.username, user.elo, user.email, user.pass_hash)
            .await;

        match user {
            Ok(usr) => (StatusCode::CREATED, Json(usr).into_response()),
            Err(e) => {
                eprintln!("{}", e.to_string());
                (StatusCode::BAD_GATEWAY, Json(json!({})).into_response())
            }
        }
    }
    pub async fn get_by_id(&self, id: Uuid) -> impl IntoResponse {
        let user = self.repo.get_by_id(id).await;

        match user {
            Ok(Some(usr)) => (StatusCode::OK, Json(json!(usr))).into_response(),
            Ok(None) => (StatusCode::NOT_FOUND, Json(json!({}))).into_response(),
            Err(e) => {
                eprintln!("{}", e);
                (StatusCode::BAD_GATEWAY, Json(json!({}))).into_response()
            }
        }
    }
    pub async fn get_by_username(&self, username: String) -> impl IntoResponse {
        let user = self.repo.get_by_username(username).await;

        match user {
            Ok(Some(usr)) => (StatusCode::OK, Json(json!(usr))).into_response(),
            Ok(None) => (StatusCode::NOT_FOUND, Json(json!({}))).into_response(),
            Err(e) => {
                eprintln!("{}", e);
                (StatusCode::BAD_GATEWAY, Json(json!({}))).into_response()
            }
        }
    }
    pub async fn delete_by_id(&self, id: Uuid) -> impl IntoResponse {
        match self.repo.delete_by_id(id).await {
            Ok(()) => (StatusCode::OK, Json(json!({}))).into_response(),
            Err(e) => {
                eprintln!("{}", e);
                (StatusCode::BAD_GATEWAY, Json(json!({}))).into_response()
            }
        }
    }
    pub async fn update()
}
