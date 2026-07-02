//! Benchmark runner: train/test split, per-scenario evaluation, and report generation.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use anyhow::{bail, Context, Result};
use tracing::info;

use super::action_value::{
    compute_action_value_metrics, evaluate_action_value, ActionValueMode, ActionValueRecord,
};
use super::baselines::{
    AlwaysNoOpBackend, FirstCandidateBackend, GlobalMajorityBackend, LastAppPrewarmBackend,
    LastForegroundBackend, MarkovBackend, NotificationPriorityBackend,
    PerCurrentAppMajorityBackend, RandomCandidateBackend, RecentNotificationBackend,
    StrongPredictiveActionBackend,
};
use super::context_loader::{extract_observable_candidates, load_contexts_by_label, load_labels};
use super::metrics::{compute_backend_metrics, round3, PredictionRecord};
use super::predictor::{LocalEvaluatorNextAppBackend, RuleBasedNextAppBackend};
use super::report::SCHEMA_VERSION;
use super::types::{
    BackendMetrics, BenchmarkConfig, BenchmarkReport, ExclusionCounts, NextAppLabel,
    NextAppPredictor, ScenarioReport,
};

pub struct BenchmarkRunConfig {
    pub inputs: Vec<std::path::PathBuf>,
    pub labels: std::path::PathBuf,
    pub train_split: f64,
    pub window_secs: u64,
}

pub fn run_benchmark(config: &BenchmarkRunConfig) -> Result<BenchmarkReport> {
    if !(0.0..1.0).contains(&config.train_split) {
        bail!("train_split must be in (0, 1), got {}", config.train_split);
    }

    let labels = load_labels(&config.labels)
        .with_context(|| format!("loading labels {}", config.labels.display()))?;

    let mut by_scenario: HashMap<String, Vec<NextAppLabel>> = HashMap::new();
    let mut exclusion_counts = ExclusionCounts::default();
    for label in &labels {
        by_scenario
            .entry(label.scenario.clone())
            .or_default()
            .push(label.clone());
        if label.eligible {
            exclusion_counts.eligible += 1;
        } else if let Some(reason) = &label.excluded_reason {
            match reason.as_str() {
                "label_not_observable" => exclusion_counts.label_not_observable += 1,
                "no_future_switch" => exclusion_counts.no_future_switch += 1,
                _ => {},
            }
        }
    }

    let horizon_secs = labels
        .first()
        .map(|l| l.prediction_horizon_ms / 1000)
        .unwrap_or(30);

    let input_pairs: Vec<(String, &Path)> = config
        .inputs
        .iter()
        .map(|p| {
            let scenario = p
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string())
                .with_context(|| format!("invalid input path {}", p.display()))?;
            Ok((scenario, p.as_path()))
        })
        .collect::<Result<Vec<_>>>()?;

    let contexts = load_contexts_by_label(&input_pairs, &labels, config.window_secs)
        .context("loading contexts from traces")?;

    let mut scenarios = Vec::new();
    let mut aggregate_records: HashMap<String, Vec<PredictionRecord>> = HashMap::new();
    let mut aggregate_action_records: HashMap<String, Vec<ActionValueRecord>> = HashMap::new();
    let mut aggregate_latencies: HashMap<String, Vec<u64>> = HashMap::new();

    for (scenario_name, trace_path) in &input_pairs {
        let scenario_labels = by_scenario
            .remove(scenario_name)
            .with_context(|| format!("no labels for scenario '{}'", scenario_name))?;

        let mut eligible: Vec<NextAppLabel> = scenario_labels
            .iter()
            .filter(|l| l.eligible)
            .cloned()
            .collect();
        eligible.sort_by_key(|l| l.window_start_ms);

        let n = eligible.len();
        let split_idx = ((n as f64) * config.train_split)
            .floor()
            .max(1.0)
            .min(n.saturating_sub(1) as f64) as usize;
        let (train, test) = eligible.split_at(split_idx);

        let mut predictors: Vec<Box<dyn NextAppPredictor>> = vec![
            Box::new(RuleBasedNextAppBackend),
            Box::new(LocalEvaluatorNextAppBackend),
            Box::new(StrongPredictiveActionBackend::default()),
            Box::new(AlwaysNoOpBackend),
            Box::new(RandomCandidateBackend::new(42)),
            Box::new(FirstCandidateBackend),
            Box::new(GlobalMajorityBackend::default()),
            Box::new(PerCurrentAppMajorityBackend::default()),
            Box::new(MarkovBackend::default()),
            Box::new(RecentNotificationBackend),
            Box::new(LastForegroundBackend),
            Box::new(NotificationPriorityBackend),
            Box::new(LastAppPrewarmBackend),
        ];

        for predictor in &mut predictors {
            predictor.train(train);
        }

        let mut backend_metrics: BTreeMap<String, BackendMetrics> = BTreeMap::new();
        for predictor in predictors.iter() {
            let (records, latencies, action_records) =
                evaluate_predictor(predictor.as_ref(), test, &contexts, scenario_name)?;
            let mut metrics = compute_backend_metrics(&records, test.len());
            metrics.action_value = compute_action_value_metrics(&action_records);
            backend_metrics.insert(predictor.name().to_string(), metrics);
            aggregate_records
                .entry(predictor.name().to_string())
                .or_default()
                .extend(records);
            aggregate_action_records
                .entry(predictor.name().to_string())
                .or_default()
                .extend(action_records);
            aggregate_latencies
                .entry(predictor.name().to_string())
                .or_default()
                .extend(latencies);
        }

        let (oracle_records, oracle_actions) = oracle_records(test);
        let mut oracle_metrics = compute_backend_metrics(&oracle_records, test.len());
        oracle_metrics.action_value = compute_action_value_metrics(&oracle_actions);
        backend_metrics.insert("oracle_upper_bound".into(), oracle_metrics);
        aggregate_records
            .entry("oracle_upper_bound".into())
            .or_default()
            .extend(oracle_records);
        aggregate_action_records
            .entry("oracle_upper_bound".into())
            .or_default()
            .extend(oracle_actions);

        let (full_loop_records, full_loop_actions) =
            dipecs_full_loop_records(test, &contexts, scenario_name)?;
        let mut full_loop_metrics = compute_backend_metrics(&full_loop_records, test.len());
        full_loop_metrics.action_value = compute_action_value_metrics(&full_loop_actions);
        backend_metrics.insert("dipecs_full_loop".into(), full_loop_metrics);
        aggregate_records
            .entry("dipecs_full_loop".into())
            .or_default()
            .extend(full_loop_records);
        aggregate_action_records
            .entry("dipecs_full_loop".into())
            .or_default()
            .extend(full_loop_actions);

        let scenario_exclusions =
            exclusion_counts_for_scenario(&scenario_labels.iter().collect::<Vec<_>>());

        scenarios.push(ScenarioReport {
            scenario: (*scenario_name).to_string(),
            windows: scenario_labels.len(),
            future_switch_windows: scenario_labels.len() - scenario_exclusions.no_future_switch,
            eligible_windows: scenario_exclusions.eligible,
            train_windows: train.len(),
            test_windows: test.len(),
            exclusions: scenario_exclusions,
            backends: backend_metrics,
        });

        info!(
            scenario = %scenario_name,
            train = train.len(),
            test = test.len(),
            trace = %trace_path.display(),
            "benchmarked scenario"
        );
    }

    if !by_scenario.is_empty() {
        bail!(
            "missing input traces for scenarios: {:?}",
            by_scenario.keys()
        );
    }

    let mut aggregate: BTreeMap<String, BackendMetrics> = BTreeMap::new();
    for (name, records) in &aggregate_records {
        let mut metrics = compute_backend_metrics(records, records.len());
        let scenario_top1: Vec<f64> = scenarios
            .iter()
            .filter(|s| s.test_windows > 0)
            .filter_map(|s| s.backends.get(name).map(|m| m.top1_accuracy_pct))
            .collect();
        let scenario_top3: Vec<f64> = scenarios
            .iter()
            .filter(|s| s.test_windows > 0)
            .filter_map(|s| s.backends.get(name).map(|m| m.top3_accuracy_pct))
            .collect();
        metrics.macro_top1_accuracy_pct = Some(round3(
            scenario_top1.iter().sum::<f64>() / scenario_top1.len().max(1) as f64,
        ));
        metrics.macro_top3_accuracy_pct = Some(round3(
            scenario_top3.iter().sum::<f64>() / scenario_top3.len().max(1) as f64,
        ));
        if let Some(latencies) = aggregate_latencies.get(name) {
            metrics.latency_us = super::metrics::latency_summary(latencies);
        }
        if let Some(action_records) = aggregate_action_records.get(name) {
            metrics.action_value = compute_action_value_metrics(action_records);
        }
        aggregate.insert(name.clone(), metrics);
    }

    let future_switch_windows = exclusion_counts.eligible + exclusion_counts.label_not_observable;
    let coverage_pct =
        round3((exclusion_counts.eligible as f64 / future_switch_windows.max(1) as f64) * 100.0);

    Ok(BenchmarkReport {
        schema_version: SCHEMA_VERSION.into(),
        dataset_id: labels
            .first()
            .map(|l| l.dataset_id.clone())
            .unwrap_or_default(),
        source: "synthetic".into(),
        config: BenchmarkConfig {
            window_secs: config.window_secs,
            horizon_secs,
            label_definition: "first foreground transition within the horizon whose package differs from the app current at window end".into(),
            eligibility_definition: "actual_next_app is present in the current window's foreground or notified app set after removing current_app".into(),
            train_split: config.train_split,
        },
        inputs: config.inputs.clone(),
        total_windows: labels.len(),
        eligible_windows: exclusion_counts.eligible,
        train_windows: scenarios.iter().map(|s| s.train_windows).sum(),
        test_windows: scenarios.iter().map(|s| s.test_windows).sum(),
        context_supported_switch_coverage_pct: coverage_pct,
        exclusions: exclusion_counts,
        aggregate,
        scenarios,
        limitations: vec![
            "All labels are derived from deterministic synthetic traces, not real-user behavior.".into(),
            "Accuracy is reported only for context-supported switches; coverage reports how selective that cohort is.".into(),
            "Latency is measured during this run and is not byte-deterministic across machines.".into(),
            "Statistical baselines are trained on the per-scenario train split and evaluated on the held-out test split.".into(),
            "StrongPredictiveActionBaseline is the primary action-value baseline: it ensembles Android-observable recency, notification, frequency, and Markov signals, then pays direct-action costs.".into(),
            "Action-value metrics are deterministic offline estimates; emulator/on-device scripts remain required for causal startup, memory, jank, and bridge latency claims.".into(),
        ],
    })
}

fn evaluate_predictor(
    predictor: &dyn NextAppPredictor,
    test: &[NextAppLabel],
    contexts: &BTreeMap<(String, i64, i64), aios_spec::StructuredContext>,
    scenario: &str,
) -> Result<(Vec<PredictionRecord>, Vec<u64>, Vec<ActionValueRecord>)> {
    let mut records = Vec::with_capacity(test.len());
    let mut latencies = Vec::with_capacity(test.len());
    let mut action_records = Vec::with_capacity(test.len());

    for label in test {
        let ctx = contexts
            .get(&(
                label.scenario.clone(),
                label.window_start_ms,
                label.window_end_ms,
            ))
            .with_context(|| {
                format!(
                    "missing context for scenario {} window {}-{}",
                    scenario, label.window_start_ms, label.window_end_ms
                )
            })?;
        let current_app = &label.current_app;
        let candidates = extract_observable_candidates(ctx, current_app);

        let result = predictor.predict(ctx, current_app, &candidates);
        let rank = if let Some(actual) = &label.actual_next_app {
            result
                .ranked
                .iter()
                .position(|p| &p.package == actual)
                .map(|i| i + 1)
        } else {
            None
        };

        let mode = action_value_mode_for_backend(predictor.name());
        action_records.push(evaluate_action_value(mode, label, &result.ranked, 5));
        records.push(PredictionRecord {
            rank,
            predicted: !result.ranked.is_empty(),
            latency_us: result.latency_us,
            noop: result.ranked.is_empty(),
            rationale_present: result.rationale_present,
        });
        latencies.push(result.latency_us);
    }

    Ok((records, latencies, action_records))
}

fn action_value_mode_for_backend(name: &str) -> ActionValueMode {
    match name {
        "rule_based" | "local_evaluator" => ActionValueMode::GovernedTopK,
        "strong_predictive_action" => ActionValueMode::DirectTopK,
        "always_noop" => ActionValueMode::NoAction,
        _ => ActionValueMode::DirectTopK,
    }
}

fn oracle_records(test: &[NextAppLabel]) -> (Vec<PredictionRecord>, Vec<ActionValueRecord>) {
    let records = test
        .iter()
        .map(|label| PredictionRecord {
            rank: label.actual_next_app.as_ref().map(|_| 1),
            predicted: label.actual_next_app.is_some(),
            latency_us: 0,
            noop: label.actual_next_app.is_none(),
            rationale_present: false,
        })
        .collect();
    let actions = test
        .iter()
        .map(|label| evaluate_action_value(ActionValueMode::Oracle, label, &[], 1))
        .collect();
    (records, actions)
}

fn dipecs_full_loop_records(
    test: &[NextAppLabel],
    contexts: &BTreeMap<(String, i64, i64), aios_spec::StructuredContext>,
    scenario: &str,
) -> Result<(Vec<PredictionRecord>, Vec<ActionValueRecord>)> {
    let router = aios_agent::DecisionRouter::default();
    let mut prediction_records = Vec::with_capacity(test.len());
    let mut action_records = Vec::with_capacity(test.len());
    for label in test {
        let ctx = contexts
            .get(&(
                label.scenario.clone(),
                label.window_start_ms,
                label.window_end_ms,
            ))
            .with_context(|| {
                format!(
                    "missing context for scenario {} window {}-{}",
                    scenario, label.window_start_ms, label.window_end_ms
                )
            })?;
        let decision = router.evaluate(ctx);
        let prewarm_ranked: Vec<super::types::ScoredPrediction> = decision
            .intent_batch
            .intents
            .iter()
            .flat_map(|intent| intent.suggested_actions.iter())
            .filter_map(
                |action| match (&action.action_type, action.target.as_deref()) {
                    (aios_spec::ActionType::PreWarmProcess, Some(target)) => target
                        .strip_prefix("pkg:")
                        .map(|package| super::types::ScoredPrediction {
                            package: package.to_string(),
                            score: 1.0,
                        }),
                    _ => None,
                },
            )
            .collect();
        let rank = label.actual_next_app.as_ref().and_then(|actual| {
            prewarm_ranked
                .iter()
                .position(|prediction| &prediction.package == actual)
                .map(|idx| idx + 1)
        });
        prediction_records.push(PredictionRecord {
            rank,
            predicted: !prewarm_ranked.is_empty(),
            latency_us: decision.latency_us,
            noop: prewarm_ranked.is_empty(),
            rationale_present: decision
                .intent_batch
                .intents
                .iter()
                .any(|intent| !intent.rationale_tags.is_empty()),
        });
        action_records.push(evaluate_action_value(
            ActionValueMode::GovernedTopK,
            label,
            &prewarm_ranked,
            1,
        ));
    }
    Ok((prediction_records, action_records))
}

fn exclusion_counts_for_scenario(labels: &[&NextAppLabel]) -> ExclusionCounts {
    let mut counts = ExclusionCounts::default();
    for label in labels {
        if label.eligible {
            counts.eligible += 1;
        } else if let Some(reason) = &label.excluded_reason {
            match reason.as_str() {
                "label_not_observable" => counts.label_not_observable += 1,
                "no_future_switch" => counts.no_future_switch += 1,
                _ => {},
            }
        }
    }
    counts
}
