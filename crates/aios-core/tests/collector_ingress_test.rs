use aios_core::collector_ingress::{CollectorIngressError, RustCollectorIngress};
use aios_spec::*;

fn make_envelope(schema_version: &str, tier: SourceTier) -> CollectorEnvelope {
    CollectorEnvelope {
        schema_version: schema_version.into(),
        source: "apps.android-collector.device".into(),
        source_tier: tier,
        device_trace_id: Some("trace-1".into()),
        captured_at_ms: 1000,
        received_at_ms: None,
        raw_event: RawEvent::ScreenState(ScreenStateEvent {
            timestamp_ms: 1000,
            state: ScreenState::Interactive,
        }),
    }
}

#[test]
fn ingress_accepts_supported_envelope_and_returns_ingested_event() {
    let ingested = RustCollectorIngress
        .accept(make_envelope("dipecs.collector.v1", SourceTier::PublicApi))
        .unwrap();

    assert!(matches!(ingested.raw_event, RawEvent::ScreenState(_)));
    assert_eq!(ingested.source_tier, SourceTier::PublicApi);
}

#[test]
fn ingress_preserves_envelope_source_tier() {
    let ingested = RustCollectorIngress
        .accept(make_envelope("dipecs.collector.v1", SourceTier::Daemon))
        .unwrap();

    assert_eq!(ingested.source_tier, SourceTier::Daemon);
}

#[test]
fn ingress_internal_path_marks_source_tier_as_daemon() {
    let raw = RawEvent::ScreenState(ScreenStateEvent {
        timestamp_ms: 1000,
        state: ScreenState::Interactive,
    });
    let ingested = RustCollectorIngress.accept_internal(raw, "ProcReader", 1000);

    assert_eq!(ingested.source_tier, SourceTier::Daemon);
}

#[test]
fn ingress_rejects_unsupported_schema_version() {
    let err = RustCollectorIngress
        .accept(make_envelope("unknown.v9", SourceTier::PublicApi))
        .unwrap_err();

    assert!(matches!(
        err,
        CollectorIngressError::UnsupportedSchemaVersion(_)
    ));
}
