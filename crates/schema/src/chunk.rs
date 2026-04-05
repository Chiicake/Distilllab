use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub id: String,
    pub source_id: String,
    pub sequence: i64,
    pub title: String,
    pub summary: String,
    pub content: String,
}
