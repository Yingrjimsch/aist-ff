mod data;

#[cfg(feature = "db")]
use data::db_repo::DbRepository;
use data::toml_repo::TomlRepository;
use data::traits::DataRepository;

use std::{env, error::Error, sync::Arc};

use axum::{
    Router,
    body::{Body, Bytes},
    extract::Path,
    http::{HeaderMap, Response, StatusCode},
    routing::{get, post},
};
use reqwest::Method;

const BASE_URL: &str = "https://mgb-aifo-proxy-dev.service.migros.cloud";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    dotenvy::dotenv().ok();

    match env::var("DATABASE_NAME") {
        Ok(name) => println!("Database Name: {}", name),
        Err(_) => println!("DATABASE_NAME not set"),
    }
    // const BASE_URL: &str = "https://jsonplaceholder.typicode.com";
    let app = Router::new()
        .route("/", get(|| async { "I am sooo healthy!" }))
        .route("/health", get(|| async { "I am sooo healthy!" }))
        .route("/{*path}", get(forward_request_to_provider))
        .route("/{*path}", post(forward_request_to_provider));

    let repo: Arc<dyn DataRepository> = build_repository().await?;

    // 2. Perform granular/lazy business queries seamlessly
    if let Some(provider) = repo.get_provider("openai").await? {
        println!("Found Provider: {}", provider.name);

        // Lazily fetch the related models only when we need them!
        let models = repo.get_models_for_provider(&provider.id).await?;
        for model in models {
            println!("  -> Supports Model: {}", model.name);
        }
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

#[cfg(feature = "db")]
async fn build_repository() -> Result<Arc<dyn DataRepository>, Box<dyn Error + Send + Sync>> {
    if let Ok(database_url) = std::env::var("DATABASE_URL") {
        let pool = sqlx::PgPool::connect(&database_url).await?;
        return Ok(Arc::new(DbRepository::new(pool)));
    }

    Ok(Arc::new(TomlRepository::new("config.toml")?))
}

#[cfg(not(feature = "db"))]
async fn build_repository() -> Result<Arc<dyn DataRepository>, Box<dyn Error + Send + Sync>> {
    Ok(Arc::new(TomlRepository::new("config.toml")?))
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
