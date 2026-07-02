use aios_cli::next_app_benchmark::{parse_labels, run_benchmark, ScenarioInput};

const LABELS: &str = include_str!("../../../data/traces/synthetic-next-app-v1.labels.jsonl");
const MULTI_APP_SWITCHING: &str =
    include_str!("../../../data/traces/scenarios/multi-app-switching.jsonl");
const MORNING_ROUTINE: &str = include_str!("../../../data/traces/scenarios/morning-routine.jsonl");
const RICH_WORKFLOW: &str = include_str!("../../../data/traces/scenarios/rich-workflow.jsonl");

fn benchmark_inputs() -> Vec<ScenarioInput> {
    vec![
        ScenarioInput {
            scenario: "multi-app-switching".into(),
            jsonl: MULTI_APP_SWITCHING.into(),
        },
        ScenarioInput {
            scenario: "morning-routine".into(),
            jsonl: MORNING_ROUTINE.into(),
        },
        ScenarioInput {
            scenario: "rich-workflow".into(),
            jsonl: RICH_WORKFLOW.into(),
        },
    ]
}

#[test]
fn synthetic_next_app_labels_have_expected_shape() {
    let labels = parse_labels(LABELS.as_bytes()).expect("labels parse");

    assert_eq!(labels.len(), 946);
    assert_eq!(labels.iter().filter(|label| label.eligible).count(), 178);
    assert_eq!(
        labels
            .iter()
            .filter(|label| label.excluded_reason.as_deref() == Some("label_not_observable"))
            .count(),
        586
    );
    assert_eq!(
        labels
            .iter()
            .filter(|label| label.excluded_reason.as_deref() == Some("no_future_switch"))
            .count(),
        182
    );
}

#[test]
fn synthetic_next_app_benchmark_does_not_regress_from_committed_baseline() {
    let labels = parse_labels(LABELS.as_bytes()).expect("labels parse");
    let report = run_benchmark(&benchmark_inputs(), &labels).expect("benchmark runs");

    assert_eq!(report.total_windows, 946);
    assert_eq!(report.eligible_windows, 178);

    assert!(
        report.rule_based.top1_accuracy_pct >= 61.236,
        "RuleBased Top-1 regressed: {}",
        report.rule_based.top1_accuracy_pct
    );
    assert!(
        report.rule_based.top3_accuracy_pct >= 65.730,
        "RuleBased Top-3 regressed: {}",
        report.rule_based.top3_accuracy_pct
    );
    assert!(
        report.rule_based.prediction_coverage_pct >= 93.820,
        "RuleBased coverage regressed: {}",
        report.rule_based.prediction_coverage_pct
    );
    assert!(
        report.rule_based.wrong_prediction_rate_pct <= 34.731,
        "RuleBased wrong-prediction rate regressed: {}",
        report.rule_based.wrong_prediction_rate_pct
    );

    assert!(
        report.local_evaluator.top1_accuracy_pct >= 43.820,
        "LocalEvaluator Top-1 regressed: {}",
        report.local_evaluator.top1_accuracy_pct
    );
    assert!(
        report.local_evaluator.top3_accuracy_pct >= 62.921,
        "LocalEvaluator Top-3 regressed: {}",
        report.local_evaluator.top3_accuracy_pct
    );
    assert!(
        report.local_evaluator.prediction_coverage_pct >= 73.596,
        "LocalEvaluator coverage regressed: {}",
        report.local_evaluator.prediction_coverage_pct
    );
    assert!(
        report.local_evaluator.no_prediction_rate_pct <= 26.404,
        "LocalEvaluator no-prediction rate regressed: {}",
        report.local_evaluator.no_prediction_rate_pct
    );

    assert!(
        report.markov_order1.top1_accuracy_pct >= 71.910,
        "Markov-1 Top-1 regressed: {}",
        report.markov_order1.top1_accuracy_pct
    );
    assert!(
        report.markov_order1.top3_accuracy_pct >= 91.573,
        "Markov-1 Top-3 regressed: {}",
        report.markov_order1.top3_accuracy_pct
    );
    assert!(
        report.markov_order1.prediction_coverage_pct >= 96.067,
        "Markov-1 coverage regressed: {}",
        report.markov_order1.prediction_coverage_pct
    );
    assert!(
        report.markov_order1.wrong_prediction_rate_pct <= 25.146,
        "Markov-1 wrong-prediction rate regressed: {}",
        report.markov_order1.wrong_prediction_rate_pct
    );

    assert!(
        report.markov_order2_backoff.top1_accuracy_pct >= 66.854,
        "Markov-2 backoff Top-1 regressed: {}",
        report.markov_order2_backoff.top1_accuracy_pct
    );
    assert!(
        report.markov_order2_backoff.top3_accuracy_pct >= 79.775,
        "Markov-2 backoff Top-3 regressed: {}",
        report.markov_order2_backoff.top3_accuracy_pct
    );
    assert!(
        report.markov_order2_backoff.prediction_coverage_pct >= 96.067,
        "Markov-2 backoff coverage regressed: {}",
        report.markov_order2_backoff.prediction_coverage_pct
    );

    assert!(
        report.notification_conditioned.top1_accuracy_pct >= 63.483,
        "Notification-conditioned Top-1 regressed: {}",
        report.notification_conditioned.top1_accuracy_pct
    );
    assert!(
        report.notification_conditioned.top3_accuracy_pct >= 79.775,
        "Notification-conditioned Top-3 regressed: {}",
        report.notification_conditioned.top3_accuracy_pct
    );
    assert!(
        report.notification_conditioned.prediction_coverage_pct >= 83.146,
        "Notification-conditioned coverage regressed: {}",
        report.notification_conditioned.prediction_coverage_pct
    );
    assert!(
        report.notification_conditioned.wrong_prediction_rate_pct <= 23.649,
        "Notification-conditioned wrong-prediction rate regressed: {}",
        report.notification_conditioned.wrong_prediction_rate_pct
    );

    assert!(
        report.naive_bayes_context.top1_accuracy_pct >= 70.225,
        "Naive Bayes Top-1 regressed: {}",
        report.naive_bayes_context.top1_accuracy_pct
    );
    assert!(
        report.naive_bayes_context.top3_accuracy_pct >= 95.506,
        "Naive Bayes Top-3 regressed: {}",
        report.naive_bayes_context.top3_accuracy_pct
    );
    assert!(
        report.naive_bayes_context.prediction_coverage_pct >= 98.315,
        "Naive Bayes coverage regressed: {}",
        report.naive_bayes_context.prediction_coverage_pct
    );
    assert!(
        report.naive_bayes_context.wrong_prediction_rate_pct <= 28.571,
        "Naive Bayes wrong-prediction rate regressed: {}",
        report.naive_bayes_context.wrong_prediction_rate_pct
    );

    assert!(
        report.hybrid_markov_local.top1_accuracy_pct >= 68.539,
        "Hybrid Markov/Local Top-1 regressed: {}",
        report.hybrid_markov_local.top1_accuracy_pct
    );
    assert!(
        report.hybrid_markov_local.top3_accuracy_pct >= 93.258,
        "Hybrid Markov/Local Top-3 regressed: {}",
        report.hybrid_markov_local.top3_accuracy_pct
    );
    assert!(
        report.hybrid_markov_local.prediction_coverage_pct >= 99.438,
        "Hybrid Markov/Local coverage regressed: {}",
        report.hybrid_markov_local.prediction_coverage_pct
    );
    assert!(
        report.hybrid_markov_local.wrong_prediction_rate_pct <= 31.073,
        "Hybrid Markov/Local wrong-prediction rate regressed: {}",
        report.hybrid_markov_local.wrong_prediction_rate_pct
    );

    assert!(
        report.enhanced_local.top1_accuracy_pct >= 74.719,
        "Enhanced local Top-1 regressed: {}",
        report.enhanced_local.top1_accuracy_pct
    );
    assert!(
        report.enhanced_local.top3_accuracy_pct >= 99.438,
        "Enhanced local Top-3 regressed: {}",
        report.enhanced_local.top3_accuracy_pct
    );
    assert!(
        report.enhanced_local.prediction_coverage_pct >= 100.0,
        "Enhanced local coverage regressed: {}",
        report.enhanced_local.prediction_coverage_pct
    );
    assert!(
        report.enhanced_local.wrong_prediction_rate_pct <= 25.281,
        "Enhanced local wrong-prediction rate regressed: {}",
        report.enhanced_local.wrong_prediction_rate_pct
    );
    assert!(
        report.markov_order1.top1_accuracy_pct > report.rule_based.top1_accuracy_pct,
        "Markov-1 should beat RuleBased Top-1 on this synthetic benchmark"
    );
    assert!(
        report.markov_order1.top1_accuracy_pct > report.markov_order2_backoff.top1_accuracy_pct,
        "Markov-1 is the stronger Markov ablation on this synthetic benchmark"
    );
    assert!(
        report.hybrid_markov_local.top3_accuracy_pct > report.markov_order1.top3_accuracy_pct,
        "Hybrid should improve Top-3 over Markov-1 on this synthetic benchmark"
    );
    assert!(
        report.naive_bayes_context.top3_accuracy_pct > report.hybrid_markov_local.top3_accuracy_pct,
        "Naive Bayes should be the strongest Top-3 ablation on this synthetic benchmark"
    );
    assert!(
        report.notification_conditioned.wrong_prediction_rate_pct
            < report.markov_order1.wrong_prediction_rate_pct,
        "Notification-conditioned should have lower wrong rate when it predicts"
    );
    assert!(
        report.enhanced_local.top1_accuracy_pct > report.markov_order1.top1_accuracy_pct,
        "Enhanced local should improve Top-1 over Markov-1 on this synthetic benchmark"
    );
    assert!(
        report.enhanced_local.top3_accuracy_pct > report.naive_bayes_context.top3_accuracy_pct,
        "Enhanced local should improve Top-3 over Naive Bayes on this synthetic benchmark"
    );
    assert!(
        report.enhanced_local.prediction_coverage_pct
            > report.naive_bayes_context.prediction_coverage_pct,
        "Enhanced local should improve coverage over Naive Bayes on this synthetic benchmark"
    );
    assert!(
        report.enhanced_local.wrong_prediction_rate_pct
            < report.naive_bayes_context.wrong_prediction_rate_pct,
        "Enhanced local should keep a lower wrong rate than Naive Bayes"
    );

    let multi = report
        .scenarios
        .iter()
        .find(|scenario| scenario.scenario == "multi-app-switching")
        .expect("multi-app-switching scenario present");
    assert_eq!(multi.eligible_windows, 52);
    assert!(
        multi.local_evaluator.top3_accuracy_pct >= 88.462,
        "LocalEvaluator multi-app-switching Top-3 regressed: {}",
        multi.local_evaluator.top3_accuracy_pct
    );
}
