mod data;
mod model_extractors;
mod opentelemetry;
mod requests;
mod secrets_loaders;

use ::opentelemetry::metrics::{Counter, Histogram};
use ::opentelemetry::{KeyValue, global};
#[cfg(feature = "db")]
use data::db_repo::DbRepository;
use data::toml_repo::TomlRepository;
use data::traits::DataRepository;
use futures_util::stream;
use tracing::{error, info};

use std::{
    error::Error,
    sync::Arc,
    time::{Duration, Instant},
};

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
    metrics: GatewayMetrics,
}

#[derive(Clone)]
struct GatewayMetrics {
    requests_total: Counter<u64>,
    model_extracted_total: Counter<u64>,
    provider_selected_total: Counter<u64>,
    upstream_requests_total: Counter<u64>,
    upstream_response_total: Counter<u64>,
    upstream_bad_gateway_total: Counter<u64>,
    upstream_response_body_bytes_total: Counter<u64>,
    request_duration_ms: Histogram<f64>,
    upstream_duration_ms: Histogram<f64>,
}

impl GatewayMetrics {
    fn new() -> Self {
        let meter = global::meter("ai_api_gateway");

        Self {
            requests_total: meter
                .u64_counter("gateway_requests_total")
                .with_description("Total number of incoming gateway requests.")
                .build(),
            model_extracted_total: meter
                .u64_counter("gateway_model_extracted_total")
                .with_description("Total number of successfully extracted model names.")
                .build(),
            provider_selected_total: meter
                .u64_counter("gateway_provider_selected_total")
                .with_description("Total number of selected upstream providers.")
                .build(),
            upstream_requests_total: meter
                .u64_counter("gateway_upstream_requests_total")
                .with_description("Total number of upstream request attempts.")
                .build(),
            upstream_response_total: meter
                .u64_counter("gateway_upstream_response_total")
                .with_description("Total number of upstream responses by status.")
                .build(),
            upstream_bad_gateway_total: meter
                .u64_counter("gateway_upstream_bad_gateway_total")
                .with_description("Total number of upstream failures mapped to bad gateway.")
                .build(),
            upstream_response_body_bytes_total: meter
                .u64_counter("gateway_upstream_response_body_bytes_total")
                .with_description("Total number of upstream response body bytes read.")
                .build(),
            request_duration_ms: meter
                .f64_histogram("gateway_request_duration_ms")
                .with_description("Gateway request duration in milliseconds.")
                .with_unit("ms")
                .build(),
            upstream_duration_ms: meter
                .f64_histogram("gateway_upstream_duration_ms")
                .with_description(
                    "Upstream request and response body read duration in milliseconds.",
                )
                .with_unit("ms")
                .build(),
        }
    }
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
        metrics: GatewayMetrics::new(),
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
) -> Result<Response<Body>, StatusCode> {
    let start = Instant::now();
    let method_label = method.to_string();
    let path_label = path.clone();
    state.metrics.requests_total.add(
        1,
        &[
            KeyValue::new("method", method_label.clone()),
            KeyValue::new("path", path_label.clone()),
        ],
    );

    let result =
        forward_request_to_provider_inner(state.clone(), method, path, headers, body).await;
    let status = match &result {
        Ok(response) => response.status(),
        Err(status) => *status,
    };

    state.metrics.request_duration_ms.record(
        start.elapsed().as_secs_f64() * 1000.0,
        &[
            KeyValue::new("method", method_label),
            KeyValue::new("path", path_label),
            KeyValue::new("status_code", status.as_u16() as i64),
            KeyValue::new("status_class", status_class(status)),
        ],
    );

    result
}

async fn forward_request_to_provider_inner(
    state: AppState,
    method: Method,
    path: String,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response<Body>, StatusCode> {
    let method_label = method.to_string();
    let ctx: RequestContext = RequestContext::new(method, path, headers, body);
    // TODO: filter model logic
    let model = fun_name(&ctx)?;
    info!(model = model, "Model has been successfully extracted.");
    if let Some(model) = &model {
        state.metrics.model_extracted_total.add(
            1,
            &[
                KeyValue::new("method", method_label.clone()),
                KeyValue::new("model", model.clone()),
            ],
        );
    }
    // TODO: select provider logic
    let providers: Vec<Provider> = match model {
        Some(ref model) => state
            .repo
            .get_providers_for_model(model)
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
    let model_label = model.as_deref().unwrap_or("unknown");
    state.metrics.provider_selected_total.add(
        1,
        &[
            KeyValue::new("method", method_label.clone()),
            KeyValue::new("model", model_label.to_string()),
            KeyValue::new("provider_id", provider.id.clone()),
            KeyValue::new("provider_name", provider.name.clone()),
        ],
    );
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
    let upstream_start = Instant::now();

    let upstream_response: reqwest::Response = client
        .request(ctx.method, url)
        .body(ctx.body)
        .headers(headers)
        .send()
        .await
        .map_err(|_| {
            state.metrics.upstream_requests_total.add(
                1,
                &[
                    KeyValue::new("method", method_label.clone()),
                    KeyValue::new("model", model_label.to_string()),
                    KeyValue::new("provider_id", provider.id.clone()),
                    KeyValue::new("result", "network_error"),
                ],
            );
            state.metrics.upstream_bad_gateway_total.add(
                1,
                &[
                    KeyValue::new("method", method_label.clone()),
                    KeyValue::new("model", model_label.to_string()),
                    KeyValue::new("provider_id", provider.id.clone()),
                    KeyValue::new("phase", "send"),
                ],
            );
            state.metrics.upstream_duration_ms.record(
                upstream_start.elapsed().as_secs_f64() * 1000.0,
                &[
                    KeyValue::new("method", method_label.clone()),
                    KeyValue::new("model", model_label.to_string()),
                    KeyValue::new("provider_id", provider.id.clone()),
                    KeyValue::new("result", "network_error"),
                ],
            );
            StatusCode::BAD_GATEWAY
        })?;

    state.metrics.upstream_requests_total.add(
        1,
        &[
            KeyValue::new("method", method_label.clone()),
            KeyValue::new("model", model_label.to_string()),
            KeyValue::new("provider_id", provider.id.clone()),
            KeyValue::new("result", "success"),
        ],
    );

    let status = upstream_response.status();
    state.metrics.upstream_response_total.add(
        1,
        &[
            KeyValue::new("method", method_label.clone()),
            KeyValue::new("model", model_label.to_string()),
            KeyValue::new("provider_id", provider.id.clone()),
            KeyValue::new("status_code", status.as_u16() as i64),
            KeyValue::new("status_class", status_class(status)),
        ],
    );

    let body = upstream_response.bytes().await.map_err(|_| {
        state.metrics.upstream_bad_gateway_total.add(
            1,
            &[
                KeyValue::new("method", method_label.clone()),
                KeyValue::new("model", model_label.to_string()),
                KeyValue::new("provider_id", provider.id.clone()),
                KeyValue::new("phase", "read_body"),
            ],
        );
        state.metrics.upstream_duration_ms.record(
            upstream_start.elapsed().as_secs_f64() * 1000.0,
            &[
                KeyValue::new("method", method_label.clone()),
                KeyValue::new("model", model_label.to_string()),
                KeyValue::new("provider_id", provider.id.clone()),
                KeyValue::new("status_code", status.as_u16() as i64),
                KeyValue::new("status_class", status_class(status)),
                KeyValue::new("result", "read_body_error"),
            ],
        );
        StatusCode::BAD_GATEWAY
    })?;

    state.metrics.upstream_response_body_bytes_total.add(
        body.len() as u64,
        &[
            KeyValue::new("method", method_label.clone()),
            KeyValue::new("model", model_label.to_string()),
            KeyValue::new("provider_id", provider.id.clone()),
            KeyValue::new("status_code", status.as_u16() as i64),
            KeyValue::new("status_class", status_class(status)),
        ],
    );
    state.metrics.upstream_duration_ms.record(
        upstream_start.elapsed().as_secs_f64() * 1000.0,
        &[
            KeyValue::new("method", method_label),
            KeyValue::new("model", model_label.to_string()),
            KeyValue::new("provider_id", provider.id.clone()),
            KeyValue::new("status_code", status.as_u16() as i64),
            KeyValue::new("status_class", status_class(status)),
            KeyValue::new("result", "success"),
        ],
    );

    Response::builder()
        .status(status)
        .body(Body::from(body))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

fn status_class(status: StatusCode) -> &'static str {
    match status.as_u16() {
        100..=199 => "1xx",
        200..=299 => "2xx",
        300..=399 => "3xx",
        400..=499 => "4xx",
        500..=599 => "5xx",
        _ => "unknown",
    }
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
