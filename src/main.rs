use std::{collections::HashMap, env, net::SocketAddr, sync::Arc, time::SystemTime};

use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, HeaderMap, HeaderValue, Method, StatusCode, Uri},
    response::{Html, IntoResponse, Response},
    routing::{any, get, post},
    Json, Router,
};
use dotenvy::dotenv;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::RwLock;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{error, info};
use uuid::Uuid;

const READ_PAGE: &str = include_str!("../static/read.html");

#[derive(Clone)]
struct AppState {
    client: Client,
    config: Config,
    library: Arc<RwLock<HashMap<String, LibraryBook>>>,
    jobs: Arc<RwLock<HashMap<Uuid, DownloadJob>>>,
}

#[derive(Clone, Debug)]
struct Config {
    bind: String,
    upstream_base_url: Option<String>,
    request_timeout_secs: u64,
    fanqie_cookie: Option<String>,
}

impl Config {
    fn from_env() -> Self {
        Self {
            bind: env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".into()),
            upstream_base_url: env::var("WORKER_UPSTREAM_URL")
                .ok()
                .filter(|v| !v.trim().is_empty())
                .map(|v| v.trim_end_matches('/').to_string()),
            request_timeout_secs: env::var("REQUEST_TIMEOUT_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(45),
            fanqie_cookie: env::var("FANQIE_COOKIE").ok().filter(|v| !v.is_empty()),
        }
    }
}

#[derive(Debug, Deserialize)]
struct ApiQuery {
    #[serde(flatten)]
    values: HashMap<String, String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct LibraryBook {
    book_id: String,
    #[serde(default)]
    book_name: String,
    #[serde(default)]
    author: String,
    #[serde(default)]
    cover: String,
}

#[derive(Debug, Deserialize)]
struct CreateJob {
    book_id: String,
    #[serde(default = "default_format")]
    format: String,
}

fn default_format() -> String { "txt".to_string() }

#[derive(Debug, Serialize, Clone)]
struct DownloadJob {
    id: Uuid,
    book_id: String,
    format: String,
    status: String,
    created_at: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(env::var("RUST_LOG").unwrap_or_else(|_| "info,tower_http=info".into()))
        .init();

    let config = Config::from_env();
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(config.request_timeout_secs))
        .user_agent("NovelDownloader-Rust/0.1")
        .brotli(true)
        .gzip(true)
        .build()?;

    let state = AppState {
        client,
        config: config.clone(),
        library: Arc::new(RwLock::new(HashMap::new())),
        jobs: Arc::new(RwLock::new(HashMap::new())),
    };

    let app = Router::new()
        .route("/", get(index))
        .route("/health", get(health))
        .route("/read", get(read_page))
        .route("/read/", get(read_page))
        .route("/api/search", get(proxy_get))
        .route("/api/detail", get(proxy_get))
        .route("/api/directory", get(proxy_get))
        .route("/api/content", get(proxy_get))
        .route("/api/download", get(proxy_get))
        .route("/api/export", get(proxy_get))
        .route("/api/export/zip", get(proxy_get))
        .route("/api/update", get(proxy_get))
        .route("/api/config", any(proxy_any))
        .route("/api/proxy/pool", any(proxy_any))
        .route("/api/library", get(list_library).post(add_library))
        .route("/api/library/:book_id", axum::routing::delete(remove_library))
        .route("/api/jobs", get(list_jobs).post(create_job))
        .route("/api/jobs/:id", get(get_job))
        .route("/tg/webhook", post(proxy_any))
        .route("/tg/setup", get(proxy_get))
        .with_state(state)
        .layer(CorsLayer::very_permissive())
        .layer(TraceLayer::new_for_http());

    let addr: SocketAddr = config.bind.parse()?;
    info!(%addr, upstream = ?config.upstream_base_url, "NovelDownloader Rust server listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn index(State(state): State<AppState>) -> Json<Value> {
    Json(json!({
        "code": 200,
        "name": "noveldownloader-rust",
        "upstream_mode": state.config.upstream_base_url.is_some(),
        "endpoints": [
            "/read", "/api/search?key=", "/api/detail?book_id=", "/api/directory?book_id=",
            "/api/content?item_id=", "/api/download?book_id=&start=&count=",
            "/api/export?book_id=&format=txt|html|epub", "/api/export/zip?book_id=&format=txt|html",
            "/api/library", "/api/jobs", "POST /tg/webhook"
        ]
    }))
}

async fn health(State(state): State<AppState>) -> Json<Value> {
    Json(json!({
        "code": 200,
        "status": "ok",
        "upstream_configured": state.config.upstream_base_url.is_some(),
        "cookie_configured": state.config.fanqie_cookie.is_some(),
    }))
}

async fn read_page() -> Html<&'static str> { Html(READ_PAGE) }

/// API compatibility gateway. In production, point WORKER_UPSTREAM_URL at the
/// signed Cloudflare Worker so existing signing/decryption/DO/Bot semantics are retained.
async fn proxy_get(
    State(state): State<AppState>,
    uri: Uri,
    Query(query): Query<ApiQuery>,
) -> Response {
    forward_upstream(&state, Method::GET, uri.path(), query.values, HeaderMap::new(), Vec::new()).await
}

async fn proxy_any(
    State(state): State<AppState>,
    method: Method,
    uri: Uri,
    Query(query): Query<ApiQuery>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    forward_upstream(&state, method, uri.path(), query.values, headers, body.to_vec()).await
}

async fn forward_upstream(
    state: &AppState,
    method: Method,
    path: &str,
    query: HashMap<String, String>,
    source_headers: HeaderMap,
    body: Vec<u8>,
) -> Response {
    let Some(base) = &state.config.upstream_base_url else {
        return api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "WORKER_UPSTREAM_URL is not configured. Native Rust signing/decryption is intentionally isolated for the next porting stage.",
        );
    };

    let mut url = match url::Url::parse(&format!("{base}{path}")) {
        Ok(url) => url,
        Err(e) => return api_error(StatusCode::INTERNAL_SERVER_ERROR, &format!("invalid upstream URL: {e}")),
    };
    url.query_pairs_mut().extend_pairs(query.iter().map(|(k, v)| (k.as_str(), v.as_str())));

    let req_method = match reqwest::Method::from_bytes(method.as_str().as_bytes()) {
        Ok(m) => m,
        Err(_) => return api_error(StatusCode::METHOD_NOT_ALLOWED, "unsupported method"),
    };
    let mut req = state.client.request(req_method, url);
    req = req.header(header::ACCEPT, "application/json, text/plain, */*");
    if !body.is_empty() { req = req.body(body); }
    for name in [header::CONTENT_TYPE, header::AUTHORIZATION] {
        if let Some(value) = source_headers.get(&name) { req = req.header(name, value); }
    }

    match req.send().await {
        Ok(upstream) => upstream_response(upstream).await,
        Err(e) => {
            error!(error = %e, "upstream request failed");
            api_error(StatusCode::BAD_GATEWAY, "upstream Worker request failed")
        }
    }
}

async fn upstream_response(upstream: reqwest::Response) -> Response {
    let status = StatusCode::from_u16(upstream.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
    let content_type = upstream.headers().get(header::CONTENT_TYPE).cloned();
    let content_disposition = upstream.headers().get(header::CONTENT_DISPOSITION).cloned();
    match upstream.bytes().await {
        Ok(bytes) => {
            let mut response = Response::new(Body::from(bytes));
            *response.status_mut() = status;
            if let Some(value) = content_type { response.headers_mut().insert(header::CONTENT_TYPE, value); }
            if let Some(value) = content_disposition { response.headers_mut().insert(header::CONTENT_DISPOSITION, value); }
            response.headers_mut().insert("x-noveldownloader-gateway", HeaderValue::from_static("rust"));
            response
        }
        Err(_) => api_error(StatusCode::BAD_GATEWAY, "unable to read upstream response"),
    }
}

async fn list_library(State(state): State<AppState>) -> Json<Value> {
    let books: Vec<LibraryBook> = state.library.read().await.values().cloned().collect();
    Json(json!({ "code": 200, "data": books }))
}

async fn add_library(State(state): State<AppState>, Json(book): Json<LibraryBook>) -> Json<Value> {
    if book.book_id.trim().is_empty() { return Json(json!({"code": 400, "message": "book_id required"})); }
    state.library.write().await.insert(book.book_id.clone(), book.clone());
    Json(json!({ "code": 200, "data": book }))
}

async fn remove_library(State(state): State<AppState>, Path(book_id): Path<String>) -> Json<Value> {
    let deleted = state.library.write().await.remove(&book_id).is_some();
    Json(json!({ "code": 200, "deleted": deleted }))
}

async fn create_job(State(state): State<AppState>, Json(input): Json<CreateJob>) -> Json<Value> {
    let job = DownloadJob {
        id: Uuid::new_v4(),
        book_id: input.book_id,
        format: input.format,
        status: "queued".into(),
        created_at: format!("{:?}", SystemTime::now()),
    };
    state.jobs.write().await.insert(job.id, job.clone());
    Json(json!({ "code": 202, "data": job, "note": "Use /api/export for synchronous exports; connect a persistent queue for background processing." }))
}

async fn list_jobs(State(state): State<AppState>) -> Json<Value> {
    let jobs: Vec<DownloadJob> = state.jobs.read().await.values().cloned().collect();
    Json(json!({ "code": 200, "data": jobs }))
}

async fn get_job(State(state): State<AppState>, Path(id): Path<Uuid>) -> Response {
    match state.jobs.read().await.get(&id).cloned() {
        Some(job) => Json(json!({ "code": 200, "data": job })).into_response(),
        None => api_error(StatusCode::NOT_FOUND, "job not found"),
    }
}

fn api_error(status: StatusCode, message: &str) -> Response {
    (status, Json(json!({ "code": status.as_u16(), "message": message }))).into_response()
}
