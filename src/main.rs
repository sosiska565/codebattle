mod models;
mod repository;
mod routes;
mod service;
use dotenvy::from_path;

use std::{path::PathBuf, sync::Arc};

use axum::{
    Router,
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::get,
};
use sqlx::PgPool;

#[tokio::main]
async fn main() {
    let mut env_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    env_path.push("codebattle-private");
    env_path.push(".env");

    from_path(env_path).expect("Failed to load config/.env file");
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let app_state = routes::routes::AppState {
        user_service: service::user_service::UserService::new(
            repository::user_repository::UserRepository::new(
                PgPool::connect(&database_url).await.unwrap(),
            ),
        ),
    };
    let routes = routes::routes::create_route(Arc::new(app_state));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();

    println!("listening on {}", listener.local_addr().unwrap());
    let _ = axum::serve(listener, routes).await;
}
