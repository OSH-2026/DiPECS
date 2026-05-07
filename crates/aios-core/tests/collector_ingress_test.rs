use aios_core::collector_ingress::{CollectorIngressError, RustCollectorIngress};
use aios_spec::*;

fn make_envelope(schema_version: &str) -> CollectorEnvelope {
    CollectorEnvelope {
        schema_version: schema_version.into(),
        source: "apps.android-collector.device".into(),
        source_tier: SourceTier::PublicApi,
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
fn ingress_accepts_supported_envelope_and_returns_raw_event() {
    let raw = RustCollectorIngress
        .accept(make_envelope("dipecs.collector.v1"))
        .unwrap();

    assert!(matches!(raw, RawEvent::ScreenState(_)));
}

#[test]
fn ingress_rejects_unsupported_schema_version() {
    let err = RustCollectorIngress
        .accept(make_envelope("unknown.v9"))
        .unwrap_err();

    assert!(matches!(
        err,
        CollectorIngressError::UnsupportedSchemaVersion(_)
    ));
}
