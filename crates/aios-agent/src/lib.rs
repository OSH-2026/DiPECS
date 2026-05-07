//! # aios-agent — 决策路由与模型后端
//!
//! 接收 `StructuredContext`, 选择本地规则/本地模型/云端模型等后端,
//! 并返回 `IntentBatch` 供 core 做最终审查。
//!
//! ## 模块结构
//!
//! - `DecisionBackend` trait — 统一的后端接口
//! - `router` — DecisionRouter, RouterConfig, CircuitState, RoutingReason
//! - `backends::rule_based` — 规则驱动的意图生成
//! - `backends::fallback` — circuit breaker 熔断后的最终安全后端

mod backends;
mod router;

pub use backends::fallback::FallbackNoOpBackend;
pub use backends::rule_based::RuleBasedBackend;
pub use router::{DecisionRouter, RouterConfig};

use aios_spec::{DecisionBackendResult, StructuredContext};
use uuid::Uuid;

// ============================================================
// DecisionBackend trait
// ============================================================

/// 统一的后端接口 — 接收 Context, 返回决策结果。
///
/// 所有后端（规则引擎、本地模型、云端 LLM、fallback）都实现此 trait。
pub trait DecisionBackend {
    fn evaluate(&self, context: &StructuredContext) -> DecisionBackendResult;
}

// ============================================================
// Helpers
// ============================================================

fn new_id() -> String {
    Uuid::new_v4().to_string()
}
