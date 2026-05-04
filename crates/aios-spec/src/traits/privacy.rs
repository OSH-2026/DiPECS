use crate::event::{RawEvent, SanitizedEvent};

/// 隐私脱敏引擎
///
/// 所有 RawEvent 在此处被转化为 SanitizedEvent。
/// 原始数据 (通知正文、文件名、Binder 参数) 在此边界之后不可访问。
pub trait PrivacySanitizer {
    /// 对单个原始事件进行脱敏
    fn sanitize(&self, raw: RawEvent) -> SanitizedEvent;

    /// 批量脱敏
    fn sanitize_batch(&self, raw_events: Vec<RawEvent>) -> Vec<SanitizedEvent> {
        raw_events.into_iter().map(|e| self.sanitize(e)).collect()
    }
}
