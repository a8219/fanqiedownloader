//! Server state and application entry point.

use std::{collections::HashMap, sync::Arc};
use axum::Router;
use tokio::sync::RwLock;
use crate::config::Config;
use crate::api::routes;

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub http: reqwest::Client,
    pub upstream_url: Option<String>,
    pub library: Arc<RwLock<HashMap<String, serde_json::Value>>>,
    pub jobs: Arc<RwLock<HashMap<String, serde_json::Value>>>,
}

pub async fn run(config: Config) -> anyhow::Result<()> {
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(config.request_timeout_secs))
        .user_agent("NovelDownloader/0.2")
        .brotli(true)
        .gzip(true)
        .build()?;

    let state = AppState {
        config: config.clone(),
        http,
        upstream_url: None, // Native mode - no upstream needed
        library: Arc::new(RwLock::new(HashMap::new())),
        jobs: Arc::new(RwLock::new(HashMap::new())),
    };

    let app: Router = routes::build_router(state);
    let addr: std::net::SocketAddr = config.bind_addr.parse()?;
    tracing::info!(%addr, "NovelDownloader Rust server listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
