//! Book search. Tries the optional proxy API first, then the APP search endpoint.

use crate::config::Config;
use serde::{Deserialize, Serialize};

const APP_SEARCH_URL: &str = "https://api5-normal-lf.fqnovel.com/reading/bookapi/search/page/v/";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub book_id: String,
    #[serde(default)]
    pub book_name: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub book_abstract: String,
    #[serde(default)]
    pub thumb_url: String,
    #[serde(default)]
    pub creation_status: String,
}

/// Search through a user-supplied proxy endpoint (`FANQIE_PROXY_API`).
pub async fn search_via_proxy(
    keyword: &str,
    proxy_base: &str,
    http: &reqwest::Client,
) -> anyhow::Result<Vec<SearchResult>> {
    let url = format!(
        "{}/search?key={}&tab_type=3",
        proxy_base.trim_end_matches('/'),
        encode(keyword)
    );
    let resp = http
        .get(&url)
        .header(reqwest::header::ACCEPT, "application/json")
        .send()
        .await?;

    if !resp.status().is_success() {
        anyhow::bail!("proxy search HTTP {}", resp.status());
    }

    let v: serde_json::Value = resp.json().await?;
    let data = v.get("data").unwrap_or(&v);

    // Proxy shapes vary; accept several keys.
    let list = data
        .get("search_book_data_list")
        .or_else(|| data.get("book_list"))
        .or_else(|| data.get("book_ids"))
        .and_then(|x| x.as_array());

    let Some(list) = list else {
        return Ok(Vec::new());
    };

    let mut out = Vec::with_capacity(list.len());
    for item in list {
        if let Some(id) = item.as_str() {
            out.push(SearchResult {
                book_id: id.to_string(),
                book_name: String::new(),
                author: String::new(),
                book_abstract: String::new(),
                thumb_url: String::new(),
                creation_status: String::new(),
            });
            continue;
        }
        let Some(id) = item.get("book_id").and_then(|x| x.as_str()) else {
            continue;
        };
        out.push(SearchResult {
            book_id: id.to_string(),
            book_name: s(item, "book_name"),
            author: s(item, "author"),
            book_abstract: s(item, "book_abstract"),
            thumb_url: s(item, "thumb_url"),
            creation_status: s(item, "creation_status"),
        });
    }
    Ok(out)
}

/// Direct APP search. Note: the upstream endpoint enforces TLS fingerprinting,
/// so this may legitimately return an empty body from a plain HTTP client.
pub async fn search_native(keyword: &str, cfg: &Config) -> anyhow::Result<Vec<SearchResult>> {
    let resp = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(cfg.request_timeout_secs))
        .user_agent(
            "Mozilla/5.0 (Linux; Android 10; P30 Build/HUAWEIP30) \
             AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.6367.179 Mobile Safari/537.36",
        )
        .build()?
        .get(APP_SEARCH_URL)
        .query(&[
            ("query", keyword),
            ("aid", "1967"),
            ("channel", "0"),
            ("os_version", "0"),
            ("device_type", "0"),
            ("device_platform", "0"),
            ("iid", "466614321180296"),
            ("passback", "0"),
            ("version_code", "999"),
        ])
        .header(reqwest::header::ACCEPT, "application/json, text/plain, */*")
        .header(reqwest::header::ACCEPT_LANGUAGE, "zh-CN,zh;q=0.9")
        .header(reqwest::header::REFERER, "https://fanqienovel.com/")
        .send()
        .await?;

    let body = resp.text().await?;
    if body.trim().is_empty() || body.trim() == "{}" {
        anyhow::bail!("search endpoint returned an empty body");
    }

    let v: serde_json::Value = serde_json::from_str(&body)?;
    if v.get("code").and_then(|c| c.as_i64()) != Some(0) {
        anyhow::bail!(
            "search code={}",
            v.get("code").unwrap_or(&serde_json::Value::Null)
        );
    }

    let Some(items) = v.get("data").and_then(|d| d.as_array()) else {
        return Ok(Vec::new());
    };

    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let b = item
            .get("book_data")
            .and_then(|d| d.as_array())
            .and_then(|a| a.first())
            .unwrap_or(item);
        let Some(id) = b.get("book_id").and_then(|x| {
            x.as_str()
                .map(|s| s.to_string())
                .or_else(|| x.as_i64().map(|n| n.to_string()))
        }) else {
            continue;
        };
        if id.is_empty() {
            continue;
        }
        out.push(SearchResult {
            book_id: id,
            book_name: s(b, "book_name"),
            author: s(b, "author"),
            book_abstract: s(b, "book_abstract"),
            thumb_url: s(b, "thumb_url"),
            creation_status: s(b, "creation_status"),
        });
    }
    Ok(out)
}

fn s(v: &serde_json::Value, key: &str) -> String {
    v.get(key)
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string()
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
