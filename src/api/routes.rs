//! API route definitions for the Axum router.

use axum::{routing::{any, get, post}, Router};
use crate::server::AppState;
use crate::api::handlers;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/", get(handlers::index))
        .route("/health", get(handlers::health))
        .route("/read", get(handlers::read_page))
        .route("/read/", get(handlers::read_page))
        .route("/api/search", get(handlers::api_search))
        .route("/api/detail", get(handlers::api_detail))
        .route("/api/directory", get(handlers::api_directory))
        .route("/api/content", get(handlers::api_content))
        .route("/api/download", get(handlers::api_download))
        .route("/api/export", get(handlers::api_export))
        .route("/api/export/zip", get(handlers::api_export_zip))
        .route("/api/update", get(handlers::api_update))
        .route("/api/config", any(handlers::api_config))
        .route("/api/proxy/pool", any(handlers::api_proxy_pool))
        .route("/api/library", get(handlers::list_library).post(handlers::add_library))
        .route("/api/library/:book_id", axum::routing::delete(handlers::remove_library))
        .route("/api/jobs", get(handlers::list_jobs).post(handlers::create_job))
        .route("/api/jobs/:id", get(handlers::get_job))
        .route("/tg/webhook", post(handlers::tg_webhook))
        .route("/tg/setup", get(handlers::tg_setup))
        .with_state(state)
        .layer(CorsLayer::very_permissive())
        .layer(TraceLayer::new_for_http())
}
