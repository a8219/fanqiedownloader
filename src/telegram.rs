//! Telegram Bot handler (skeleton for native implementation).

use crate::config::Config;

pub struct TelegramBot {
    pub token: String,
    pub http: reqwest::Client,
}

impl TelegramBot {
    pub fn new(config: &Config) -> Option<Self> {
        config.tg_bot_token.as_ref().map(|token| Self {
            token: token.clone(),
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap(),
        })
    }

    pub async fn send_message(&self, chat_id: &str, text: &str) -> anyhow::Result<()> {
        let url = format!("https://api.telegram.org/bot{}/sendMessage", self.token);
        self.http.post(&url)
            .json(&serde_json::json!({"chat_id": chat_id, "text": text}))
            .send()
            .await?;
        Ok(())
    }

    pub async fn send_document(&self, chat_id: &str, filename: &str, data: Vec<u8>, caption: &str) -> anyhow::Result<()> {
        let url = format!("https://api.telegram.org/bot{}/sendDocument", self.token);
        let part = reqwest::multipart::Part::bytes(data)
            .file_name(filename.to_string());
        let form = reqwest::multipart::Form::new()
            .text("chat_id", chat_id.to_string())
            .text("caption", caption.to_string())
            .part("document", part);
        self.http.post(&url).multipart(form).send().await?;
        Ok(())
    }
}
