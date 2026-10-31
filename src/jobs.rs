//! Download job registry (in-memory placeholder for a real queue).

use std::collections::HashMap;

pub type JobMap = HashMap<String, serde_json::Value>;
