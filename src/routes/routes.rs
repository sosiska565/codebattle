use std::sync::Arc;

use crate::error::AppError;
use crate::models::dto::user_dto::{UserCreateRequest, UserResponse, UserUpdateRequest};
use crate::models::users::User;
use crate::repository::user_repository::UserRepository;
use crate::service::user_service::UserService;
use axum::http::StatusCode;
use axum::{
    Json, Router,
    extract::{Path, State},
    response::IntoResponse,
    routing::get,
};
use uuid::Uuid;
use validator::Validate;

pub struct AppState<R: UserRepository> {
    pub user_service: UserService<R>,
}

pub fn create_route<R: UserRepository + 'static>(state: Arc<AppState<R>>) -> Router {
    Router::new()
        .route("/users", get(get_all_users).post(create_user))
        .route(
            "/users/{id}",
            get(get_user_by_id)
                .patch(update_user)
                .delete(delete_user_by_id),
        )
        .with_state(state)
}

async fn get_all_users<R: UserRepository>(
    State(state): State<Arc<AppState<R>>>,
) -> Result<impl IntoResponse, AppError> {
    let users: Vec<User> = state.user_service.get_all().await?;
    Ok(Json(users))
}

async fn create_user<R: UserRepository + 'static>(
    State(state): State<Arc<AppState<R>>>,
    Json(dto): Json<UserCreateRequest>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let user = state.user_service.create(dto).await?;
    Ok((StatusCode::CREATED, Json(UserResponse::from(user))))
}

async fn get_user_by_id<R: UserRepository + 'static>(
    State(state): State<Arc<AppState<R>>>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let user = state.user_service.get_by_id(id).await?;
    Ok(Json(UserResponse::from(user)))
}

async fn delete_user_by_id<R: UserRepository + 'static>(
    State(state): State<Arc<AppState<R>>>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    state.user_service.delete_by_id(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn update_user<R: UserRepository + 'static>(
    State(state): State<Arc<AppState<R>>>,
    Path(id): Path<Uuid>,
    Json(dto): Json<UserUpdateRequest>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let user = state.user_service.update(id, dto).await?;
    Ok(Json(UserResponse::from(user)))
}
