mod models;
mod repository;
mod routes;
mod service;
mod error;
use dotenvy::from_path;

use sqlx::PgPool;
use std::{path::PathBuf, sync::Arc};

#[tokio::main]
async fn main() {
    let mut env_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    env_path.push("codebattle-private");
    env_path.push(".env");

    from_path(env_path).expect("Failed to load config/.env file");
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = PgPool::connect(&database_url)
        .await
        .expect("failed to connect to Postgres");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("failed to run migrations");

    let repo = Arc::new(repository::user_repository::PgUserRepository::new(pool));
    let user_service = service::user_service::UserService::new(repo);

    let app_state = routes::routes::AppState { user_service };
    let routes = routes::routes::create_route(Arc::new(app_state));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();

    println!("listening on {}", listener.local_addr().unwrap());
    let _ = axum::serve(listener, routes).await;
}
