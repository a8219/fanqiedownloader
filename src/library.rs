//! In-memory library (replace with SQLite for persistence).

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryBook {
    pub book_id: String,
    #[serde(default)]
    pub book_name: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub cover: String,
}

pub type Library = HashMap<String, LibraryBook>;
