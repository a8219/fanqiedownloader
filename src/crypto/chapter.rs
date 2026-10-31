//! Chapter fetch pipeline: device check -> key check -> decrypt -> font decode.
//!
//! Derived from fanqie-assistant (GPL-3.0). See THIRD_PARTY_NOTICES.md.

use crate::crypto::device::{ChapterContent, Device, DeviceClient, KeyInfo};
use crate::font_map;
use crate::server::DeviceCache;

pub struct ChapterFetcher {
    client: DeviceClient,
    device: Option<Device>,
    key: Option<KeyInfo>,
}

impl ChapterFetcher {
    pub fn new(http: reqwest::Client) -> Self {
        Self {
            client: DeviceClient::new(http),
            device: None,
            key: None,
        }
    }

    /// Restore a previously registered device + key (warm path).
    pub fn restore(
        &mut self,
        device_id: String,
        install_id: String,
        device_type: String,
        key: Option<Vec<u8>>,
        keyver: Option<String>,
    ) {
        self.device = Some(Device {
            device_id,
            install_id,
            device_type,
        });
        if let (Some(k), Some(v)) = (key, keyver) {
            self.key = Some(KeyInfo { key: k, keyver: v });
        }
    }

    /// Snapshot the current device/key for caching.
    pub fn export(&self) -> Option<DeviceCache> {
        let d = self.device.as_ref()?;
        Some(DeviceCache {
            device_id: d.device_id.clone(),
            install_id: d.install_id.clone(),
            device_type: d.device_type.clone(),
            key: self.key.as_ref().map(|k| k.key.clone()),
            keyver: self.key.as_ref().map(|k| k.keyver.clone()),
        })
    }

    async fn ensure_device(&mut self) -> anyhow::Result<()> {
        if self.device.is_none() {
            let d = self.client.register_device().await?;
            tracing::info!(device_id = %d.device_id, "device registered");
            self.device = Some(d);
        }
        Ok(())
    }

    async fn ensure_key(&mut self) -> anyhow::Result<()> {
        if self.key.is_none() {
            let device = self
                .device
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("device not ready"))?;
            let k = self.client.register_key(device).await?;
            tracing::info!(keyver = %k.keyver, "chapter key registered");
            self.key = Some(k);
        }
        Ok(())
    }

    /// Full chapter: decrypt, decode the custom font, strip tags.
    pub async fn get_full_chapter(&mut self, item_id: &str) -> anyhow::Result<ChapterContent> {
        self.ensure_device().await?;
        self.ensure_key().await?;

        let device = self.device.as_ref().expect("device");
        let key = self.key.as_ref().expect("key");

        let mut ch = self.client.fetch_chapter(device, key, item_id).await?;
        ch.content = font_map::decode_font(&strip_tags(&ch.content));
        Ok(ch)
    }
}

/// Remove markup and normalise whitespace.
pub fn strip_tags(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;

    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                out.push('\n');
            }
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }

    let decoded = out
        .replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&#39;", "'")
        .replace("&amp;", "&");

    let mut lines: Vec<&str> = decoded.lines().map(|l| l.trim()).collect();
    lines.retain(|l| !l.is_empty());

    lines.join("\n")
}
