mod data;
mod model_extractors;
mod requests;

#[cfg(feature = "db")]
use data::db_repo::DbRepository;
use data::toml_repo::TomlRepository;
use data::traits::DataRepository;

use std::{error::Error, sync::Arc};

use axum::{
    Router,
    body::{Body, Bytes},
    extract::{Path, State},
    http::{HeaderMap, Response, StatusCode},
    routing::{any, get},
};
use rand::prelude::IndexedRandom;
use reqwest::Method;

use crate::{
    data::models::Provider,
    model_extractors::{openai_json_extractor::OpenAiJsonExtractor, traits::ModelExtractor},
    requests::request_context::RequestContext,
};

#[derive(Clone)]
struct AppState {
    repo: Arc<dyn DataRepository>,
}

// const BASE_URL: &str = "https://jsonplaceholder.typicode.com";
const BASE_URL: &str = "https://mgb-aifo-proxy-dev.service.migros.cloud";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    dotenvy::dotenv().ok();

    // .route("/{*path}", post(forward_request_to_provider));

    let repo: Arc<dyn DataRepository> = build_repository().await?;
    let state: AppState = AppState { repo };
    print_system_settings(state.clone()).await?;
    // 2. Perform granular/lazy business queries seamlessly

    let app: Router = Router::new()
        .route("/", get(|| async { "I am sooo healthy!" }))
        .route("/health", get(|| async { "I am sooo healthy!" }))
        .route("/{*path}", any(forward_request_to_provider))
        .with_state(state);

    let listener: tokio::net::TcpListener =
        match tokio::net::TcpListener::bind("0.0.0.0:3000").await {
            Ok(listener) => listener,
            Err(_) => tokio::net::TcpListener::bind("0.0.0.0:0").await.unwrap(),
        };

    let addr: std::net::SocketAddr = listener.local_addr().unwrap();
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
    State(state): State<AppState>,
    method: Method,
    Path(path): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response<Body>, StatusCode> {
    println!("test");
    let ctx: RequestContext = RequestContext::new(method, path, headers, body);
    // TODO: filter model logic
    let extractor: OpenAiJsonExtractor = OpenAiJsonExtractor;
    let model: Option<String> = extractor.extract(&ctx).map_err(|err| {
        eprintln!("failed to extract model: {err}");
        StatusCode::BAD_REQUEST
    })?;
    println!("Extracted Model: {:?}", model);
    // TODO: select provider logic
    let providers: Vec<Provider> = match model {
        Some(model) => state
            .repo
            .get_providers_for_model(&model)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        None => Vec::new(),
    };
    println!("Extracted Providers: {:?}", providers);

    // TODO: switch api key if necessary
    // TODO: switch path accordingly
    let provider = providers
        .as_slice()
        .choose(&mut rand::rng())
        .ok_or(StatusCode::BAD_GATEWAY)?;
    println!("Random chosen provider: {:?}", provider);
    let url: String = format!(
        "{}/{}",
        provider.url.trim_end_matches('/'),
        ctx.path.trim_start_matches('/')
    );
    let mut headers: HeaderMap = ctx.headers;
    headers.remove("host");

    println!("Requesting URL: {}", url);
    let client: reqwest::Client = reqwest::Client::new();

    let upstream_response: reqwest::Response = client
        .request(ctx.method, url)
        .body(ctx.body)
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

// TODO: here make some nice print statements for the whole init
async fn print_system_settings(state: AppState) -> Result<(), Box<dyn Error + Send + Sync>> {
    println!("hello");
    if let Some(provider) = state.repo.get_provider("openai").await? {
        println!("Found Provider: {}", provider.name);

        // Lazily fetch the related models only when we need them!
        let models: Vec<data::models::Model> =
            state.repo.get_models_for_provider(&provider.id).await?;
        for model in models {
            println!("  -> Supports Model: {}", model.name);
        }
    }
    Ok(())
}
