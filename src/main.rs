mod error;
mod models;
mod repository;
mod routes;
mod service;
use dotenvy::from_path;

use sqlx::PgPool;
use std::{path::PathBuf, sync::Arc};

use crate::service::battle_service::BattleService;
use crate::service::battle_ws_service::BattleWsService;
use crate::service::matchmaking_service::MatchmakingService;
use crate::service::piston_service::PistonService;
use crate::service::problem_service::ProblemService;
use crate::service::redis_service::RedisService;

#[tokio::main]
async fn main() {
    let mut env_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    env_path.push("codebattle-private");
    env_path.push(".env");

    tracing_subscriber::fmt::init();

    from_path(env_path).expect("Failed to load config/.env file");
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let jwt_secret = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");
    let jwt_ttl_seconds = std::env::var("JWT_TTL_SECONDS").expect("JWT_TTL_SECONDS must be set");

    let pool = PgPool::connect(&database_url)
        .await
        .expect("failed to connect to Postgres");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("failed to run migrations");

    let db = sea_orm::SqlxPostgresConnector::from_sqlx_postgres_pool(pool);
    let user_repo = Arc::new(repository::user_repository::PgUserRepository::new(
        db.clone(),
    ));
    let user_service = Arc::new(service::user_service::UserService::new(user_repo.clone()));
    let token_service = Arc::new(service::token_service::TokenService::new(
        &jwt_secret,
        jwt_ttl_seconds.parse().unwrap(),
    ));
    let auth_service = Arc::new(service::auth_service::AuthService::new(
        user_repo.clone(),
        token_service.clone(),
    ));
    let battle_repo = Arc::new(repository::battle_repository::PgBattleRepository::new(db));

    let piston_url =
        std::env::var("PISTON_URL").unwrap_or_else(|_| "http://127.0.0.1:2000".to_string());
    let redis_url = std::env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://:codebattle@127.0.0.1:6379/".to_string());

    let piston = Arc::new(PistonService::new(&piston_url));
    let redis = Arc::new(RedisService::new(&redis_url).expect("failed to connect to Redis"));
    let problem_service = Arc::new(ProblemService::new(piston.clone()));
    let battle_service = Arc::new(BattleService::new(
        problem_service.clone(),
        piston,
        redis,
        battle_repo,
        user_repo,
    ));
    let battle_ws_service = Arc::new(BattleWsService::new(battle_service.clone()));
    let matchmaking_service = Arc::new(MatchmakingService::new(battle_service.clone()));

    let app_state = routes::routes::AppState {
        user_service,
        auth_service,
        token_service,
        battle_ws_service,
        battle_service,
        matchmaking_service,
    };
    let routes = routes::routes::create_route(Arc::new(app_state));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();

    println!("listening on {}", listener.local_addr().unwrap());
    let _ = axum::serve(listener, routes).await;
}
