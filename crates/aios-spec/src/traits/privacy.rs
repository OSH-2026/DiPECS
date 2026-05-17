use crate::event::{RawEvent, SourceTier};
use crate::sanitized::SanitizedEvent;

/// 隐私脱敏引擎
///
/// 所有 RawEvent 在此处被转化为 SanitizedEvent。
/// 原始数据 (通知正文、文件名、Binder 参数) 在此边界之后不可访问。
pub trait PrivacySanitizer {
    /// 对单个原始事件进行脱敏 (使用类型默认的 SourceTier)
    fn sanitize(&self, raw: RawEvent) -> SanitizedEvent;

    /// 对单个原始事件进行脱敏，并以入口声明的 `source_tier` 覆盖结果。
    ///
    /// Ingress 边界 (Android envelope / Rust 内部采集) 应当通过此方法
    /// 调用脱敏器，使得能力等级在管线后段保持权威。
    fn sanitize_with_tier(&self, raw: RawEvent, source_tier: SourceTier) -> SanitizedEvent {
        let mut event = self.sanitize(raw);
        event.source_tier = source_tier;
        event
    }

    /// 批量脱敏
    fn sanitize_batch(&self, raw_events: Vec<RawEvent>) -> Vec<SanitizedEvent> {
        raw_events.into_iter().map(|e| self.sanitize(e)).collect()
    }
}
