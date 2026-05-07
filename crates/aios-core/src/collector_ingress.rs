use aios_spec::{CollectorEnvelope, RawEvent, SourceTier};
use thiserror::Error;

const SUPPORTED_COLLECTOR_SCHEMA: &str = "dipecs.collector.v1";

/// Rust 侧 collector 入口。
///
/// 所有进入 core 管线的 `RawEvent` 都必须通过此入口，无论来自
/// apps 侧 (JSONL/JNI/socket) 还是 Rust 系统采集器。
#[derive(Debug, Default)]
pub struct RustCollectorIngress;

impl RustCollectorIngress {
    /// 校验并解包来自 apps 侧或外部系统的 envelope。
    pub fn accept(&self, envelope: CollectorEnvelope) -> Result<RawEvent, CollectorIngressError> {
        if envelope.schema_version != SUPPORTED_COLLECTOR_SCHEMA {
            return Err(CollectorIngressError::UnsupportedSchemaVersion(
                envelope.schema_version,
            ));
        }
        Ok(envelope.raw_event)
    }

    /// 包装并接入来自 Rust 系统采集器的事件。
    ///
    /// 内部采集器是受信来源，自动填充当前 schema 版本、
    /// `SourceTier::Daemon` 和设备侧 trace id。
    pub fn accept_internal(&self, raw: RawEvent, source: &str, captured_at_ms: i64) -> RawEvent {
        let _envelope = CollectorEnvelope {
            schema_version: SUPPORTED_COLLECTOR_SCHEMA.into(),
            source: source.into(),
            source_tier: SourceTier::Daemon,
            device_trace_id: None,
            captured_at_ms,
            received_at_ms: None,
            raw_event: raw,
        };
        // 内部事件自动通过 schema 校验，直接返回 RawEvent。
        // envelope 元信息未来可用于 Trace，当前不进核心管线。
        _envelope.raw_event
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CollectorIngressError {
    #[error("unsupported collector schema version: {0}")]
    UnsupportedSchemaVersion(String),
}
