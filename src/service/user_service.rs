use crate::models::user::{User, UserCreateDto};
use crate::repository::user_repository;
use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use bcrypt::{DEFAULT_COST, hash};
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
            Ok(usrs) => return (StatusCode::OK, Json(usrs)).into_response(),
            Err(e) => {
                eprintln!("{}", e);
                return (StatusCode::BAD_GATEWAY).into_response();
            }
        };
    }
    pub async fn create(&self, user_create_dto: UserCreateDto) -> Response {
        match self
            .repo
            .exists_by_username(&user_create_dto.username)
            .await
        {
            Ok(true) => return (StatusCode::CONFLICT).into_response(),
            Ok(false) => {}
            Err(e) => {
                eprintln!("{}", e);
                return (StatusCode::BAD_GATEWAY).into_response();
            }
        };

        match self.repo.exists_by_email(&user_create_dto.email).await {
            Ok(true) => return (StatusCode::CONFLICT).into_response(),
            Ok(false) => {}
            Err(e) => {
                eprintln!("{}", e);
                return (StatusCode::BAD_GATEWAY).into_response();
            }
        };

        let pass_hash =
            tokio::task::spawn_blocking(move || hash(user_create_dto.raw_password, DEFAULT_COST))
                .await
                .unwrap()
                .unwrap();

        let user = self
            .repo
            .create(
                &user_create_dto.username,
                &user_create_dto.email,
                &pass_hash,
                1000,
            )
            .await;

        match user {
            Ok(usr) => return (StatusCode::CREATED, Json(usr)).into_response(),
            Err(e) => {
                eprintln!("Create user service error: {}", e);
                return (StatusCode::BAD_GATEWAY).into_response();
            }
        };
    }
    pub async fn get_by_id(&self, id: Uuid) -> Response {
        let user = self.repo.get_by_id(&id).await;

        match user {
            Ok(Some(usr)) => return (StatusCode::OK, Json(json!(usr))).into_response(),
            Ok(None) => return (StatusCode::NOT_FOUND).into_response(),
            Err(e) => {
                eprintln!("{}", e);
                return (StatusCode::BAD_GATEWAY).into_response();
            }
        };
    }
    pub async fn get_by_username(&self, username: String) -> Response {
        let user = self.repo.get_by_username(&username).await;

        match user {
            Ok(Some(usr)) => return (StatusCode::OK, Json(json!(usr))).into_response(),
            Ok(None) => return (StatusCode::NOT_FOUND).into_response(),
            Err(e) => {
                eprintln!("{}", e);
                return (StatusCode::BAD_GATEWAY).into_response();
            }
        };
    }
    pub async fn delete_by_id(&self, id: Uuid) -> Response {
        match self.repo.delete_by_id(&id).await {
            Ok(()) => return (StatusCode::OK).into_response(),
            Err(e) => {
                eprintln!("{}", e);
                return (StatusCode::BAD_GATEWAY).into_response();
            }
        };
    }
    pub async fn update(&self, user: User) -> Response {
        match self.repo.update(&user).await {
            Ok(usr) => return (StatusCode::OK, Json(json!(usr))).into_response(),
            Err(e) => {
                eprintln!("{}", e);
                return (StatusCode::BAD_GATEWAY).into_response();
            }
        };
    }
}
