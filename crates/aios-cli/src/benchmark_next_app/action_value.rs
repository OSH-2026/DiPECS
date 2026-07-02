//! Offline action-value model for the next-app benchmark.
//!
//! This is not a replacement for emulator/on-device measurements. It is a
//! deterministic backtest layer that forces every predictor to answer the
//! system question: did its Top-k output become useful action value after costs?

use super::metrics::round3;
use super::types::{ActionValueMetrics, NextAppLabel, ScoredPrediction};

const PREWARM_SAVED_MS: f64 = 120.0;
const WASTED_PREWARM_COST_MS: f64 = 12.0;
const CONTROL_PLANE_COST_MS: f64 = 1.5;
const POLICY_BLOCKED_COST_MS: f64 = 0.5;

#[derive(Debug, Clone, Default)]
pub struct ActionValueRecord {
    pub prewarm_attempts: usize,
    pub prewarm_hit: bool,
    pub policy_blocked_actions: usize,
    pub saved_ms: f64,
    pub wasted_cost_ms: f64,
    pub control_plane_cost_ms: f64,
    pub net_benefit_ms: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionValueMode {
    DirectTopK,
    GovernedTopK,
    Oracle,
    NoAction,
}

pub fn evaluate_action_value(
    mode: ActionValueMode,
    label: &NextAppLabel,
    ranked: &[ScoredPrediction],
    top_k: usize,
) -> ActionValueRecord {
    match mode {
        ActionValueMode::NoAction => ActionValueRecord::default(),
        ActionValueMode::Oracle => oracle_record(label),
        ActionValueMode::DirectTopK => direct_record(label, ranked, top_k, 0),
        ActionValueMode::GovernedTopK => governed_record(label, ranked, top_k),
    }
}

pub fn compute_action_value_metrics(records: &[ActionValueRecord]) -> ActionValueMetrics {
    let prewarm_attempts: usize = records.iter().map(|r| r.prewarm_attempts).sum();
    let prewarm_hits = records.iter().filter(|r| r.prewarm_hit).count();
    let wasted_prewarm = prewarm_attempts.saturating_sub(prewarm_hits);
    let policy_blocked_actions: usize = records.iter().map(|r| r.policy_blocked_actions).sum();
    let saved_ms: f64 = records.iter().map(|r| r.saved_ms).sum();
    let wasted_cost_ms: f64 = records.iter().map(|r| r.wasted_cost_ms).sum();
    let control_plane_cost_ms: f64 = records.iter().map(|r| r.control_plane_cost_ms).sum();
    let net_benefit_ms: f64 = records.iter().map(|r| r.net_benefit_ms).sum();

    ActionValueMetrics {
        prewarm_attempts,
        prewarm_hits,
        wasted_prewarm,
        prewarm_hit_rate_pct: pct(prewarm_hits, prewarm_attempts),
        wasted_prewarm_rate_pct: pct(wasted_prewarm, prewarm_attempts),
        policy_blocked_actions,
        saved_latency_ms: round3(saved_ms),
        wasted_action_cost_ms: round3(wasted_cost_ms),
        control_plane_cost_ms: round3(control_plane_cost_ms),
        net_benefit_ms: round3(net_benefit_ms),
    }
}

fn oracle_record(label: &NextAppLabel) -> ActionValueRecord {
    if label.actual_next_app.is_none() {
        return ActionValueRecord::default();
    }
    let saved_ms = PREWARM_SAVED_MS;
    ActionValueRecord {
        prewarm_attempts: 1,
        prewarm_hit: true,
        policy_blocked_actions: 0,
        saved_ms,
        wasted_cost_ms: 0.0,
        control_plane_cost_ms: 0.0,
        net_benefit_ms: saved_ms,
    }
}

fn governed_record(
    label: &NextAppLabel,
    ranked: &[ScoredPrediction],
    top_k: usize,
) -> ActionValueRecord {
    let considered = ranked.len().min(top_k);
    if considered == 0 {
        return ActionValueRecord::default();
    }
    let budget = 1;
    let blocked = considered.saturating_sub(budget);
    direct_record(label, &ranked[..budget], budget, blocked)
}

fn direct_record(
    label: &NextAppLabel,
    ranked: &[ScoredPrediction],
    top_k: usize,
    policy_blocked_actions: usize,
) -> ActionValueRecord {
    let attempted = ranked.len().min(top_k);
    if attempted == 0 {
        return ActionValueRecord::default();
    }

    let actual = label.actual_next_app.as_deref();
    let hit = actual.is_some_and(|actual| {
        ranked
            .iter()
            .take(top_k)
            .any(|prediction| prediction.package == actual)
    });
    let saved_ms = if hit { PREWARM_SAVED_MS } else { 0.0 };
    let wasted = attempted.saturating_sub(usize::from(hit));
    let wasted_cost_ms = wasted as f64 * WASTED_PREWARM_COST_MS
        + policy_blocked_actions as f64 * POLICY_BLOCKED_COST_MS;
    let control_plane_cost_ms = CONTROL_PLANE_COST_MS;

    ActionValueRecord {
        prewarm_attempts: attempted,
        prewarm_hit: hit,
        policy_blocked_actions,
        saved_ms,
        wasted_cost_ms,
        control_plane_cost_ms,
        net_benefit_ms: saved_ms - wasted_cost_ms - control_plane_cost_ms,
    }
}

fn pct(part: usize, whole: usize) -> f64 {
    if whole == 0 {
        0.0
    } else {
        round3((part as f64 / whole as f64) * 100.0)
    }
}
