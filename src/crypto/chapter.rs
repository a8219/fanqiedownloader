//! Chapter content fetching and decryption pipeline.

use crate::crypto::device::{DeviceClient, Device, KeyInfo, ChapterContent};
use crate::font_map;

pub struct ChapterFetcher {
    client: DeviceClient,
    device: Option<Device>,
    key_info: Option<KeyInfo>,
}

impl ChapterFetcher {
    pub fn new() -> Self {
        Self {
            client: DeviceClient::new(),
            device: None,
            key_info: None,
        }
    }

    /// Ensure device is registered and key is obtained.
    async fn ensure_device(&mut self) -> anyhow::Result<(&Device, &KeyInfo)> {
        if self.device.is_none() {
            let dev = self.client.register_device().await?;
            tracing::info!("Device registered: {} / {}", dev.device_id, dev.install_id);
            self.device = Some(dev);
        }
        if self.key_info.is_none() {
            let dev = self.device.as_ref().unwrap();
            let key = self.client.register_key(dev).await?;
            tracing::info!("Key registered: ver={}", key.keyver);
            self.key_info = Some(key);
        }
        Ok((self.device.as_ref().unwrap(), self.key_info.as_ref().unwrap()))
    }

    /// Get a full chapter by item_id, with font decoding and tag stripping.
    pub async fn get_full_chapter(&mut self, item_id: &str) -> anyhow::Result<ChapterContent> {
        let (device, key_info) = self.ensure_device().await?;
        let mut ch = self.client.get_chapter(device, key_info, item_id).await?;
        // Decode font obfuscation
        ch.content = font_map::decode_font(&ch.content);
        // Strip HTML tags
        ch.content = strip_tags(&ch.content);
        Ok(ch)
    }
}

/// Strip HTML tags and decode entities.
pub fn strip_tags(html: &str) -> String {
    let mut s = html.to_string();
    // Remove script/style blocks
    s = regex_like_replace(&s, "<script[^>]*>[\\s\\S]*?</script>", "");
    s = regex_like_replace(&s, "<style[^>]*>[\\s\\S]*?</style>", "");
    // Convert breaks to newlines
    s = s.replace("<br />", "\n").replace("<br>", "\n").replace("<br/>", "\n");
    s = s.replace("</p>", "\n");
    // Strip remaining tags
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        if c == '<' { in_tag = true; }
        if !in_tag { out.push(c); }
        if c == '>' { in_tag = false; }
    }
    // Decode entities
    out = out.replace("&nbsp;", " ")
             .replace("&amp;", "&")
             .replace("&lt;", "<")
             .replace("&gt;", ">")
             .replace(""", "\"")
             .replace("'", "'");
    // Collapse excessive newlines
    while out.contains("\n\n\n") {
        out = out.replace("\n\n\n", "\n\n");
    }
    out.trim().to_string()
}

/// Simple regex-like replace (literal string match with wildcards).
fn regex_like_replace(input: &str, _pattern: &str, replacement: &str) -> String {
    // Simplified: just remove script/style blocks by finding tags
    // For production, use the `regex` crate.
    let mut result = String::with_capacity(input.len());
    let lower = input.to_lowercase();
    let mut i = 0;
    while i < input.len() {
        if lower[i..].starts_with("<script") {
            if let Some(end) = lower[i..].find("</script>") {
                i += end + 9;
                continue;
            }
        }
        if lower[i..].starts_with("<style") {
            if let Some(end) = lower[i..].find("</style>") {
                i += end + 8;
                continue;
            }
        }
        if let Some(c) = input[i..].chars().next() {
            result.push(c);
            i += c.len_utf8();
        } else {
            break;
        }
    }
    result
}
