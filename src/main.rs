use noveldownloader::{Config, server};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| "info,tower_http=info".into()))
        .init();

    let config = Config::from_env();
    tracing::info!(
        bind = %config.bind_addr,
        cookie = config.fanqie_cookie.is_some(),
        proxy = config.proxy_api.is_some(),
        tg = config.tg_bot_token.is_some(),
        "NovelDownloader starting"
    );

    server::run(config).await
}
