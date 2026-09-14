use std::os::linux::raw::stat;

use crate::models::user::{self, User};
use crate::repository::user_repository;
use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;
use uuid::Uuid;

pub struct UserService {
    repo: user_repository::UserRepository,
}

impl UserService {
    pub fn new(repo: user_repository::UserRepository) -> Self {
        Self { repo }
    }
    pub async fn get_all(&self) -> Response {
        match self.repo.get_all().await {
            Ok(usrs) => (StatusCode::OK, Json(usrs)).into_response(),
            Err(e) => {
                eprintln!("{}", e.to_string());
                (StatusCode::BAD_GATEWAY, Json(json!({}))).into_response()
            }
        }
    }
    pub async fn create(&self, user: User) -> Response {
        let user = self
            .repo
            .create(user.username, user.email, user.pass_hash)
            .await;

        match user {
            Ok(usr) => (StatusCode::CREATED, Json(usr)).into_response(),
            Err(e) => {
                eprintln!("{}", e.to_string());
                (StatusCode::BAD_GATEWAY, Json(json!({}))).into_response()
            }
        }
    }
    pub async fn get_by_id(&self, id: Uuid) -> Response {
        let user = self.repo.get_by_id(id).await;

        match user {
            Ok(Some(usr)) => (StatusCode::OK, Json(json!(usr))).into_response(),
            Ok(None) => (StatusCode::NOT_FOUND, Json(json!({}))).into_response(),
            Err(e) => {
                eprintln!("{}", e.to_string());
                (StatusCode::BAD_GATEWAY, Json(json!({}))).into_response()
            }
        }
    }
    pub async fn get_by_username(&self, username: String) -> Response {
        let user = self.repo.get_by_username(username).await;

        match user {
            Ok(Some(usr)) => (StatusCode::OK, Json(json!(usr))).into_response(),
            Ok(None) => (StatusCode::NOT_FOUND, Json(json!({}))).into_response(),
            Err(e) => {
                eprintln!("{}", e.to_string());
                (StatusCode::BAD_GATEWAY, Json(json!({}))).into_response()
            }
        }
    }
    pub async fn delete_by_id(&self, id: Uuid) -> Response {
        match self.repo.delete_by_id(id).await {
            Ok(()) => (StatusCode::OK, Json(json!({}))).into_response(),
            Err(e) => {
                eprintln!("{}", e.to_string());
                (StatusCode::BAD_GATEWAY, Json(json!({}))).into_response()
            }
        }
    }
    pub async fn update(&self, user: User) -> Response {
        match self.repo.update(user).await {
            Ok(usr) => (StatusCode::OK, Json(json!(usr))).into_response(),
            Err(e) => {
                eprintln!("{}", e.to_string());
                (StatusCode::BAD_GATEWAY, Json(json!({}))).into_response()
            }
        }
    }
}
