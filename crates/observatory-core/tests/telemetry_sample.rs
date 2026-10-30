//! What a mission contributes to the eleven exported instruments (architecture §10): the values
//! the exporter records, computed by core from the mapped events. Pure.

mod metrics_support;

use std::collections::BTreeSet;

use metrics_support::{m0_events, prices};
use observatory_core::config::{Config, OverheadApportionment};
use observatory_core::metrics::{CostInput, Notes, cost_report};
use observatory_core::model::{CycleTemplate, Timestamp};
use observatory_core::telemetry::{Label, SampleInput, SeriesPlan, mission_sample};

fn end_of(events: &[observatory_core::model::Event]) -> Timestamp {
    events
        .iter()
        .filter(|event| !event.kind.is_panel_only())
        .map(|event| event.at)
        .max()
        .expect("events")
}

#[test]
fn a_mission_sample_uses_only_contract_names_labels_and_totals_of_the_cost_report() {
    let events = m0_events();
    let cfg = Config::default().telemetry;
    let template = CycleTemplate::engineering();
    let table = prices();
    let end = end_of(&events);
    let sample = mission_sample(&SampleInput {
        mission: "m0",
        events: &events,
        template: &template,
        notes: &Notes::default(),
        prices: &table,
        end,
        cfg: &cfg,
    });
    let plan = SeriesPlan::from(&cfg);

    assert!(!sample.counters.is_empty());
    let mut series = BTreeSet::new();
    for point in sample.counters.iter().chain(&sample.observations) {
        let planned = plan
            .instruments
            .iter()
            .find(|instrument| instrument.name == point.name)
            .unwrap_or_else(|| panic!("{} is not a contract instrument", point.name));
        let keys: Vec<Label> = point.labels.iter().map(|(label, _)| *label).collect();
        assert_eq!(
            keys, planned.labels,
            "{} carries its planned labels",
            point.name
        );
        assert!(point.value >= 0.0 && point.value.is_finite());
        series.insert((point.name, point.labels.clone()));
    }
    assert!(
        series.len() < cfg.max_series as usize,
        "{} series on the M0 capture",
        series.len()
    );

    let report = cost_report(&CostInput {
        events: &events,
        template: &template,
        notes: &Notes::default(),
        prices: &table,
        end,
        budget: 0.0,
        launch_budget_usd: None,
        apportionment: OverheadApportionment::Separate,
    });
    let total = |name: &str| -> f64 {
        sample
            .counters
            .iter()
            .filter(|point| point.name == name)
            .map(|point| point.value)
            .sum()
    };
    // Roles that burn no tokens are not exported; every turn on the M0 capture is an agent's.
    assert_eq!(
        total("observatory.tokens.input"),
        report.total_tokens.input as f64
    );
    assert_eq!(
        total("observatory.tokens.output"),
        report.total_tokens.output as f64
    );
    assert_eq!(
        total("observatory.tokens.cache_read"),
        report.total_tokens.cache_read as f64
    );
    assert!((total("observatory.cost_usd") - report.cost_so_far.amount).abs() < 1e-6);
    println!(
        "M0 sample: {} counter points, {} observations, {} distinct series",
        sample.counters.len(),
        sample.observations.len(),
        series.len()
    );
}

#[test]
fn milestone_and_model_values_of_a_sample_stay_inside_their_bounds() {
    let events = m0_events();
    let cfg = Config::default().telemetry;
    let template = CycleTemplate::engineering();
    let table = prices();
    let sample = mission_sample(&SampleInput {
        mission: "m0",
        events: &events,
        template: &template,
        notes: &Notes::default(),
        prices: &table,
        end: end_of(&events),
        cfg: &cfg,
    });
    let values = |label: Label| -> BTreeSet<String> {
        sample
            .counters
            .iter()
            .chain(&sample.observations)
            .flat_map(|point| point.labels.iter())
            .filter(|(key, _)| *key == label)
            .map(|(_, value)| value.clone())
            .collect()
    };
    assert!(values(Label::Milestone).len() <= cfg.max_milestone_labels as usize);
    assert!(values(Label::Model).len() <= 5, "four named and other");
    assert_eq!(values(Label::Mission), BTreeSet::from(["m0".to_owned()]));
    for role in values(Label::Role) {
        assert!(
            ["orchestrator", "worker", "validator"].contains(&role.as_str()),
            "{role}"
        );
    }
}
