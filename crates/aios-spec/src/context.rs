//! 上下文窗口 — "what to send to the LLM"
//!
//! 脱敏后的 SanitizedEvent 按时间窗口聚合,
//! 形成发送给 Cloud LLM 的结构化上下文。
//! 这是 DiPECS daemon 向上的核心接口。

use serde::{Deserialize, Serialize};

use crate::event::{
    ExtensionCategory, LocationType, NetworkType, RingerMode, SanitizedEvent, SemanticHint,
    SourceTier,
};

/// 时间窗口内的脱敏上下文
///
/// 这是 aios-agent 发送给 Cloud LLM 的唯一数据格式。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredContext {
    /// 窗口唯一 ID
    pub window_id: String,
    /// 窗口起始时间 (epoch ms)
    pub window_start_ms: i64,
    /// 窗口结束时间 (epoch ms)
    pub window_end_ms: i64,
    /// 窗口持续的秒数
    pub duration_secs: u32,
    /// 窗口内的事件序列 (按时间排序, 已脱敏)
    pub events: Vec<SanitizedEvent>,
    /// 窗口聚合摘要 (帮助 LLM 快速理解)
    pub summary: ContextSummary,
}

/// 窗口聚合摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSummary {
    /// 窗口内的前台 app 序列 (按时间顺序, 去重)
    pub foreground_apps: Vec<String>,
    /// 收到通知的 app 列表 (去重)
    pub notified_apps: Vec<String>,
    /// 触发的语义标签汇总 (去重)
    pub all_semantic_hints: Vec<SemanticHint>,
    /// 文件活动汇总 (扩展名类别 → 次数)
    pub file_activity: Vec<(ExtensionCategory, u32)>,
    /// 系统状态快照 (取窗口内的最新值)
    pub latest_system_status: Option<SystemStatusSnapshot>,
    /// 来源能力等级
    pub source_tier: SourceTier,
}

/// 系统状态快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatusSnapshot {
    pub battery_pct: Option<u8>,
    pub is_charging: bool,
    pub network: NetworkType,
    pub ringer_mode: RingerMode,
    pub location_type: LocationType,
    pub headphone_connected: bool,
}
