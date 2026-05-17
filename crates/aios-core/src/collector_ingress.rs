use aios_spec::{CollectorEnvelope, IngestedRawEvent, RawEvent, SourceTier};
use thiserror::Error;

const SUPPORTED_COLLECTOR_SCHEMA: &str = "dipecs.collector.v1";

/// Rust 侧 collector 入口。
///
/// 所有进入 core 管线的 `RawEvent` 都必须通过此入口，无论来自
/// apps 侧 (JSONL/JNI/socket) 还是 Rust 系统采集器。`SourceTier`
/// 由入口权威决定，并随 `IngestedRawEvent` 一路传递到脱敏器。
#[derive(Debug, Default)]
pub struct RustCollectorIngress;

impl RustCollectorIngress {
    /// 校验并解包来自 apps 侧或外部系统的 envelope。
    ///
    /// envelope 中声明的 `source_tier` 会随事件一并返回，供下游脱敏器
    /// 与策略层使用。
    pub fn accept(
        &self,
        envelope: CollectorEnvelope,
    ) -> Result<IngestedRawEvent, CollectorIngressError> {
        if envelope.schema_version != SUPPORTED_COLLECTOR_SCHEMA {
            return Err(CollectorIngressError::UnsupportedSchemaVersion(
                envelope.schema_version,
            ));
        }
        Ok(IngestedRawEvent {
            raw_event: envelope.raw_event,
            source_tier: envelope.source_tier,
        })
    }

    /// 包装来自 Rust 系统采集器的事件。
    ///
    /// 内部采集器是受信来源，固定标记为 `SourceTier::Daemon`。
    /// `source` 和 `captured_at_ms` 当前仅用于调用方诊断，
    /// 未来可在 envelope 化改造后接入 Trace。
    pub fn accept_internal(
        &self,
        raw: RawEvent,
        _source: &str,
        _captured_at_ms: i64,
    ) -> IngestedRawEvent {
        IngestedRawEvent {
            raw_event: raw,
            source_tier: SourceTier::Daemon,
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CollectorIngressError {
    #[error("unsupported collector schema version: {0}")]
    UnsupportedSchemaVersion(String),
}
