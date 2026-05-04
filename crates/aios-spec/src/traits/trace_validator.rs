use crate::trace::{GoldenTrace, ReplayResult};

/// Trace 验证器
///
/// 给定相同的 RawEvent 输入序列, 验证:
/// 1. 脱敏输出是否逐条一致 (PrivacyAirGap 的确定性)
/// 2. 策略引擎的决策是否一致 (PolicyEngine 的确定性)
pub trait TraceValidator {
    /// 对比 Golden Trace, 返回验证结果
    fn validate(&self, golden: &GoldenTrace) -> ReplayResult;
}
