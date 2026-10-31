//! Device registration, key registration, and chapter fetching.

use crate::crypto::sign::*;
use crate::crypto::ttencrypt;
use crate::crypto::md5;
use base64::Engine;
use rand::Rng;
use serde::{Deserialize, Serialize};

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

pub struct DeviceClient {
    pub http: reqwest::Client,
}

impl DeviceClient {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .user_agent("okhttp/4.10.0")
                .build()
                .unwrap(),
        }
    }

    /// Build APP query parameters.
    pub fn build_query(&self, device: &Device, extra: &[(&str, &str)]) -> String {
        let mut params = vec![
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
        params.extend_from_slice(extra);
        params.iter()
            .map(|(k, v)| format!("{}={}", k, urlencode(v)))
            .collect::<Vec<_>>()
            .join("&")
    }

    /// Register a new anonymous device.
    pub async fn register_device(&self) -> anyhow::Result<Device> {
        let body = generate_device_body();
        let encrypted = ttencrypt::tt_encrypt(serde_json::to_vec(&body)?.as_slice());
        
        let res = self.http
            .post(REGISTER_URL)
            .header("Content-Type", "application/octet-stream; tt-data=a")
            .header("User-Agent", "okhttp/4.10.0")
            .body(encrypted)
            .send()
            .await?;
        
        if !res.status().is_success() {
            anyhow::bail!("Device registration failed: HTTP {}", res.status());
        }
        
        let json: serde_json::Value = res.json().await?;
        let device_id = json["device_id_str"].as_str()
            .or_else(|| json["device_id"].as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing device_id in response"))?
            .to_string();
        let install_id = json["install_id_str"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing install_id_str"))?
            .to_string();
        let device_type = body["header"]["device_model"].as_str().unwrap_or("P30").to_string();
        
        Ok(Device { device_id, install_id, device_type })
    }

    /// Register encryption key for chapter decryption.
    pub async fn register_key(&self, device: &Device) -> anyhow::Result<KeyInfo> {
        let url = format!("{}/reading/crypt/registerkey?{}", READING_BASE, self.build_query(device, &[]));
        
        // Convert device_id to 16-byte big-endian
        let id: u128 = device.device_id.parse()
            .map_err(|_| anyhow::anyhow!("Invalid device_id for key registration"))?;
        let id_bytes = id.to_be_bytes();
        
        // AES-CBC encrypt with shared key
        let iv: [u8; 16] = rand::thread_rng().gen();
        let encrypted = ttencrypt::aes_cbc_encrypt(&SHARED_KEY, &iv, &id_bytes);
        let mut content = Vec::with_capacity(iv.len() + encrypted.len());
        content.extend_from_slice(&iv);
        content.extend_from_slice(&encrypted);
        let content_b64 = base64::engine::general_purpose::STANDARD.encode(&content);
        
        let plain_body = serde_json::json!({ "content": content_b64 }).to_string();
        let gzipped = ttencrypt::gzip(plain_body.as_bytes());
        
        let headers = sign_request(&url, Some(&gzipped));
        
        let res = self.http
            .post(&url)
            .header("x-argus", &headers.x_argus)
            .header("x-ladon", &headers.x_ladon)
            .header("x-khronos", &headers.x_khronos)
            .header("User-Agent", APP_UA)
            .header("Content-Type", "application/json; charset=utf-8")
            .header("Content-Encoding", "gzip")
            .body(gzipped)
            .send()
            .await?;
        
        if !res.status().is_success() {
            anyhow::bail!("Key registration failed: HTTP {}", res.status());
        }
        
        let json: serde_json::Value = res.json().await?;
        let encrypted_key = json["data"]["key"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing key in response"))?;
        let keyver = json["data"]["keyver"].as_str().unwrap_or("0").to_string();
        
        let buf = base64::engine::general_purpose::STANDARD.decode(encrypted_key)?;
        let key_iv = &buf[..16];
        let key_data = &buf[16..];
        let final_key = ttencrypt::aes_cbc_decrypt(&SHARED_KEY, key_iv, key_data);
        
        Ok(KeyInfo { key: final_key, keyver })
    }

    /// Fetch a full chapter (encrypted) and decrypt it.
    pub async fn get_chapter(&self, device: &Device, key_info: &KeyInfo, item_id: &str) -> anyhow::Result<ChapterContent> {
        let url = format!("{}/reading/reader/full/v?{}", READING_BASE, 
            self.build_query(device, &[("item_id", item_id), ("req_type", "1")]));
        let headers = sign_request(&url, None);
        
        let res = self.http
            .get(&url)
            .header("x-argus", &headers.x_argus)
            .header("x-ladon", &headers.x_ladon)
            .header("x-khronos", &headers.x_khronos)
            .header("User-Agent", APP_UA)
            .send()
            .await?;
        
        let json: serde_json::Value = res.json().await?;
        let data = &json["data"];
        
        let content_encrypted = data["content"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing content"))?;
        let content_buf = base64::engine::general_purpose::STANDARD.decode(content_encrypted)?;
        
        let iv = &content_buf[..16];
        let ciphertext = &content_buf[16..];
        let plaintext = ttencrypt::aes_cbc_decrypt(&key_info.key, iv, ciphertext);
        
        Ok(ChapterContent {
            content: String::from_utf8_lossy(&plaintext).to_string(),
            title: data["title"].as_str().unwrap_or("").to_string(),
            book_name: data["book_name"].as_str().unwrap_or("").to_string(),
        })
    }
}

#[derive(Debug)]
pub struct ChapterContent {
    pub content: String,
    pub title: String,
    pub book_name: String,
}

fn generate_device_body() -> serde_json::Value {
    let mut rng = rand::thread_rng();
    let open_udid: String = (0..16).map(|_| format!("{:02x}", rng.gen::<u8>())).collect();
    let device_id: String = (0..15).map(|_| rng.gen_range(0..10).to_string()).collect();
    
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
            "openudid": open_udid,
            "cdid": "",
            "sim_serial_number": [],
            "ssmix": "a",
        }
    })
}

fn urlencode(s: &str) -> String {
    s.bytes().map(|b| {
        if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.' || b == b'~' {
            (b as char).to_string()
        } else {
            format!("%{:02X}", b)
        }
    }).collect()
}
