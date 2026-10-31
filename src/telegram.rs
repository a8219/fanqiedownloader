//! Telegram Bot client.

use crate::config::Config;
use serde_json::json;

#[derive(Clone)]
pub struct TelegramBot {
    token: String,
    http: reqwest::Client,
}

impl TelegramBot {
    pub fn from_config(cfg: &Config) -> Option<Self> {
        cfg.tg_bot_token.clone().map(Self::from_token)
    }

    pub fn from_token(token: String) -> Self {
        Self {
            token,
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .expect("telegram client"),
        }
    }

    fn endpoint(&self, method: &str) -> String {
        format!("https://api.telegram.org/bot{}/{}", self.token, method)
    }

    pub async fn send_message(&self, chat_id: &str, text: &str) -> anyhow::Result<()> {
        let resp = self
            .http
            .post(self.endpoint("sendMessage"))
            .json(&json!({
                "chat_id": chat_id,
                "text": text,
                "disable_web_page_preview": true,
            }))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("sendMessage {status}: {body}");
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
            .mime_str("text/plain; charset=utf-8")?;

        let form = reqwest::multipart::Form::new()
            .text("chat_id", chat_id.to_string())
            .text("caption", caption.to_string())
            .part("document", part);

        let resp = self
            .http
            .post(self.endpoint("sendDocument"))
            .multipart(form)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("sendDocument {status}: {body}");
        }
        Ok(())
    }

    pub async fn set_webhook(&self, url: &str, secret: Option<&str>) -> anyhow::Result<()> {
        let mut payload = json!({
            "url": url,
            "allowed_updates": ["message", "channel_post", "callback_query"],
            "drop_pending_updates": true,
        });
        if let Some(secret) = secret {
            payload["secret_token"] = json!(secret);
        }

        let resp = self
            .http
            .post(self.endpoint("setWebhook"))
            .json(&payload)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("setWebhook {status}: {body}");
        }
        Ok(())
    }

    pub async fn delete_webhook(&self) -> anyhow::Result<()> {
        self.http
            .post(self.endpoint("deleteWebhook"))
            .json(&json!({ "drop_pending_updates": true }))
            .send()
            .await?;
        Ok(())
    }
}
