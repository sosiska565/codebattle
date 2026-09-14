use std::sync::Arc;

use crate::models::user::{User, UserCreateDto};
use crate::service::user_service::UserService;
use axum::{
    Json, Router,
    extract::{Path, State},
    response::IntoResponse,
    routing::get,
};
use uuid::Uuid;

pub struct AppState {
    pub user_service: UserService,
}

pub fn create_route(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/user", get(get_all_users).post(create_user))
        .route("/user/{id}", get(get_user_by_id).delete(delete_user_by_id))
        .with_state(state)
}

async fn get_all_users(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    state.user_service.get_all().await
}

async fn create_user(
    State(state): State<Arc<AppState>>,
    Json(user_create_dto): Json<UserCreateDto>,
) -> impl IntoResponse {
    state.user_service.create(user_create_dto).await
}

async fn get_user_by_id(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    state.user_service.get_by_id(id).await
}

async fn delete_user_by_id(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    state.user_service.delete_by_id(id).await
}
