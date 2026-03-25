// Distilllab 的共享数据结构入口。
// 这里定义的类型会被 runtime、memory、desktop 等模块共同使用，
// 用来保证不同模块对同一个业务对象的理解一致。
pub mod run;
pub mod source;

// 对外暴露当前阶段最小的一组核心记录类型。
pub use run::{RunRecord, RunState};
pub use source::{SourceRecord, SourceType};
