use axum::{Router, extract::Path, response::IntoResponse, http::StatusCode};

const S3_BUCKET: &str = "kml-laurent";
const S3_REGION: &str = "eu-west-3";

async fn proxy_s3(Path(path): Path<String>) -> impl IntoResponse {
    let url = format!(
        "https://{}.s3.{}.amazonaws.com/{}",
        S3_BUCKET, S3_REGION, path
    );
    match reqwest::get(&url).await {
        Ok(resp) if resp.status().is_success() => {
            let body = resp.bytes().await.unwrap_or_default();
            (StatusCode::OK, [("content-type", "application/vnd.google-earth.kml+xml")], body).into_response()
        }
        Ok(resp) => (StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::NOT_FOUND), "Not found").into_response(),
        Err(e) => (StatusCode::BAD_GATEWAY, format!("S3 fetch error: {}", e)).into_response(),
    }
}

#[tokio::main]
async fn main() {
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());

    let app = Router::new().route("/{*path}", axum::routing::get(proxy_s3));

    let addr = format!("0.0.0.0:{}", port);
    println!("Proxying S3 bucket {} on {}", S3_BUCKET, addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
