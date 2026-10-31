//! Application state and server bootstrap.

use std::{collections::HashMap, sync::Arc};
use axum::Router;
use tokio::sync::RwLock;
use crate::api::routes;
use crate::config::Config;

/// Cache for a successfully registered device (shared across requests).
#[derive(Clone, Debug)]
pub struct DeviceCache {
    pub device_id: String,
    pub install_id: String,
    pub device_type: String,
    pub key: Option<Vec<u8>>,
    pub keyver: Option<String>,
}

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub http: reqwest::Client,
    pub library: Arc<RwLock<HashMap<String, serde_json::Value>>>,
    pub jobs: Arc<RwLock<HashMap<String, serde_json::Value>>>,
    /// Shared device/key cache so we don't re-register on every chapter fetch.
    pub device: Arc<RwLock<Option<DeviceCache>>>,
    /// Per-book chapter-count records for update detection.
    pub records: Arc<RwLock<HashMap<String, usize>>>,
}

pub async fn run(config: Config) -> anyhow::Result<()> {
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(config.request_timeout_secs))
        .user_agent("NovelDownloader/0.3")
        .brotli(true)
        .gzip(true)
        .pool_max_idle_per_host(32)
        .build()?;

    let state = AppState {
        config: config.clone(),
        http,
        library: Arc::new(RwLock::new(HashMap::new())),
        jobs: Arc::new(RwLock::new(HashMap::new())),
        device: Arc::new(RwLock::new(None)),
        records: Arc::new(RwLock::new(HashMap::new())),
    };

    let app: Router = routes::build_router(state);
    let addr: std::net::SocketAddr = config.bind_addr.parse()?;
    tracing::info!(%addr, "NovelDownloader listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
