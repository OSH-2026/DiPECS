//! 验证 ActionBus 的事件通道

use aios_core::action_bus::ActionBus;
use aios_spec::*;

fn make_raw_event() -> RawEvent {
    RawEvent::ProcStateChange(ProcStateEvent {
        timestamp_ms: 1000,
        pid: 42,
        uid: 10123,
        package_name: Some("com.test.app".into()),
        vm_rss_kb: 128000,
        vm_swap_kb: 0,
        threads: 8,
        oom_score: 0,
        io_read_mb: 1,
        io_write_mb: 2,
        state: ProcState::Running,
    })
}

fn make_intent_batch() -> IntentBatch {
    IntentBatch {
        window_id: "w1".into(),
        intents: vec![Intent {
            intent_id: "int-1".into(),
            intent_type: IntentType::Idle,
            confidence: 0.5,
            risk_level: RiskLevel::Low,
            suggested_actions: vec![SuggestedAction {
                action_type: ActionType::NoOp,
                target: None,
                urgency: ActionUrgency::IdleTime,
            }],
            rationale_tags: vec![],
        }],
        generated_at_ms: 5000,
        model: "test".into(),
    }
}

#[tokio::test]
async fn test_push_raw_event_succeeds() {
    let bus = ActionBus::new(4);
    let event = make_raw_event();
    let result = bus.push_raw_event(event).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_push_raw_event_received() {
    let mut bus = ActionBus::new(4);
    bus.push_raw_event(make_raw_event()).await.unwrap();

    let received = bus.raw_events_rx.try_recv();
    assert!(received.is_ok());
}

#[tokio::test]
async fn test_push_intent_succeeds() {
    let bus = ActionBus::new(4);
    let batch = make_intent_batch();
    let result = bus.push_intent(batch).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_push_intent_received() {
    let mut bus = ActionBus::new(4);
    bus.push_intent(make_intent_batch()).await.unwrap();

    let received = bus.intent_rx.try_recv();
    assert!(received.is_ok());
    let batch = received.unwrap();
    assert_eq!(batch.model, "test");
    assert_eq!(batch.intents.len(), 1);
}

#[tokio::test]
async fn test_channel_closed_after_rx_dropped() {
    let bus = ActionBus::new(4);
    let tx = bus.raw_events_tx;
    drop(bus.raw_events_rx); // close rx, draining any buffered sends

    let result = tx.send(make_raw_event()).await;
    assert!(result.is_err(), "send should fail after rx dropped");
}

#[tokio::test]
async fn test_intent_channel_closed_after_rx_dropped() {
    let bus = ActionBus::new(4);
    let tx = bus.intent_tx;
    drop(bus.intent_rx);

    let result = tx.send(make_intent_batch()).await;
    assert!(result.is_err());
}

#[test]
fn test_action_bus_default_creates() {
    let bus = ActionBus::default();
    // 验证 sender/receiver 可用
    assert!(!bus.raw_events_tx.is_closed());
    assert!(!bus.intent_tx.is_closed());
}
