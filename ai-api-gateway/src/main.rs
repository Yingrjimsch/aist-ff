mod config;

use axum::{
    Router,
    body::Bytes,
    extract::Path,
    http::{HeaderMap, Response, StatusCode},
    routing::{get, post},
};
use config::{ConfigLoader, TomlConfigLoader};
use reqwest::{Body, Method};

const BASE_URL: &str = "https://mgb-aifo-proxy-dev.service.migros.cloud";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // const BASE_URL: &str = "https://jsonplaceholder.typicode.com";
    let app = Router::new()
        .route("/", get(|| async { "I am sooo healthy!" }))
        .route("/health", get(|| async { "I am sooo healthy!" }))
        .route("/{*path}", get(forward_request_to_provider))
        .route("/{*path}", post(forward_request_to_provider));

    let loader: Box<dyn ConfigLoader> = Box::new(TomlConfigLoader::new("config.toml"));
    let app_config = loader.load()?;

    println!("Successfully loaded config!");
    println!("--- Loaded Providers ---");
    for provider in &app_config.providers {
        println!("Name: {}, URL: {}", provider.name, provider.url);
    }

    println!("\n--- Loaded Models ---");
    for model in &app_config.models {
        for id in &model.provider_ids {
            println!("Provider: {}", id)
        }
        println!("Model Name: {}", model.name);
    }

    let listener = match tokio::net::TcpListener::bind("0.0.0.0:3000").await {
        Ok(listener) => listener,
        Err(_) => tokio::net::TcpListener::bind("0.0.0.0:0").await.unwrap(),
    };

    let addr = listener.local_addr().unwrap();
    println!("Listening on http://{addr}");
    axum::serve(listener, app).await.unwrap();
    Ok(())
}

async fn forward_request_to_provider(
    method: Method,
    Path(path): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response<Body>, StatusCode> {
    let url: String = format!("{BASE_URL}/{path}");
    let mut headers: HeaderMap = headers;
    headers.remove("host");

    let client: reqwest::Client = reqwest::Client::new();

    let upstream_response: reqwest::Response = client
        .request(method, url)
        .body(body)
        .headers(headers)
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    let status = upstream_response.status();
    let body = upstream_response
        .bytes()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    Response::builder()
        .status(status)
        .body(Body::from(body))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
