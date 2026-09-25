use std::sync::Arc;

use crate::error::AppError;
use crate::models::dto::auth_dto::TokenResponse;
use crate::models::dto::user_dto::{
    UserCreateRequest, UserLoginRequest, UserResponse, UserUpdateRequest,
};
use crate::models::users::User;
use crate::service::auth_service::AuthService;
use crate::service::battle_ws_service::BattleWsService;
use crate::service::token_service::TokenService;
use crate::service::user_service::UserService;
use axum::extract::WebSocketUpgrade;
use axum::http::StatusCode;
use axum::routing::{any, post};
use axum::{
    Json, Router,
    extract::{Path, State},
    response::IntoResponse,
    routing::get,
};
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid;
use validator::Validate;

pub struct AppState {
    pub user_service: Arc<UserService>,
    pub auth_service: Arc<AuthService>,
    pub token_service: Arc<TokenService>,
    pub battle_ws_service: Arc<BattleWsService>,
}

pub fn create_route(state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);
    Router::new()
        .route("/users", get(get_all_users).post(create_user))
        .route(
            "/users/{id}",
            get(get_user_by_id)
                .patch(update_user)
                .delete(delete_user_by_id),
        )
        .route("/auth/login", post(login))
        .route("/ws", any(ws_handler))
        .layer(cors)
        .with_state(state)
}

async fn login(
    State(state): State<Arc<AppState>>,
    Json(dto): Json<UserLoginRequest>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let token = state.auth_service.login(dto).await?;
    Ok(Json(token))
}

async fn get_all_users(State(state): State<Arc<AppState>>) -> Result<impl IntoResponse, AppError> {
    let users: Vec<User> = state.user_service.get_all().await?;
    Ok(Json(users))
}

async fn create_user(
    State(state): State<Arc<AppState>>,
    Json(dto): Json<UserCreateRequest>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let user = state.user_service.create(dto).await?;
    let access_token = state
        .token_service
        .generate_token(user.id, &user.username)?;
    let token = TokenResponse {
        access_token,
        token_type: "Bearer".to_string(),
        expires_in: state.token_service.ttl_seconds,
    };
    Ok((StatusCode::CREATED, Json(token)))
}

async fn get_user_by_id(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let user = state.user_service.get_by_id(id).await?;
    Ok(Json(UserResponse::from(user)))
}

async fn delete_user_by_id(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    state.user_service.delete_by_id(id).await?;
    Ok(StatusCode::OK)
}

async fn update_user(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(dto): Json<UserUpdateRequest>,
) -> Result<impl IntoResponse, AppError> {
    dto.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    let user = state.user_service.update(id, dto).await?;
    Ok(Json(UserResponse::from(user)))
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    Ok(ws.on_upgrade(move |socket| async move {
        state.battle_ws_service.echo(socket).await;
    }))
}
