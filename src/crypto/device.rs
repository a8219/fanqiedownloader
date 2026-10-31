//! Anonymous device registration and chapter key provisioning.
//!
//! Derived from fanqie-assistant (GPL-3.0). See THIRD_PARTY_NOTICES.md.

use base64::Engine;
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::crypto::sign::{sign_request, AID, APP_UA, READING_BASE, REGISTER_URL, SHARED_KEY};
use crate::crypto::ttencrypt;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Device {
    pub device_id: String,
    pub install_id: String,
    pub device_type: String,
}

#[derive(Clone, Debug)]
pub struct KeyInfo {
    pub key: Vec<u8>,
    pub keyver: String,
}

#[derive(Debug)]
pub struct ChapterContent {
    pub content: String,
    pub title: String,
    pub book_name: String,
    pub key_version: Option<String>,
}

pub struct DeviceClient {
    http: reqwest::Client,
}

impl DeviceClient {
    pub fn new(http: reqwest::Client) -> Self {
        Self { http }
    }

    /// Build the `?k=v&...` query used by every APP endpoint.
    pub fn build_query(device: &Device, extra: &[(&str, &str)]) -> String {
        let mut pairs: Vec<(&str, &str)> = vec![
            ("iid", device.install_id.as_str()),
            ("device_id", device.device_id.as_str()),
            ("ac", "wifi"),
            ("channel", "43536163a"),
            ("aid", AID),
            ("app_name", "novelapp"),
            ("version_code", "70132"),
            ("version_name", "7.0.1.32"),
            ("device_platform", "android"),
            ("os", "android"),
            ("ssmix", "a"),
            ("os_version", "10"),
            ("device_type", device.device_type.as_str()),
            ("device_brand", "realme"),
            ("update_version_code", "70132"),
            ("manifest_version_code", "70132"),
        ];
        pairs.extend_from_slice(extra);

        pairs
            .into_iter()
            .map(|(k, v)| format!("{k}={}", encode(v)))
            .collect::<Vec<_>>()
            .join("&")
    }

    /// POST /service/2/device_register/ to obtain a fresh anonymous device.
    pub async fn register_device(&self) -> anyhow::Result<Device> {
        let body = build_device_body();
        let plaintext = serde_json::to_vec(&body)?;
        let ciphertext = ttencrypt::tt_encrypt(&plaintext);

        let resp = self
            .http
            .post(REGISTER_URL)
            .header(
                reqwest::header::CONTENT_TYPE,
                "application/octet-stream; tt-data=a",
            )
            .header(reqwest::header::USER_AGENT, "okhttp/4.10.0")
            .body(ciphertext)
            .send()
            .await?;

        if !resp.status().is_success() {
            anyhow::bail!("device_register HTTP {}", resp.status());
        }

        let json: serde_json::Value = resp.json().await?;

        let device_id = json
            .get("device_id_str")
            .and_then(|v| v.as_str())
            .or_else(|| json.get("device_id").and_then(|v| v.as_str()))
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("device_register: missing device_id"))?;

        let install_id = json
            .get("install_id_str")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("device_register: missing install_id_str"))?;

        Ok(Device {
            device_id,
            install_id,
            device_type: "P30".to_string(),
        })
    }

    /// POST /reading/crypt/registerkey to obtain the chapter decryption key.
    pub async fn register_key(&self, device: &Device) -> anyhow::Result<KeyInfo> {
        let url = format!(
            "{}/reading/crypt/registerkey?{}",
            READING_BASE,
            Self::build_query(device, &[])
        );

        let device_numeric: u128 = device
            .device_id
            .parse()
            .map_err(|_| anyhow::anyhow!("device_id is not numeric"))?;
        let id_bytes = device_numeric.to_be_bytes();

        let iv: [u8; 16] = rand::thread_rng().gen();
        let encrypted = ttencrypt::aes_cbc_encrypt(&SHARED_KEY, &iv, &id_bytes);

        let mut blob = Vec::with_capacity(16 + encrypted.len());
        blob.extend_from_slice(&iv);
        blob.extend_from_slice(&encrypted);

        let content_b64 = base64::engine::general_purpose::STANDARD.encode(&blob);
        let plain = serde_json::json!({ "content": content_b64 }).to_string();
        let gzipped = ttencrypt::gzip(plain.as_bytes());

        let sig = sign_request(&url, Some(&gzipped));

        let resp = self
            .http
            .post(&url)
            .header("x-argus", &sig.x_argus)
            .header("x-ladon", &sig.x_ladon)
            .header("x-khronos", &sig.x_khronos)
            .header(reqwest::header::USER_AGENT, APP_UA)
            .header(
                reqwest::header::CONTENT_TYPE,
                "application/json; charset=utf-8",
            )
            .header(reqwest::header::CONTENT_ENCODING, "gzip")
            .body(gzipped)
            .send()
            .await?;

        if !resp.status().is_success() {
            anyhow::bail!("registerkey HTTP {}", resp.status());
        }

        let json: serde_json::Value = resp.json().await?;
        let encrypted_key = json
            .pointer("/data/key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("registerkey: missing data.key"))?;
        let keyver = json
            .pointer("/data/keyver")
            .map(|v| {
                v.as_str()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| v.to_string())
            })
            .unwrap_or_else(|| "0".to_string());

        let blob = base64::engine::general_purpose::STANDARD.decode(encrypted_key)?;
        if blob.len() < 32 {
            anyhow::bail!("registerkey: key blob too short");
        }
        let key = ttencrypt::aes_cbc_decrypt(&SHARED_KEY, &blob[..16], &blob[16..]);

        Ok(KeyInfo { key, keyver })
    }

    /// GET /reading/reader/full/v and decrypt the returned chapter body.
    pub async fn fetch_chapter(
        &self,
        device: &Device,
        key: &KeyInfo,
        item_id: &str,
    ) -> anyhow::Result<ChapterContent> {
        let url = format!(
            "{}/reading/reader/full/v?{}",
            READING_BASE,
            Self::build_query(device, &[("item_id", item_id), ("req_type", "1")])
        );
        let sig = sign_request(&url, None);

        let resp = self
            .http
            .get(&url)
            .header("x-argus", &sig.x_argus)
            .header("x-ladon", &sig.x_ladon)
            .header("x-khronos", &sig.x_khronos)
            .header(reqwest::header::USER_AGENT, APP_UA)
            .send()
            .await?;

        if !resp.status().is_success() {
            anyhow::bail!("reader/full HTTP {}", resp.status());
        }

        let json: serde_json::Value = resp.json().await?;
        let data = json
            .get("data")
            .ok_or_else(|| anyhow::anyhow!("reader/full: missing data"))?;

        let title = data
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let book_name = data
            .get("book_name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let key_version = data.get("key_version").map(|v| {
            v.as_str()
                .map(|s| s.to_string())
                .unwrap_or_else(|| v.to_string())
        });

        let raw = data.get("content").and_then(|v| v.as_str()).unwrap_or("");
        if raw.is_empty() || raw == "Invalid" {
            anyhow::bail!("reader/full: content unavailable ({raw})");
        }

        let blob = base64::engine::general_purpose::STANDARD.decode(raw)?;
        if blob.len() < 32 {
            anyhow::bail!("reader/full: ciphertext too short");
        }
        let plaintext = ttencrypt::aes_cbc_decrypt(&key.key, &blob[..16], &blob[16..]);

        Ok(ChapterContent {
            content: String::from_utf8_lossy(&plaintext).to_string(),
            title,
            book_name,
            key_version,
        })
    }
}

fn build_device_body() -> serde_json::Value {
    let mut rng = rand::thread_rng();

    let openudid: String = (0..16)
        .map(|_| format!("{:02x}", rng.gen::<u8>()))
        .collect();
    let device_id: String = (0..15)
        .map(|_| rng.gen_range(0..10u8).to_string())
        .collect();

    serde_json::json!({
        "header": {
            "device_model": "P30",
            "os_api": 29,
            "os_version": "10",
            "device_brand": "HUAWEI",
            "device_manufacturer": "HUAWEI",
            "device_platform": "android",
            "resolution": "1080x2240",
            "display_density": "2.0",
            "device_type": "phone",
            "aid": AID,
            "channel": "43536163a",
            "version_code": "70132",
            "version_name": "7.0.1.32",
            "update_version_code": "70132",
            "manifest_version_code": "70132",
            "device_id": device_id,
            "openudid": openudid,
            "cdid": "",
            "sim_serial_number": [],
            "ssmix": "a",
        }
    })
}

fn encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len() * 3);
    for b in input.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}
