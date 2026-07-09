mod data;
mod model_extractors;
mod opentelemetry;
mod requests;
mod secrets_loaders;

use ::opentelemetry::{KeyValue, global};
#[cfg(feature = "db")]
use data::db_repo::DbRepository;
use data::toml_repo::TomlRepository;
use data::traits::DataRepository;
use futures_util::stream;
use tracing::{debug, error, info};

use std::{error::Error, sync::Arc, time::Duration};

use axum::{
    Router,
    body::{Body, Bytes},
    extract::{Path, State},
    http::{HeaderMap, Response, StatusCode},
    response::IntoResponse,
    routing::{any, get},
};
use rand::{RngExt, prelude::IndexedRandom};
use reqwest::Method;

use crate::{
    data::models::Provider,
    model_extractors::{ModelExtractor, OpenAiJsonExtractor},
    opentelemetry::init_telemetry,
    requests::request_context::RequestContext,
    secrets_loaders::{
        apply::apply_auth, composite_secret_loader::CompositeSecretLoader,
        env_secret_loader::EnvSecretLoader, file_secret_loader::FileSecretLoader,
        traits::SecretLoader,
    },
};

#[derive(Clone)]
struct AppState {
    repo: Arc<dyn DataRepository>,
    secret_loader: Arc<dyn SecretLoader>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let (_logger_provider, _meter_provider) = init_telemetry();
    // TODO: logs and meter was implemented in general now i have to define the logs and metrics for the app
    // let meter = global::meter("ai_gateway_metrics");
    // let token_counter = meter
    //     .u64_counter("tokens_consumed_total")
    //     .with_description("Total number of tokens processed by the gateway")
    //     .build();
    // let attributes = [
    //     KeyValue::new("model", "hello"),
    //     KeyValue::new("status", "success"),
    // ];
    // token_counter.add(120, &attributes);
    // tracing::info!(
    //     target: "gateway::llm",
    //     "LLM generation complete"
    // );
    dotenvy::dotenv().ok();

    // .route("/{*path}", post(forward_request_to_provider));

    let repo: Arc<dyn DataRepository> = build_repository().await?;
    let secret_loader =
        CompositeSecretLoader::new(vec![Box::new(EnvSecretLoader), Box::new(FileSecretLoader)]);
    let state: AppState = AppState {
        repo,
        secret_loader: Arc::new(secret_loader),
    };
    print_system_settings(state.clone()).await?;
    // 2. Perform granular/lazy business queries seamlessly

    let app: Router = Router::new()
        .route("/", get(|| async { "I am sooo healthy!" }))
        .route("/favicon.ico", any(|| async { StatusCode::NO_CONTENT }))
        .route("/terminal", get(scramble_handler))
        .route("/health", get(|| async { "I am sooo healthy!" }))
        .route("/{*path}", any(forward_request_to_provider))
        .with_state(state);

    let listener: tokio::net::TcpListener =
        match tokio::net::TcpListener::bind("0.0.0.0:3000").await {
            Ok(listener) => listener,
            Err(_) => tokio::net::TcpListener::bind("0.0.0.0:0").await.unwrap(),
        };

    let addr: std::net::SocketAddr = listener.local_addr().unwrap();
    info!("Listening on http://{addr}");
    axum::serve(listener, app).await.unwrap();
    info!("Successfull startup");

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
) -> Result<Response<Body>, (StatusCode)> {
    let ctx: RequestContext = RequestContext::new(method, path, headers, body);
    // TODO: filter model logic
    let model = fun_name(&ctx)?;
    info!(model = model, "Model has been successfully extracted.");
    // TODO: select provider logic
    let providers: Vec<Provider> = match model {
        Some(model) => state
            .repo
            .get_providers_for_model(&model)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        None => Vec::new(),
    };
    info!("Extracted Providers: {:?}", providers);

    // TODO: switch path accordingly
    let provider = providers
        .as_slice()
        .choose(&mut rand::rng())
        .ok_or(StatusCode::BAD_GATEWAY)?;
    info!("Random chosen provider: {:?}", provider);
    info!("Provider auth method {:?}", provider.auth_method);
    let url: String = format!(
        "{}/{}",
        provider.url.trim_end_matches('/'),
        ctx.path.trim_start_matches('/')
    );

    // TODO: switch api key if necessary
    let mut headers: HeaderMap = ctx.headers;
    apply_auth(
        &mut headers,
        &provider.auth_method,
        state.secret_loader.as_ref(),
    )
    .map_err(|err| {
        error!("failed to apply auth: {err:?}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    info!("Headers: {:?}", headers);
    headers.remove("host");

    info!("Requesting URL: {}", url);
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

fn fun_name(ctx: &RequestContext) -> Result<Option<String>, StatusCode> {
    let extractor: OpenAiJsonExtractor = OpenAiJsonExtractor;
    let model: Option<String> = extractor.extract(ctx).map_err(|err| {
        //TODO: error message is not shown in Browser when i want to do curl
        error!("failed to extract model: {err}");
        StatusCode::BAD_REQUEST
    })?;
    Ok(model)
}

// TODO: here make some nice print statements for the whole init
async fn print_system_settings(state: AppState) -> Result<(), Box<dyn Error + Send + Sync>> {
    if let Some(provider) = state.repo.get_provider("openai").await? {
        info!("Found Provider: {}", provider.name);

        // Lazily fetch the related models only when we need them!
        let models: Vec<data::models::Model> =
            state.repo.get_models_for_provider(&provider.id).await?;
        for model in models {
            info!("  -> Supports Model: {}", model.name);
        }
    }
    Ok(())
}

async fn scramble_handler() -> impl IntoResponse {
    let target = "I am sooo\n healthy!";
    let chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*()_+-=";

    // Total animation frames
    let total_frames = 35;

    // Create a stream that ticks every 50 milliseconds
    let interval = tokio::time::interval(Duration::from_millis(50));
    let frame_stream = stream::unfold((interval, 0), move |(mut interval, frame)| async move {
        if frame > total_frames {
            return None; // End the stream
        }
        interval.tick().await;

        let mut rng = rand::rng();
        let mut display = String::new();

        // ANSI Escape Codes:
        // \x1B[2J clears the screen, \x1B[H moves cursor to the top-left corner
        display.push_str("\x1B[2J\x1B[H");

        // Build the current frame character by character
        for (i, target_char) in target.chars().enumerate() {
            if target_char == '\n' {
                display.push('\n');
                continue;
            }

            // Letters lock into place progressively from left to right
            let lock_threshold = (total_frames * i) / target.len();

            if frame >= total_frames || frame > lock_threshold && rng.random_bool(0.3) {
                display.push(target_char);
            } else {
                // Pick a random cipher character
                let idx = rng.random_range(0..chars.len());
                display.push(chars.chars().nth(idx).unwrap());
            }
        }

        // If it's the final frame, add a clean trailing newline
        if frame == total_frames {
            display.push_str("\n");
        }

        Some((Ok::<_, std::io::Error>(display), (interval, frame + 1)))
    });

    // Return the stream as an HTTP body chunk response
    Body::from_stream(frame_stream)
}
