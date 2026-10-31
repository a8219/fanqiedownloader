//! API request handlers.

use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response},
    Json,
};
use serde_json::{json, Value};
use std::collections::HashMap;
use uuid::Uuid;
use crate::server::AppState;

pub async fn index(State(state): State<AppState>) -> Json<Value> {
    Json(json!({
        "code": 200,
        "name": "noveldownloader-rust",
        "version": "0.2.0",
        "features": ["native-signing", "device-registration", "chapter-decryption", "font-decoding", "search", "export"],
        "endpoints": [
            "/read", "/api/search?key=", "/api/detail?book_id=", "/api/directory?book_id=",
            "/api/content?item_id=", "/api/download?book_id=&start=&count=",
            "/api/export?book_id=&format=txt|html|epub", "/api/export/zip?book_id=&format=txt|html",
            "/api/library", "/api/jobs", "POST /tg/webhook"
        ]
    }))
}

pub async fn health(State(state): State<AppState>) -> Json<Value> {
    Json(json!({
        "code": 200,
        "status": "ok",
        "cookie_configured": state.config.fanqie_cookie.is_some(),
        "proxy_configured": state.config.proxy_api.is_some(),
        "online_read": state.config.online_read,
    }))
}

pub async fn read_page() -> Html<&'static str> {
    Html(crate::read_page::READ_PAGE_HTML)
}

pub async fn api_search(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let key = params.get("key").cloned().unwrap_or_default();
    if key.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "key required");
    }

    // Try proxy first if configured
    if let Some(proxy) = &state.config.proxy_api {
        if let Ok(results) = crate::search::search_via_proxy(&key, proxy, &state.http).await {
            return Json(json!({"code": 200, "from": "proxy", "data": {"search_book_data_list": results}})).into_response();
        }
    }

    // Native APP search (self-contained)
    match crate::search::search(&key, &state.config).await {
        Ok(results) => Json(json!({"code": 200, "from": "app", "data": {"search_book_data_list": results, "total_count": results.len()}})).into_response(),
        Err(e) => {
            // Fallback: treat numeric key as book_id
            if key.chars().all(|c| c.is_ascii_digit()) && key.len() >= 15 {
                return Json(json!({"code": 200, "from": "id", "data": {"book_ids": [key]}, "note": "key treated as book_id"})).into_response();
            }
            json_error(StatusCode::NOT_IMPLEMENTED, &format!("search failed: {}", e))
        }
    }
}

pub async fn api_detail(State(state): State<AppState>, Query(params): Query<HashMap<String, String>>) -> Response {
    proxy_or_error(&state, "/api/detail", params).await
}

pub async fn api_directory(State(state): State<AppState>, Query(params): Query<HashMap<String, String>>) -> Response {
    proxy_or_error(&state, "/api/directory", params).await
}

pub async fn api_content(State(state): State<AppState>, Query(params): Query<HashMap<String, String>>) -> Response {
    proxy_or_error(&state, "/api/content", params).await
}

pub async fn api_download(State(state): State<AppState>, Query(params): Query<HashMap<String, String>>) -> Response {
    proxy_or_error(&state, "/api/download", params).await
}

pub async fn api_export(State(state): State<AppState>, Query(params): Query<HashMap<String, String>>) -> Response {
    proxy_or_error(&state, "/api/export", params).await
}

pub async fn api_export_zip(State(state): State<AppState>, Query(params): Query<HashMap<String, String>>) -> Response {
    proxy_or_error(&state, "/api/export/zip", params).await
}

pub async fn api_update(State(state): State<AppState>, Query(params): Query<HashMap<String, String>>) -> Response {
    proxy_or_error(&state, "/api/update", params).await
}

pub async fn api_config(State(state): State<AppState>, Query(_params): Query<HashMap<String, String>>) -> Response {
    Json(json!({
        "code": 200,
        "proxy_api": state.config.proxy_api,
        "cookie_configured": state.config.fanqie_cookie.is_some(),
    })).into_response()
}

pub async fn api_proxy_pool() -> Response {
    Json(json!({"code": 200, "pool": [], "message": "Native mode: no proxy pool needed"})).into_response()
}

pub async fn list_library(State(state): State<AppState>) -> Response {
    let books: Vec<Value> = state.library.read().await.values().cloned().collect();
    Json(json!({"code": 200, "data": books})).into_response()
}

pub async fn add_library(State(state): State<AppState>, Json(book): Json<Value>) -> Response {
    let book_id = book.get("book_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
    if book_id.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "book_id required");
    }
    state.library.write().await.insert(book_id, book.clone());
    Json(json!({"code": 200, "data": book})).into_response()
}

pub async fn remove_library(State(state): State<AppState>, Path(book_id): Path<String>) -> Response {
    let deleted = state.library.write().await.remove(&book_id).is_some();
    Json(json!({"code": 200, "deleted": deleted})).into_response()
}

pub async fn create_job(State(state): State<AppState>, Json(body): Json<Value>) -> Response {
    let job = json!({
        "id": Uuid::new_v4().to_string(),
        "book_id": body.get("book_id").and_then(|v| v.as_str()).unwrap_or(""),
        "format": body.get("format").and_then(|v| v.as_str()).unwrap_or("txt"),
        "status": "queued",
    });
    let id = job["id"].as_str().unwrap().to_string();
    state.jobs.write().await.insert(id, job.clone());
    Json(json!({"code": 202, "data": job})).into_response()
}

pub async fn list_jobs(State(state): State<AppState>) -> Response {
    let jobs: Vec<Value> = state.jobs.read().await.values().cloned().collect();
    Json(json!({"code": 200, "data": jobs})).into_response()
}

pub async fn get_job(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    match state.jobs.read().await.get(&id).cloned() {
        Some(job) => Json(json!({"code": 200, "data": job})).into_response(),
        None => json_error(StatusCode::NOT_FOUND, "job not found"),
    }
}

pub async fn tg_webhook(State(state): State<AppState>, body: axum::body::Bytes) -> Response {
    // Forward to upstream Worker if configured, otherwise handle natively
    if let Some(upstream) = &state.upstream_url {
        let url = format!("{}{}", upstream, "/tg/webhook");
        match state.http.post(&url).header("Content-Type", "application/json").body(body.to_vec()).send().await {
            Ok(res) => {
                let status = res.status();
                let resp_body = res.bytes().await.unwrap_or_default();
                let mut response = Response::new(Body::from(resp_body));
                *response.status_mut() = status;
                response
            }
            Err(_) => json_error(StatusCode::BAD_GATEWAY, "upstream TG webhook failed"),
        }
    } else {
        Json(json!({"code": 200, "message": "webhook received (native TG handler not yet implemented)"})).into_response()
    }
}

pub async fn tg_setup(State(state): State<AppState>) -> Response {
    if let Some(token) = &state.config.tg_bot_token {
        Json(json!({"code": 200, "message": "token configured", "has_token": true})).into_response()
    } else {
        json_error(StatusCode::BAD_REQUEST, "TG_BOT_TOKEN not configured")
    }
}

async fn proxy_or_error(state: &AppState, path: &str, params: HashMap<String, String>) -> Response {
    // Try native implementation first for content requests
    if path == "/api/content" {
        if let Some(item_id) = params.get("item_id") {
            // Try native chapter fetch
            let fetcher = crate::crypto::chapter::ChapterFetcher::new();
            // Note: ChapterFetcher needs mutable state; for now, return error if no upstream
            // Full native implementation will use a shared fetcher in AppState
        }
    }

    // Fallback to upstream Worker if configured
    if let Some(upstream) = &state.upstream_url {
        let mut url = format!("{}{}", upstream, path);
        if !params.is_empty() {
            let qs: Vec<String> = params.iter().map(|(k, v)| format!("{}={}", urlencode(k), urlencode(v))).collect();
            url.push('?');
            url.push_str(&qs.join("&"));
        }
        match state.http.get(&url).header(header::ACCEPT, "application/json").send().await {
            Ok(res) => {
                let status = res.status();
                let ct = res.headers().get(header::CONTENT_TYPE).cloned();
                let cd = res.headers().get(header::CONTENT_DISPOSITION).cloned();
                let bytes = res.bytes().await.unwrap_or_default();
                let mut response = Response::new(Body::from(bytes));
                *response.status_mut() = status;
                if let Some(v) = ct { response.headers_mut().insert(header::CONTENT_TYPE, v); }
                if let Some(v) = cd { response.headers_mut().insert(header::CONTENT_DISPOSITION, v); }
                response
            }
            Err(e) => json_error(StatusCode::BAD_GATEWAY, &format!("upstream error: {}", e)),
        }
    } else {
        json_error(StatusCode::SERVICE_UNAVAILABLE, &format!("No upstream configured for {}", path))
    }
}

fn json_error(status: StatusCode, msg: &str) -> Response {
    (status, Json(json!({"code": status.as_u16(), "message": msg}))).into_response()
}

fn urlencode(s: &str) -> String {
    s.bytes().map(|b| {
        if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.' || b == b'~' {
            (b as char).to_string()
        } else {
            format!("%{:02X}", b)
        }
    }).collect()
}
