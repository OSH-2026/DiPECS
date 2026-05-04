//! 意图与动作 — "what the LLM tells us to do"
//!
//! Cloud LLM 返回的结构化决策, 从 agent 流向 core。

use serde::{Deserialize, Serialize};

use crate::event::ExtensionCategory;

/// 云端 LLM 返回的结构化决策
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentBatch {
    /// 请求对应的窗口 ID
    pub window_id: String,
    /// 候选意图列表 (按置信度降序)
    pub intents: Vec<Intent>,
    /// 生成时间 (epoch ms)
    pub generated_at_ms: i64,
    /// 模型标识
    pub model: String,
}

/// 单条意图
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intent {
    /// 意图唯一 ID
    pub intent_id: String,
    /// 意图类型
    pub intent_type: IntentType,
    /// 置信度 (0.0 ~ 1.0)
    pub confidence: f32,
    /// 风险等级
    pub risk_level: RiskLevel,
    /// 该意图的推荐动作列表
    pub suggested_actions: Vec<SuggestedAction>,
    /// LLM 给出的理由标签 (简短, 不用自然语言)
    pub rationale_tags: Vec<String>,
}

/// 意图类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IntentType {
    /// 用户将打开某个 app
    OpenApp(String),
    /// 用户将切换到某个 app
    SwitchToApp(String),
    /// 用户将查看某条通知
    CheckNotification(String),
    /// 用户将处理某类文件
    HandleFile(ExtensionCategory),
    /// 用户即将进入某个物理场景
    EnterContext(String),
    /// 无明确意图, 保持观察
    Idle,
}

/// 风险等级
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum RiskLevel {
    /// 可自动执行
    Low,
    /// 需要策略引擎二次确认后执行
    Medium,
    /// 仅建议, 不自动执行
    High,
}

/// 推荐动作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestedAction {
    pub action_type: ActionType,
    /// 目标标识 (app package 或其他)
    pub target: Option<String>,
    /// 紧迫度
    pub urgency: ActionUrgency,
}

/// 动作类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionType {
    /// 预热应用进程
    PreWarmProcess,
    /// 预加载热点文件到页缓存
    PrefetchFile,
    /// 保活当前前台进程
    KeepAlive,
    /// 释放指定进程的非关键内存
    ReleaseMemory,
    /// 不执行任何操作
    NoOp,
}

/// 动作紧迫度
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ActionUrgency {
    /// 立即执行
    Immediate,
    /// 空闲时执行
    IdleTime,
    /// 延迟执行
    Deferred,
}
