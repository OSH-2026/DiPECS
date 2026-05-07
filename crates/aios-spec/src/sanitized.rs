//! 脱敏后事件类型 — 不含 PII，可在系统内自由传输。
//!
//! `SanitizedEvent` 是 `PrivacyAirGap` 的输出，是 DiPECS 数据模型的核心。

use serde::{Deserialize, Serialize};

use crate::event::{
    AppTransition, ExtensionCategory, FsActivityType, InteractionType, LocationType, NetworkType,
    RingerMode, ScreenState, SemanticHint, SourceTier, TextHint,
};

/// 脱敏后的事件。不再包含任何 PII。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SanitizedEvent {
    pub event_id: String,
    pub timestamp_ms: i64,
    pub event_type: SanitizedEventType,
    pub source_tier: SourceTier,
    pub app_package: Option<String>,
    pub uid: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SanitizedEventType {
    /// 应用前后台切换 (来自 UsageStatsManager)
    AppTransition {
        package_name: String,
        activity_class: Option<String>,
        transition: AppTransition,
    },
    /// 应用间交互 (从 Binder 事务推断)
    InterAppInteraction {
        source_package: Option<String>,
        target_service: String,
        interaction_type: InteractionType,
    },
    /// 通知 (脱敏后)
    Notification {
        source_package: String,
        category: Option<String>,
        channel_id: Option<String>,
        title_hint: TextHint,
        text_hint: TextHint,
        semantic_hints: Vec<SemanticHint>,
        is_ongoing: bool,
        group_key: Option<String>,
    },
    /// 进程资源状态
    ProcessResource {
        pid: u32,
        package_name: Option<String>,
        vm_rss_mb: u32,
        vm_swap_mb: u32,
        thread_count: u32,
        oom_score: i32,
    },
    /// 文件系统活动
    FileActivity {
        package_name: Option<String>,
        extension_category: ExtensionCategory,
        activity_type: FsActivityType,
        is_hot_file: bool,
    },
    /// 屏幕状态
    Screen { state: ScreenState },
    /// 系统状态快照
    SystemStatus {
        battery_pct: Option<u8>,
        is_charging: bool,
        network: NetworkType,
        ringer_mode: RingerMode,
        location_type: LocationType,
        headphone_connected: bool,
    },
}
