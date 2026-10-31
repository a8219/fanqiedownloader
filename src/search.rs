//! Native search implementation using the Tomato APP search API.
//! Self-contained: no proxy needed. Uses reqwest with full Chrome-like headers.

use crate::config::Config;
use serde::{Deserialize, Serialize};

const SEARCH_URL: &str = "https://api5-normal-lf.fqnovel.com/reading/bookapi/search/page/v/";

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub book_id: String,
    pub book_name: String,
    pub author: String,
    #[serde(default)]
    pub book_abstract: String,
    #[serde(default)]
    pub thumb_url: String,
    #[serde(default)]
    pub creation_status: String,
}

pub async fn search(keyword: &str, config: &Config) -> anyhow::Result<Vec<SearchResult>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(config.request_timeout_secs))
        .user_agent("Mozilla/5.0 (Linux; Android 10; P30 Build/HUAWEIP30) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.6367.179 Mobile Safari/537.36")
        .build()?;

    let params = [
        ("query", keyword),
        ("aid", "1967"),
        ("channel", "0"),
        ("os_version", "0"),
        ("device_type", "0"),
        ("device_platform", "0"),
        ("iid", "466614321180296"),
        ("passback", "0"),
        ("version_code", "999"),
    ];

    let res = client
        .get(SEARCH_URL)
        .query(&params)
        .header("Accept", "application/json, text/plain, */*")
        .header("Accept-Language", "zh-CN,zh;q=0.9")
        .header("Referer", "https://fanqienovel.com/")
        .send()
        .await?;

    if !res.status().is_success() {
        anyhow::bail!("Search API returned HTTP {}", res.status());
    }

    let body = res.text().await?;
    if body.trim().is_empty() || body.trim() == "{}" {
        anyhow::bail!("Search API returned empty response");
    }

    let json: serde_json::Value = serde_json::from_str(&body)?;
    if json["code"].as_i64() != Some(0) {
        anyhow::bail!("Search API error: code={}, message={}", json["code"], json.get("message").and_then(|m| m.as_str()).unwrap_or(""));
    }

    let data = json["data"].as_array()
        .ok_or_else(|| anyhow::anyhow!("Search response data is not an array"))?;

    let results: Vec<SearchResult> = data.iter().filter_map(|item| {
        let b = item.get("book_data").and_then(|d| d.as_array()).and_then(|a| a.first()).unwrap_or(item);
        let book_id = b.get("book_id").and_then(|v| v.as_str()).or_else(|| b.get("book_id").and_then(|v| v.as_i64()).map(|i| i.to_string()))?;
        if book_id.is_empty() { return None; }
        Some(SearchResult {
            book_id,
            book_name: b.get("book_name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            author: b.get("author").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            book_abstract: b.get("book_abstract").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            thumb_url: b.get("thumb_url").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            creation_status: b.get("creation_status").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        })
    }).collect();

    Ok(results)
}
