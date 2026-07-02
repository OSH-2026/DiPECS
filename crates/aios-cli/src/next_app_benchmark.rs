//! Synthetic next-app benchmark utilities.
//!
//! This module is intentionally library-first so integration tests can rerun the
//! published benchmark without depending on a CLI subcommand.

use std::collections::{BTreeMap, HashMap};
use std::io::{BufRead, BufReader, Read};

use aios_agent::{DecisionBackend, LocalEvaluatorBackend, RuleBasedBackend};
use aios_core::collector_ingress::RustCollectorIngress;
use aios_core::privacy_airgap::DefaultPrivacyAirGap;
use aios_spec::traits::PrivacySanitizer;
use aios_spec::{
    AppTransition, CollectorEnvelope, ContextSummary, DecisionBackendResult, ExtensionCategory,
    IntentType, RawEvent, SanitizedEvent, SanitizedEventType, SemanticHint, SourceTier,
    StructuredContext, SystemStatusSnapshot,
};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

const SCHEMA_VERSION: &str = "dipecs.collector.v1";

#[derive(Debug, Clone, Deserialize)]
pub struct NextAppLabel {
    pub dataset_id: String,
    pub scenario: String,
    pub window_start_ms: i64,
    pub window_end_ms: i64,
    pub prediction_horizon_ms: i64,
    pub context_event_count: usize,
    pub current_app: Option<String>,
    pub observable_candidates: Vec<String>,
    pub actual_next_app: Option<String>,
    pub actual_next_app_at_ms: Option<i64>,
    pub source: String,
    pub eligible: bool,
    pub excluded_reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ScenarioInput {
    pub scenario: String,
    pub jsonl: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct BackendMetrics {
    pub eligible_windows: u64,
    pub predicted_windows: u64,
    pub top1_hits: u64,
    pub top3_hits: u64,
    pub top1_accuracy_pct: f64,
    pub top3_accuracy_pct: f64,
    pub prediction_coverage_pct: f64,
    pub conditional_top1_accuracy_pct: f64,
    pub wrong_prediction_rate_pct: f64,
    pub no_prediction_rate_pct: f64,
    pub mean_reciprocal_rank: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct ScenarioMetrics {
    pub scenario: String,
    pub windows: u64,
    pub eligible_windows: u64,
    pub rule_based: BackendMetrics,
    pub local_evaluator: BackendMetrics,
    pub markov_order1: BackendMetrics,
    pub markov_order2_backoff: BackendMetrics,
    pub notification_conditioned: BackendMetrics,
    pub naive_bayes_context: BackendMetrics,
    pub hybrid_markov_local: BackendMetrics,
    pub enhanced_local: BackendMetrics,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct BenchmarkReport {
    pub total_windows: u64,
    pub eligible_windows: u64,
    pub rule_based: BackendMetrics,
    pub local_evaluator: BackendMetrics,
    pub markov_order1: BackendMetrics,
    pub markov_order2_backoff: BackendMetrics,
    pub notification_conditioned: BackendMetrics,
    pub naive_bayes_context: BackendMetrics,
    pub hybrid_markov_local: BackendMetrics,
    pub enhanced_local: BackendMetrics,
    pub scenarios: Vec<ScenarioMetrics>,
}

#[derive(Debug, Clone)]
struct TraceEvent {
    event_id: String,
    line_no: usize,
    timestamp_ms: i64,
    raw_event: RawEvent,
}

#[derive(Debug, Clone)]
struct Candidate {
    package: String,
    confidence: f32,
    ordinal: usize,
}

struct LabelContext<'a> {
    label: &'a NextAppLabel,
    context: StructuredContext,
}

#[derive(Debug, Default)]
struct MetricsAccumulator {
    eligible_windows: u64,
    predicted_windows: u64,
    top1_hits: u64,
    top3_hits: u64,
    reciprocal_rank_sum: f64,
}

impl MetricsAccumulator {
    fn record(&mut self, actual: &str, candidates: &[String]) {
        self.eligible_windows += 1;
        if candidates.is_empty() {
            return;
        }

        self.predicted_windows += 1;
        if candidates.first().is_some_and(|pkg| pkg == actual) {
            self.top1_hits += 1;
        }
        if let Some(index) = candidates.iter().position(|pkg| pkg == actual) {
            if index < 3 {
                self.top3_hits += 1;
            }
            self.reciprocal_rank_sum += 1.0 / (index + 1) as f64;
        }
    }

    fn finish(self) -> BackendMetrics {
        let eligible = self.eligible_windows as f64;
        let predicted = self.predicted_windows as f64;
        let wrong = self.predicted_windows.saturating_sub(self.top1_hits) as f64;

        BackendMetrics {
            eligible_windows: self.eligible_windows,
            predicted_windows: self.predicted_windows,
            top1_hits: self.top1_hits,
            top3_hits: self.top3_hits,
            top1_accuracy_pct: pct(self.top1_hits as f64, eligible),
            top3_accuracy_pct: pct(self.top3_hits as f64, eligible),
            prediction_coverage_pct: pct(predicted, eligible),
            conditional_top1_accuracy_pct: pct(self.top1_hits as f64, predicted),
            wrong_prediction_rate_pct: pct(wrong, predicted),
            no_prediction_rate_pct: pct(eligible - predicted, eligible),
            mean_reciprocal_rank: round3(if eligible == 0.0 {
                0.0
            } else {
                self.reciprocal_rank_sum / eligible
            }),
        }
    }
}

pub fn parse_labels<R: Read>(reader: R) -> Result<Vec<NextAppLabel>> {
    BufReader::new(reader)
        .lines()
        .enumerate()
        .filter_map(|(line_idx, line)| match line {
            Ok(line) if line.trim().is_empty() => None,
            other => Some((line_idx + 1, other)),
        })
        .map(|(line_no, line)| {
            let line = line.with_context(|| format!("read label line {line_no}"))?;
            serde_json::from_str(&line).with_context(|| format!("parse label line {line_no}"))
        })
        .collect()
}

pub fn run_benchmark(inputs: &[ScenarioInput], labels: &[NextAppLabel]) -> Result<BenchmarkReport> {
    let labels_by_scenario = labels.iter().fold(
        BTreeMap::<String, Vec<&NextAppLabel>>::new(),
        |mut grouped, label| {
            grouped
                .entry(label.scenario.clone())
                .or_default()
                .push(label);
            grouped
        },
    );
    let input_by_scenario: HashMap<&str, &str> = inputs
        .iter()
        .map(|input| (input.scenario.as_str(), input.jsonl.as_str()))
        .collect();

    let mut report = BenchmarkReport::default();

    for (scenario, scenario_labels) in labels_by_scenario {
        let mut scenario_labels = scenario_labels;
        scenario_labels.sort_by_key(|label| (label.window_end_ms, label.window_start_ms));
        let jsonl = input_by_scenario
            .get(scenario.as_str())
            .with_context(|| format!("missing trace input for scenario {scenario}"))?;
        let trace = parse_trace(jsonl.as_bytes())
            .with_context(|| format!("parse trace for scenario {scenario}"))?;
        let markov = MarkovTransitionPredictor::from_trace(&trace);
        let notification_predictor = NotificationConditionedPredictor::from_trace(&trace);

        let mut scenario_rule = MetricsAccumulator::default();
        let mut scenario_local = MetricsAccumulator::default();
        let mut scenario_markov_order1 = MetricsAccumulator::default();
        let mut scenario_markov_order2 = MetricsAccumulator::default();
        let mut scenario_notification = MetricsAccumulator::default();
        let mut scenario_naive_bayes = MetricsAccumulator::default();
        let mut scenario_hybrid = MetricsAccumulator::default();
        let mut scenario_enhanced = MetricsAccumulator::default();

        let mut label_contexts = Vec::new();
        for label in &scenario_labels {
            let context = context_for_label(label, &trace)?;
            if context.events.len() != label.context_event_count {
                anyhow::bail!(
                    "context event count drift for {} [{}..{}): expected {}, got {}",
                    label.scenario,
                    label.window_start_ms,
                    label.window_end_ms,
                    label.context_event_count,
                    context.events.len()
                );
            }
            label_contexts.push(LabelContext { label, context });
        }

        for (index, labeled_context) in label_contexts.iter().enumerate() {
            let label = labeled_context.label;
            let context = &labeled_context.context;
            if !label.eligible {
                continue;
            }
            let Some(actual) = label.actual_next_app.as_deref() else {
                anyhow::bail!("eligible label has no actual_next_app: {label:?}");
            };

            let current_app = label.current_app.as_deref();
            let rule_candidates =
                ranked_candidates(&RuleBasedBackend.evaluate(context), current_app);
            let local_candidates =
                ranked_candidates(&LocalEvaluatorBackend.evaluate(context), current_app);
            let markov_order1_candidates = markov.predict_order1(label, context);
            let markov_order2_candidates = markov.predict_order2_backoff(label, context);
            let notification_candidates = notification_predictor.predict(label, context);
            let naive_bayes = NaiveBayesContextPredictor::from_history(
                &label_contexts[..index],
                label.window_end_ms,
            );
            let naive_bayes_candidates = naive_bayes.predict(label, context);
            let hybrid_candidates =
                hybrid_candidates(&markov_order2_candidates, &local_candidates, 0.6, 0.4);
            let enhanced_candidates = enhanced_local_candidates(
                &markov_order1_candidates,
                &naive_bayes_candidates,
                &notification_candidates,
                &local_candidates,
            );

            scenario_rule.record(actual, &rule_candidates);
            scenario_local.record(actual, &local_candidates);
            scenario_markov_order1.record(actual, &markov_order1_candidates);
            scenario_markov_order2.record(actual, &markov_order2_candidates);
            scenario_notification.record(actual, &notification_candidates);
            scenario_naive_bayes.record(actual, &naive_bayes_candidates);
            scenario_hybrid.record(actual, &hybrid_candidates);
            scenario_enhanced.record(actual, &enhanced_candidates);
        }

        let scenario_metrics = ScenarioMetrics {
            scenario,
            windows: scenario_labels.len() as u64,
            eligible_windows: scenario_labels
                .iter()
                .filter(|label| label.eligible)
                .count() as u64,
            rule_based: scenario_rule.finish(),
            local_evaluator: scenario_local.finish(),
            markov_order1: scenario_markov_order1.finish(),
            markov_order2_backoff: scenario_markov_order2.finish(),
            notification_conditioned: scenario_notification.finish(),
            naive_bayes_context: scenario_naive_bayes.finish(),
            hybrid_markov_local: scenario_hybrid.finish(),
            enhanced_local: scenario_enhanced.finish(),
        };
        report.total_windows += scenario_metrics.windows;
        report.eligible_windows += scenario_metrics.eligible_windows;
        report.scenarios.push(scenario_metrics);
    }

    report.rule_based = aggregate_backend(&report.scenarios, |scenario| &scenario.rule_based);
    report.local_evaluator =
        aggregate_backend(&report.scenarios, |scenario| &scenario.local_evaluator);
    report.markov_order1 = aggregate_backend(&report.scenarios, |scenario| &scenario.markov_order1);
    report.markov_order2_backoff = aggregate_backend(&report.scenarios, |scenario| {
        &scenario.markov_order2_backoff
    });
    report.notification_conditioned = aggregate_backend(&report.scenarios, |scenario| {
        &scenario.notification_conditioned
    });
    report.naive_bayes_context =
        aggregate_backend(&report.scenarios, |scenario| &scenario.naive_bayes_context);
    report.hybrid_markov_local =
        aggregate_backend(&report.scenarios, |scenario| &scenario.hybrid_markov_local);
    report.enhanced_local =
        aggregate_backend(&report.scenarios, |scenario| &scenario.enhanced_local);
    Ok(report)
}

#[derive(Debug, Clone)]
struct ForegroundTransition {
    previous: Option<String>,
    from: String,
    to: String,
    at_ms: i64,
}

#[derive(Debug, Clone, Default)]
struct MarkovTransitionPredictor {
    transitions: Vec<ForegroundTransition>,
}

impl MarkovTransitionPredictor {
    fn from_trace(trace: &[TraceEvent]) -> Self {
        let mut transitions = Vec::new();
        let mut previous: Option<&str> = None;
        let mut current: Option<&str> = None;

        for event in trace {
            let RawEvent::AppTransition(app_transition) = &event.raw_event else {
                continue;
            };
            if app_transition.transition != AppTransition::Foreground {
                continue;
            }
            let next = app_transition.package_name.as_str();
            if current == Some(next) {
                continue;
            }
            if let Some(from) = current {
                transitions.push(ForegroundTransition {
                    previous: previous.map(str::to_string),
                    from: from.to_string(),
                    to: next.to_string(),
                    at_ms: event.timestamp_ms,
                });
            }
            previous = current;
            current = Some(next);
        }

        Self { transitions }
    }

    fn predict_order1(&self, label: &NextAppLabel, context: &StructuredContext) -> Vec<String> {
        let Some(current_app) = label.current_app.as_deref() else {
            return Vec::new();
        };
        let observable = observable_candidates(context, current_app);
        if observable.is_empty() {
            return Vec::new();
        }

        let mut counts: HashMap<&str, u32> = HashMap::new();
        for transition in &self.transitions {
            if transition.at_ms >= label.window_end_ms {
                break;
            }
            if transition.from == current_app && observable.contains(&transition.to.as_str()) {
                *counts.entry(transition.to.as_str()).or_insert(0) += 1;
            }
        }
        if counts.is_empty() {
            return Vec::new();
        }

        let mut ranked: Vec<(&str, u32)> = counts.into_iter().collect();
        ranked.sort_by(|(app_a, count_a), (app_b, count_b)| {
            count_b.cmp(count_a).then_with(|| app_a.cmp(app_b))
        });
        ranked
            .into_iter()
            .map(|(package, _)| package.to_string())
            .collect()
    }

    fn predict_order2_backoff(
        &self,
        label: &NextAppLabel,
        context: &StructuredContext,
    ) -> Vec<String> {
        let Some(current_app) = label.current_app.as_deref() else {
            return Vec::new();
        };
        let Some(previous_app) = self.previous_app_before(current_app, label.window_end_ms) else {
            return self.predict_order1(label, context);
        };
        let observable = observable_candidates(context, current_app);
        if observable.is_empty() {
            return Vec::new();
        }

        let mut counts: HashMap<&str, u32> = HashMap::new();
        for transition in &self.transitions {
            if transition.at_ms >= label.window_end_ms {
                break;
            }
            if transition.previous.as_deref() == Some(previous_app)
                && transition.from == current_app
                && observable.contains(&transition.to.as_str())
            {
                *counts.entry(transition.to.as_str()).or_insert(0) += 1;
            }
        }

        if counts.is_empty() {
            return self.predict_order1(label, context);
        }

        let mut ranked: Vec<(&str, u32)> = counts.into_iter().collect();
        ranked.sort_by(|(app_a, count_a), (app_b, count_b)| {
            count_b.cmp(count_a).then_with(|| app_a.cmp(app_b))
        });
        ranked
            .into_iter()
            .map(|(package, _)| package.to_string())
            .collect()
    }

    fn previous_app_before(&self, current_app: &str, window_end_ms: i64) -> Option<&str> {
        self.transitions
            .iter()
            .rev()
            .find(|transition| transition.at_ms < window_end_ms && transition.to == current_app)
            .map(|transition| transition.from.as_str())
    }
}

#[derive(Debug, Clone)]
struct NotificationTransition {
    current_app: String,
    notified_app: String,
    to: String,
    at_ms: i64,
}

#[derive(Debug, Clone, Default)]
struct NotificationConditionedPredictor {
    transitions: Vec<NotificationTransition>,
}

impl NotificationConditionedPredictor {
    fn from_trace(trace: &[TraceEvent]) -> Self {
        let mut transitions = Vec::new();
        let mut current_app: Option<&str> = None;
        let mut recent_notifications: Vec<(i64, &str)> = Vec::new();

        for event in trace {
            match &event.raw_event {
                RawEvent::NotificationPosted(notification) => {
                    recent_notifications
                        .push((event.timestamp_ms, notification.package_name.as_str()));
                    recent_notifications
                        .retain(|(at_ms, _)| event.timestamp_ms.saturating_sub(*at_ms) <= 30_000);
                },
                RawEvent::NotificationInteraction(notification) => {
                    recent_notifications
                        .push((event.timestamp_ms, notification.package_name.as_str()));
                    recent_notifications
                        .retain(|(at_ms, _)| event.timestamp_ms.saturating_sub(*at_ms) <= 30_000);
                },
                RawEvent::AppTransition(app_transition)
                    if app_transition.transition == AppTransition::Foreground =>
                {
                    let next = app_transition.package_name.as_str();
                    if current_app == Some(next) {
                        continue;
                    }
                    if let Some(from) = current_app {
                        for (notification_at_ms, notified_app) in
                            recent_notifications.iter().copied()
                        {
                            if event.timestamp_ms.saturating_sub(notification_at_ms) <= 30_000 {
                                transitions.push(NotificationTransition {
                                    current_app: from.to_string(),
                                    notified_app: notified_app.to_string(),
                                    to: next.to_string(),
                                    at_ms: event.timestamp_ms,
                                });
                            }
                        }
                    }
                    current_app = Some(next);
                },
                _ => {},
            }
        }

        Self { transitions }
    }

    fn predict(&self, label: &NextAppLabel, context: &StructuredContext) -> Vec<String> {
        let Some(current_app) = label.current_app.as_deref() else {
            return Vec::new();
        };
        let observable = observable_candidates(context, current_app);
        let notified_apps = context.summary.notified_apps.as_slice();
        if observable.is_empty() || notified_apps.is_empty() {
            return Vec::new();
        }

        let mut pair_counts: HashMap<&str, u32> = HashMap::new();
        let mut notified_counts: HashMap<&str, u32> = HashMap::new();
        for transition in &self.transitions {
            if transition.at_ms >= label.window_end_ms {
                break;
            }
            if !notified_apps.contains(&transition.notified_app) {
                continue;
            }
            if !observable.contains(&transition.to.as_str()) {
                continue;
            }
            *notified_counts.entry(transition.to.as_str()).or_insert(0) += 1;
            if transition.current_app == current_app {
                *pair_counts.entry(transition.to.as_str()).or_insert(0) += 1;
            }
        }

        let counts = if pair_counts.is_empty() {
            notified_counts
        } else {
            pair_counts
        };
        if counts.is_empty() {
            return Vec::new();
        }

        rank_counts(counts)
    }
}

#[derive(Debug, Clone, Default)]
struct NaiveBayesContextPredictor {
    class_counts: HashMap<String, u32>,
    feature_counts: HashMap<String, HashMap<String, u32>>,
    class_feature_totals: HashMap<String, u32>,
    vocabulary: Vec<String>,
    total_examples: u32,
}

impl NaiveBayesContextPredictor {
    fn from_history(history: &[LabelContext<'_>], current_window_end_ms: i64) -> Self {
        let mut predictor = Self::default();
        for labeled_context in history {
            let label = labeled_context.label;
            if !label.eligible {
                continue;
            }
            let Some(actual) = label.actual_next_app.as_deref() else {
                continue;
            };
            if label
                .actual_next_app_at_ms
                .is_none_or(|actual_at_ms| actual_at_ms >= current_window_end_ms)
            {
                continue;
            }
            predictor.observe(
                actual,
                &features_for_context(label, &labeled_context.context),
            );
        }
        predictor
    }

    fn observe(&mut self, class: &str, features: &[String]) {
        *self.class_counts.entry(class.to_string()).or_insert(0) += 1;
        self.total_examples += 1;

        for feature in features {
            if !self.vocabulary.contains(feature) {
                self.vocabulary.push(feature.clone());
            }
            *self
                .feature_counts
                .entry(class.to_string())
                .or_default()
                .entry(feature.clone())
                .or_insert(0) += 1;
            *self
                .class_feature_totals
                .entry(class.to_string())
                .or_insert(0) += 1;
        }
    }

    fn predict(&self, label: &NextAppLabel, context: &StructuredContext) -> Vec<String> {
        if self.total_examples == 0 {
            return Vec::new();
        }
        let Some(current_app) = label.current_app.as_deref() else {
            return Vec::new();
        };
        let candidates = observable_candidates(context, current_app);
        if candidates.is_empty() {
            return Vec::new();
        }

        let features = features_for_context(label, context);
        let class_space = self.class_counts.len().max(1) as f64;
        let vocabulary_size = self.vocabulary.len().max(1) as f64;
        let mut scored = Vec::new();

        for candidate in candidates {
            let class_count = *self.class_counts.get(candidate).unwrap_or(&0) as f64;
            let mut log_score =
                ((class_count + 1.0) / (self.total_examples as f64 + class_space)).ln();
            let class_feature_total =
                *self.class_feature_totals.get(candidate).unwrap_or(&0) as f64;
            let feature_counts = self.feature_counts.get(candidate);

            for feature in &features {
                let count = feature_counts
                    .and_then(|counts| counts.get(feature))
                    .copied()
                    .unwrap_or(0) as f64;
                log_score += ((count + 1.0) / (class_feature_total + vocabulary_size)).ln();
            }
            scored.push((candidate, log_score));
        }

        scored.sort_by(|(app_a, score_a), (app_b, score_b)| {
            score_b.total_cmp(score_a).then_with(|| app_a.cmp(app_b))
        });
        scored
            .into_iter()
            .map(|(package, _)| package.to_string())
            .collect()
    }
}

fn parse_trace<R: Read>(reader: R) -> Result<Vec<TraceEvent>> {
    let mut events = Vec::new();
    for (line_idx, line) in BufReader::new(reader).lines().enumerate() {
        let line_no = line_idx + 1;
        let line = line.with_context(|| format!("read trace line {line_no}"))?;
        if line.trim().is_empty() {
            continue;
        }
        let value: serde_json::Value =
            serde_json::from_str(&line).with_context(|| format!("parse trace line {line_no}"))?;
        let raw_event_value = value.get("rawEvent").cloned().unwrap_or_default();
        if raw_event_value.is_null() {
            continue;
        }
        let raw_event: RawEvent = serde_json::from_value(raw_event_value)
            .with_context(|| format!("parse rawEvent on trace line {line_no}"))?;
        let timestamp_ms = value
            .get("timestampMs")
            .and_then(serde_json::Value::as_i64)
            .unwrap_or_else(|| raw_timestamp_ms(&raw_event));
        let event_id = value
            .get("eventId")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .to_string();
        events.push(TraceEvent {
            event_id,
            line_no,
            timestamp_ms,
            raw_event,
        });
    }

    events.sort_by(|a, b| {
        (a.timestamp_ms, a.event_id.as_str(), a.line_no).cmp(&(
            b.timestamp_ms,
            b.event_id.as_str(),
            b.line_no,
        ))
    });
    Ok(events)
}

fn context_for_label(label: &NextAppLabel, trace: &[TraceEvent]) -> Result<StructuredContext> {
    let ingress = RustCollectorIngress;
    let sanitizer = DefaultPrivacyAirGap;
    let mut events = Vec::new();

    for event in trace.iter().filter(|event| {
        event.timestamp_ms >= label.window_start_ms && event.timestamp_ms < label.window_end_ms
    }) {
        let envelope = CollectorEnvelope {
            schema_version: SCHEMA_VERSION.to_string(),
            source: "next-app-benchmark".into(),
            source_tier: SourceTier::PublicApi,
            device_trace_id: Some(event.event_id.clone()),
            captured_at_ms: event.timestamp_ms,
            received_at_ms: None,
            raw_event: event.raw_event.clone(),
        };
        let ingested = ingress.accept(envelope)?;
        events.push(sanitizer.sanitize_with_tier(ingested.raw_event, ingested.source_tier));
    }

    let summary = build_summary(&events);
    Ok(StructuredContext {
        window_id: format!(
            "{}:{}-{}",
            label.scenario, label.window_start_ms, label.window_end_ms
        ),
        window_start_ms: label.window_start_ms,
        window_end_ms: label.window_end_ms,
        duration_secs: ((label.window_end_ms - label.window_start_ms).max(0) / 1000) as u32,
        events,
        summary,
    })
}

fn build_summary(events: &[SanitizedEvent]) -> ContextSummary {
    let mut foreground_apps: Vec<String> = Vec::new();
    let mut notified_apps: Vec<String> = Vec::new();
    let mut all_semantic_hints: Vec<SemanticHint> = Vec::new();
    let mut file_activity_counts: HashMap<ExtensionCategory, u32> = HashMap::new();
    let mut latest_system_status: Option<SystemStatusSnapshot> = None;
    let mut source_tier = SourceTier::PublicApi;

    for event in events {
        if event.source_tier == SourceTier::Daemon {
            source_tier = SourceTier::Daemon;
        }
        match &event.event_type {
            SanitizedEventType::AppTransition {
                package_name,
                transition: AppTransition::Foreground,
                ..
            } if !foreground_apps.contains(package_name) => {
                foreground_apps.push(package_name.clone());
            },
            SanitizedEventType::ProcessResource {
                package_name: Some(pkg),
                ..
            } if !foreground_apps.contains(pkg) => {
                foreground_apps.push(pkg.clone());
            },
            SanitizedEventType::Notification {
                source_package,
                semantic_hints,
                ..
            } => {
                if !notified_apps.contains(source_package) {
                    notified_apps.push(source_package.clone());
                }
                for hint in semantic_hints {
                    if !all_semantic_hints.contains(hint) {
                        all_semantic_hints.push(hint.clone());
                    }
                }
            },
            SanitizedEventType::FileActivity {
                extension_category, ..
            } => {
                *file_activity_counts
                    .entry(extension_category.clone())
                    .or_insert(0) += 1;
            },
            SanitizedEventType::SystemStatus {
                battery_pct,
                is_charging,
                network,
                ringer_mode,
                location_type,
                headphone_connected,
            } => {
                latest_system_status = Some(SystemStatusSnapshot {
                    battery_pct: *battery_pct,
                    is_charging: *is_charging,
                    network: network.clone(),
                    ringer_mode: ringer_mode.clone(),
                    location_type: location_type.clone(),
                    headphone_connected: *headphone_connected,
                });
            },
            SanitizedEventType::InterAppInteraction {
                source_package: Some(pkg),
                ..
            } if !foreground_apps.contains(pkg) => {
                foreground_apps.push(pkg.clone());
            },
            _ => {},
        }
    }

    ContextSummary {
        foreground_apps,
        notified_apps,
        all_semantic_hints,
        file_activity: file_activity_counts.into_iter().collect(),
        latest_system_status,
        source_tier,
    }
}

fn ranked_candidates(result: &DecisionBackendResult, current_app: Option<&str>) -> Vec<String> {
    let mut candidates = Vec::new();
    let mut ordinal = 0usize;

    for intent in &result.intent_batch.intents {
        if let Some(package) = package_from_intent(&intent.intent_type) {
            candidates.push(Candidate {
                package,
                confidence: intent.confidence,
                ordinal,
            });
            ordinal += 1;
        }
        for action in &intent.suggested_actions {
            if let Some(package) = action
                .target
                .as_deref()
                .and_then(|target| target.strip_prefix("pkg:"))
            {
                candidates.push(Candidate {
                    package: package.to_string(),
                    confidence: intent.confidence,
                    ordinal,
                });
                ordinal += 1;
            }
        }
    }

    let mut best_by_package: HashMap<String, Candidate> = HashMap::new();
    for candidate in candidates {
        if current_app.is_some_and(|current| current == candidate.package) {
            continue;
        }
        let replace = best_by_package
            .get(&candidate.package)
            .is_none_or(|existing| {
                candidate.confidence > existing.confidence
                    || (candidate.confidence == existing.confidence
                        && candidate.ordinal < existing.ordinal)
            });
        if replace {
            best_by_package.insert(candidate.package.clone(), candidate);
        }
    }

    let mut unique: Vec<Candidate> = best_by_package.into_values().collect();
    unique.sort_by(|a, b| {
        b.confidence
            .total_cmp(&a.confidence)
            .then_with(|| a.ordinal.cmp(&b.ordinal))
            .then_with(|| a.package.cmp(&b.package))
    });
    unique
        .into_iter()
        .map(|candidate| candidate.package)
        .collect()
}

fn package_from_intent(intent_type: &IntentType) -> Option<String> {
    match intent_type {
        IntentType::OpenApp(package)
        | IntentType::SwitchToApp(package)
        | IntentType::CheckNotification(package) => Some(package.clone()),
        _ => None,
    }
}

fn observable_candidates<'a>(context: &'a StructuredContext, current_app: &str) -> Vec<&'a str> {
    let mut candidates = Vec::new();
    for package in context
        .summary
        .foreground_apps
        .iter()
        .chain(context.summary.notified_apps.iter())
    {
        if package != current_app && !candidates.contains(&package.as_str()) {
            candidates.push(package.as_str());
        }
    }
    candidates
}

fn rank_counts(counts: HashMap<&str, u32>) -> Vec<String> {
    let mut ranked: Vec<(&str, u32)> = counts.into_iter().collect();
    ranked.sort_by(|(app_a, count_a), (app_b, count_b)| {
        count_b.cmp(count_a).then_with(|| app_a.cmp(app_b))
    });
    ranked
        .into_iter()
        .map(|(package, _)| package.to_string())
        .collect()
}

fn features_for_context(label: &NextAppLabel, context: &StructuredContext) -> Vec<String> {
    let mut features = Vec::new();
    if let Some(current_app) = label.current_app.as_deref() {
        features.push(format!("current:{current_app}"));
    }
    for package in &context.summary.foreground_apps {
        if label.current_app.as_deref() != Some(package.as_str()) {
            features.push(format!("foreground:{package}"));
        }
    }
    for package in &context.summary.notified_apps {
        features.push(format!("notified:{package}"));
    }
    for hint in &context.summary.all_semantic_hints {
        features.push(format!("hint:{hint:?}"));
    }
    if let Some(status) = &context.summary.latest_system_status {
        features.push(format!("network:{:?}", status.network));
        features.push(format!("charging:{}", status.is_charging));
        features.push(format!("location:{:?}", status.location_type));
    }
    let hour_bucket = ((label.window_end_ms / 3_600_000).rem_euclid(24)) as u8;
    features.push(format!("hour:{hour_bucket}"));
    features
}

fn hybrid_candidates(
    markov_candidates: &[String],
    local_candidates: &[String],
    markov_weight: f64,
    local_weight: f64,
) -> Vec<String> {
    let mut scores: HashMap<&str, f64> = HashMap::new();
    add_rank_scores(&mut scores, markov_candidates, markov_weight);
    add_rank_scores(&mut scores, local_candidates, local_weight);

    let mut ranked: Vec<(&str, f64)> = scores.into_iter().collect();
    ranked.sort_by(|(app_a, score_a), (app_b, score_b)| {
        score_b.total_cmp(score_a).then_with(|| app_a.cmp(app_b))
    });
    ranked
        .into_iter()
        .map(|(package, _)| package.to_string())
        .collect()
}

fn enhanced_local_candidates(
    markov_order1_candidates: &[String],
    naive_bayes_candidates: &[String],
    notification_candidates: &[String],
    local_candidates: &[String],
) -> Vec<String> {
    let mut scores: HashMap<&str, f64> = HashMap::new();
    add_rank_scores(&mut scores, markov_order1_candidates, 0.70);
    add_rank_scores(&mut scores, naive_bayes_candidates, 0.35);
    add_rank_scores(&mut scores, local_candidates, 0.05);

    let supported_candidates: Vec<&str> = markov_order1_candidates
        .iter()
        .chain(naive_bayes_candidates.iter())
        .chain(local_candidates.iter())
        .map(String::as_str)
        .collect();
    add_filtered_rank_scores(
        &mut scores,
        notification_candidates,
        0.30,
        &supported_candidates,
    );

    let mut ranked: Vec<(&str, f64)> = scores.into_iter().collect();
    ranked.sort_by(|(app_a, score_a), (app_b, score_b)| {
        score_b.total_cmp(score_a).then_with(|| app_a.cmp(app_b))
    });
    ranked
        .into_iter()
        .map(|(package, _)| package.to_string())
        .collect()
}

fn add_rank_scores<'a>(scores: &mut HashMap<&'a str, f64>, candidates: &'a [String], weight: f64) {
    for (index, package) in candidates.iter().enumerate() {
        let rank_score = 1.0 / (index + 1) as f64;
        *scores.entry(package.as_str()).or_insert(0.0) += weight * rank_score;
    }
}

fn add_filtered_rank_scores<'a>(
    scores: &mut HashMap<&'a str, f64>,
    candidates: &'a [String],
    weight: f64,
    supported_candidates: &[&str],
) {
    for (index, package) in candidates.iter().enumerate() {
        if !supported_candidates.contains(&package.as_str()) {
            continue;
        }
        let rank_score = 1.0 / (index + 1) as f64;
        *scores.entry(package.as_str()).or_insert(0.0) += weight * rank_score;
    }
}

fn aggregate_backend<F>(scenarios: &[ScenarioMetrics], pick: F) -> BackendMetrics
where
    F: Fn(&ScenarioMetrics) -> &BackendMetrics,
{
    let mut accumulator = MetricsAccumulator::default();
    for metrics in scenarios.iter().map(pick) {
        accumulator.eligible_windows += metrics.eligible_windows;
        accumulator.predicted_windows += metrics.predicted_windows;
        accumulator.top1_hits += metrics.top1_hits;
        accumulator.top3_hits += metrics.top3_hits;
        accumulator.reciprocal_rank_sum +=
            metrics.mean_reciprocal_rank * metrics.eligible_windows as f64;
    }
    accumulator.finish()
}

fn raw_timestamp_ms(raw_event: &RawEvent) -> i64 {
    match raw_event {
        RawEvent::AppTransition(event) => event.timestamp_ms,
        RawEvent::BinderTransaction(event) => event.timestamp_ms,
        RawEvent::ProcStateChange(event) => event.timestamp_ms,
        RawEvent::FileSystemAccess(event) => event.timestamp_ms,
        RawEvent::NotificationPosted(event) => event.timestamp_ms,
        RawEvent::NotificationInteraction(event) => event.timestamp_ms,
        RawEvent::ScreenState(event) => event.timestamp_ms,
        RawEvent::SystemState(event) => event.timestamp_ms,
    }
}

fn pct(numerator: f64, denominator: f64) -> f64 {
    if denominator == 0.0 {
        0.0
    } else {
        round3(numerator * 100.0 / denominator)
    }
}

fn round3(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}
