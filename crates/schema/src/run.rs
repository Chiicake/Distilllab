use serde::{Deserialize, Serialize};

// RunState 表示一次运行任务当前所处的生命周期阶段。
// Phase 0 先只保留最小状态集合，后续再扩展为 waiting_for_user 等更细状态。
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum RunState {
    Pending,
    Running,
    Completed,
    Failed,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum RunType{
    Demo,
    ImportAndDistill,
    Deepening,
    ComposeAndVerify,
}

// RunRecord 是 Distilllab 中“运行一次任务”的最小记录。
// 后续 import、inquiry、compose 等流程都会先落成 run，再逐步扩展 trace 和 step 信息。
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RunRecord {
    // 运行实例的唯一标识。
    pub id: String,

    // 运行类型，例如 ImportAndDistillRun、DeepeningRun。
    pub run_type: RunType,

    // 当前运行状态。
    pub status: RunState,

    // 创建时间；后续可替换为更严格的时间类型。
    pub created_at: String,
}
