use serde::{Deserialize, Serialize};

// SourceType 用来区分原始输入来源。
// Distilllab 第一阶段先只区分普通文档和 AI coding session。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SourceType {
    Document,
    Session,
}

impl SourceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            SourceType::Document => "document",
            SourceType::Session => "session",
        }
    }
}

// SourceRecord 是 Distilllab 接收到的一条原始输入记录。
// 它是后续 chunk、work item、project grouping 的起点。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceRecord {
    // 原始来源的唯一标识。
    pub id: String,

    // 来源类型，决定后续解析和抽取策略。
    pub source_type: SourceType,

    // 用户可读标题，例如文件名或导入标题。
    pub title: String,

    // 创建时间；后续可替换为更严格的时间类型。
    pub created_at: String,
}
