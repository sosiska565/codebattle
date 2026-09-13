mod models;
mod repository;
mod routes;
mod service;

use axum::{
    Router,
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::get,
};

#[tokio::main]
async fn main() {
    let app = Router::new().route("/", get(handler));

    let app = app.fallback(handler_404);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();

    println!("listening on {}", listener.local_addr().unwrap());
    let _ = axum::serve(listener, app).await;
}

async fn handler() -> Html<&'static str> {
    Html("<h1>hello world</h1>")
}

async fn handler_404() -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        Html(
            "<img src=\"https://encrypted-tbn0.gstatic.com/images?q=tbn:ANd9GcQ36o0llmjMX_fETdo_9z27x20N85tVCFnd18Y3jl32iQ&s\">",
        ),
    )
}
