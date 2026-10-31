//! In-memory book library (swap for a database later).

use std::collections::HashMap;

pub type LibraryMap = HashMap<String, serde_json::Value>;
