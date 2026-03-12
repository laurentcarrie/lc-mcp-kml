use axum::Router;
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() {
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let static_dir = std::env::var("STATIC_DIR").unwrap_or_else(|_| "static".to_string());

    let app = Router::new().nest_service("/", ServeDir::new(&static_dir));

    let addr = format!("0.0.0.0:{}", port);
    println!("Serving {} on {}", static_dir, addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
