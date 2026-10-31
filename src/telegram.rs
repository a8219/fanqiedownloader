//! Telegram Bot: messaging, document upload, webhook registration.

use crate::config::Config;
use serde_json::json;

#[derive(Clone)]
pub struct TelegramBot {
    pub token: String,
    pub http: reqwest::Client,
}

impl TelegramBot {
    pub fn from_config(config: &Config) -> Option<Self> {
        config.tg_bot_token.as_ref().map(|t| Self::from_token(t.clone()))
    }

    pub fn from_token(token: String) -> Self {
        Self {
            token,
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("tg client"),
        }
    }

    fn api(&self, method: &str) -> String {
        format!("https://api.telegram.org/bot{}/{}", self.token, method)
    }

    pub async fn send_message(&self, chat_id: &str, text: &str) -> anyhow::Result<()> {
        let resp = self
            .http
            .post(self.api("sendMessage"))
            .json(&json!({
                "chat_id": chat_id,
                "text": text,
                "disable_web_page_preview": true,
            }))
            .send()
            .await?;
        if !resp.status().is_success() {
            tracing::warn!("tg sendMessage {}: {}", resp.status(), resp.text().await.unwrap_or_default());
        }
        Ok(())
    }

    pub async fn send_document(
        &self,
        chat_id: &str,
        filename: &str,
        data: Vec<u8>,
        caption: &str,
    ) -> anyhow::Result<()> {
        let part = reqwest::multipart::Part::bytes(data)
            .file_name(filename.to_string())
            .mime_str("text/plain")?;
        let form = reqwest::multipart::Form::new()
            .text("chat_id", chat_id.to_string())
            .text("caption", caption.to_string())
            .part("document", part);
        let resp = self.http.post(self.api("sendDocument")).multipart(form).send().await?;
        if !resp.status().is_success() {
            tracing::warn!("tg sendDocument {}: {}", resp.status(), resp.text().await.unwrap_or_default());
        }
        Ok(())
    }

    pub async fn set_webhook(&self, url: &str) -> anyhow::Result<()> {
        let resp = self
            .http
            .post(self.api("setWebhook"))
            .json(&json!({ "url": url, "allowed_updates": ["message", "callback_query"] }))
            .send()
            .await?;
        if !resp.status().is_success() {
            anyhow::bail!("setWebhook failed: {}", resp.status());
        }
        Ok(())
    }
}
