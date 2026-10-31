use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub bind_addr: String,
    pub request_timeout_secs: u64,
    pub fanqie_cookie: Option<String>,
    pub proxy_api: Option<String>,
    pub tg_bot_token: Option<String>,
    pub admin_username: Option<String>,
    pub admin_password: Option<String>,
    pub online_read: bool,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            bind_addr: env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".into()),
            request_timeout_secs: env::var("REQUEST_TIMEOUT_SECS").ok().and_then(|v| v.parse().ok()).unwrap_or(45),
            fanqie_cookie: env::var("FANQIE_COOKIE").ok().filter(|v| !v.is_empty()),
            proxy_api: env::var("FANQIE_PROXY_API").ok().filter(|v| !v.is_empty()),
            tg_bot_token: env::var("TG_BOT_TOKEN").ok().filter(|v| !v.is_empty()),
            admin_username: env::var("FANQIE_ADMIN_USERNAME").ok().filter(|v| !v.is_empty()),
            admin_password: env::var("FANQIE_ADMIN_PASSWORD").ok().filter(|v| !v.is_empty()),
            online_read: env::var("FANQIE_ONLINE_READ").map(|v| v == "1").unwrap_or(true),
        }
    }
}
