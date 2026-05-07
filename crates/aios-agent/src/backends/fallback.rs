//! FallbackNoOpBackend — circuit breaker 熔断后的最终安全后端。
//!
//! 返回单个 Idle/NoOp 意图，confidence 为 0.0，并附加 error 信息。

use std::time::Instant;

use aios_spec::{
    ActionType, ActionUrgency, DecisionBackendResult, DecisionRoute, Intent, IntentBatch,
    IntentType, RiskLevel, StructuredContext, SuggestedAction,
};

use crate::{new_id, DecisionBackend};

pub struct FallbackNoOpBackend;

impl DecisionBackend for FallbackNoOpBackend {
    fn evaluate(&self, context: &StructuredContext) -> DecisionBackendResult {
        let start = Instant::now();
        let intent_batch = IntentBatch {
            window_id: context.window_id.clone(),
            intents: vec![Intent {
                intent_id: new_id(),
                intent_type: IntentType::Idle,
                confidence: 0.0,
                risk_level: RiskLevel::Low,
                suggested_actions: vec![SuggestedAction {
                    action_type: ActionType::NoOp,
                    target: None,
                    urgency: ActionUrgency::IdleTime,
                }],
                rationale_tags: vec!["fallback_noop".into()],
            }],
            generated_at_ms: context.window_end_ms,
            model: "fallback-noop-v0.1".to_string(),
        };

        DecisionBackendResult {
            route: DecisionRoute::FallbackNoOp,
            intent_batch,
            rationale_tags: vec!["fallback_noop".into()],
            latency_us: start.elapsed().as_micros() as u64,
            error: Some("circuit breaker engaged — falling back to no-op".into()),
        }
    }
}
