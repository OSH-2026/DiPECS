//! Trace 引擎 — 确定性回放验证
//!
//! 记录 `GoldenTrace` 并在回放时验证脱敏和策略的确定性。

use aios_spec::traits::TraceValidator;
use aios_spec::{GoldenTrace, ReplayResult, SanitizedEvent};
use aios_spec::traits::PrivacySanitizer;

/// 默认 Trace 引擎
pub struct DefaultTraceEngine {
    /// 用于在回放时重新脱敏的 sanitizer
    sanitizer: Box<dyn PrivacySanitizer + Send + Sync>,
}

impl DefaultTraceEngine {
    /// 创建 Trace 引擎
    pub fn new(sanitizer: impl PrivacySanitizer + Send + Sync + 'static) -> Self {
        Self { sanitizer: Box::new(sanitizer) }
    }
}

impl TraceValidator for DefaultTraceEngine {
    fn validate(&self, golden: &GoldenTrace) -> ReplayResult {
        // 1. 逐条重新脱敏, 检查是否与期望输出一致
        let mut sanitization_divergences = Vec::new();
        let actual_sanitized: Vec<SanitizedEvent> = golden
            .raw_events
            .iter()
            .map(|raw| self.sanitizer.sanitize(raw.clone()))
            .collect();

        for (i, (actual, expected)) in actual_sanitized.iter()
            .zip(golden.expected_sanitized.iter())
            .enumerate()
        {
            if !sanitized_eq(actual, expected) {
                sanitization_divergences.push(i);
            }
        }

        // 2. 策略差异检测 (占位 — 实际实现需要在回放时调用 PolicyEngine)
        let policy_divergences = Vec::new();

        ReplayResult {
            trace_id: golden.trace_id.clone(),
            sanitization_match: sanitization_divergences.is_empty(),
            sanitization_divergences,
            policy_match: policy_divergences.is_empty(),
            policy_divergences,
        }
    }
}

/// 比较两个 SanitizedEvent 是否一致
///
/// 注意: event_id 和 timestamp_ms 不在比较范围内,
/// 因为它们可能因生成时间不同而变化。
/// 只比较语义内容 (event_type, source_tier, app_package, uid)。
fn sanitized_eq(a: &SanitizedEvent, b: &SanitizedEvent) -> bool {
    // 序列化后比较是确定性的 (serde_json 保证 key 顺序)
    // 但需要先排除 event_id
    let a_json = serde_json::to_string(&a.event_type).unwrap_or_default();
    let b_json = serde_json::to_string(&b.event_type).unwrap_or_default();
    a_json == b_json
        && a.source_tier == b.source_tier
        && a.app_package == b.app_package
        && a.uid == b.uid
}
