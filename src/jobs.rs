//! Download job tracking (replace with persistent queue).

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadJob {
    pub id: String,
    pub book_id: String,
    pub format: String,
    pub status: String,
    pub created_at: u64,
}

impl DownloadJob {
    pub fn new(book_id: String, format: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            book_id,
            format,
            status: "queued".into(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }
}

pub type Jobs = HashMap<String, DownloadJob>;
