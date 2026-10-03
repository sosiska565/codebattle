use std::sync::Arc;

use crate::error::AppError;
use crate::models::dto::auth_dto::TokenResponse;
use crate::models::dto::user_dto::{
    UserCreateRequest, UserLoginRequest, UserResponse, UserUpdateRequest,
};
use crate::models::users::User;
use crate::service::auth_service::AuthService;
use crate::service::battle_service::BattleService;
use crate::service::battle_ws_service::BattleWsService;
use crate::service::matchmaking_service::MatchmakingService;
use crate::service::token_service::TokenService;
use crate::service::user_service::UserService;
use axum::extract::{Query, WebSocketUpgrade};
use axum::http::StatusCode;
use axum::routing::post;
use axum::{
    Json, Router,
    extract::{Path, State},
    response::IntoResponse,
    routing::get,
};
use serde::Deserialize;
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid;
use validator::Validate;

pub struct AppState {
    pub user_service: Arc<UserService>,
    pub auth_service: Arc<AuthService>,
    pub token_service: Arc<TokenService>,
    pub battle_ws_service: Arc<BattleWsService>,
    pub battle_service: Arc<BattleService>,
    pub matchmaking_service: Arc<MatchmakingService>,
}

#[derive(Deserialize)]
struct WsAuthQuery {
    token: String,
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
        .route("/users/{id}/battles", get(get_user_battles))
        .route("/auth/login", post(login))
        .route("/ws/matchmaking", get(matchmaking_ws))
        .route("/ws/battle/{battle_id}", get(battle_ws))
        // .route("/battle", any(ws_handler).post(upload_file))
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

async fn get_user_battles(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let history = state.battle_service.history(id).await?;
    Ok(Json(history))
}

async fn authenticate_ws(state: &AppState, token: &str) -> Result<User, AppError> {
    let claims = state.token_service.verify_token(token)?;
    let id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Unauthorized("invalid token subject".to_string()))?;
    state.user_service.get_by_id(id).await
}

async fn matchmaking_ws(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Query(q): Query<WsAuthQuery>,
) -> Result<impl IntoResponse, AppError> {
    let user = authenticate_ws(&state, &q.token).await?;
    Ok(ws.on_upgrade(move |socket| async move {
        state.matchmaking_service.handle(socket, user).await;
    }))
}

async fn battle_ws(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Path(battle_id): Path<Uuid>,
    Query(q): Query<WsAuthQuery>,
) -> Result<impl IntoResponse, AppError> {
    let user = authenticate_ws(&state, &q.token).await?;
    Ok(ws.on_upgrade(move |socket| async move {
        state
            .battle_ws_service
            .handle_battle(socket, battle_id, user.id)
            .await;
    }))
}

// async fn upload_file(
//     State(state): State<Arc<AppState>>,
//     mut multipart: Multipart,
// ) -> Result<impl IntoResponse, AppError> {
//     Ok(state.battle_service.upload_file(&mut multipart).await)
// }
