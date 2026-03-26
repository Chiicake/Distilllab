use serde::{Deserialize, Serialize};

// Chunk 表示 Source 被切分后的最小片段。
// 它是后续 WorkItem 提取、证据引用和检索的基础单位。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub id: String,
    pub source_id: String,
    pub sequence: i64,
    pub content: String,
}
