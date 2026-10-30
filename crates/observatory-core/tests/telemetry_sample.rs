//! What a mission contributes to the eleven exported instruments (architecture §10): the values
//! the exporter records, computed by core from the mapped events. Pure.

mod metrics_support;

use std::collections::BTreeSet;

use metrics_support::{EventExt, cache_write, ev, m0_events, prices, t, turn, usage_report};
use observatory_core::config::{Config, OverheadApportionment};
use observatory_core::metrics::{CostInput, Notes, cost_report};
use observatory_core::model::{CycleTemplate, Event, EventKind, Role, Timestamp};
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

fn cache_write_events() -> Vec<Event> {
    vec![
        ev("fd", EventKind::FeatureDefined, 0)
            .feature("f1")
            .milestone("m1")
            .by(Role::Orchestrator),
        turn(
            "t1",
            10,
            Role::Worker,
            "claude-sonnet-5-5",
            "build",
            [1_000_000, 0, 0],
        )
        .feature("f1")
        .milestone("m1"),
        usage_report(
            "u1",
            20,
            Role::Worker,
            "claude-sonnet-5-5",
            "build",
            cache_write(400_000),
        )
        .feature("f1")
        .milestone("m1"),
        usage_report(
            "u2",
            30,
            Role::ScrutinyValidator,
            "claude-opus-5-5",
            "scrutiny",
            cache_write(100_000),
        )
        .feature("f1")
        .milestone("m1"),
    ]
}

fn sample_of(events: &[Event]) -> observatory_core::telemetry::MissionSample {
    let cfg = Config::default().telemetry;
    mission_sample(&SampleInput {
        mission: "m0",
        events,
        template: &CycleTemplate::engineering(),
        notes: &Notes::default(),
        prices: &prices(),
        end: t(40),
        cfg: &cfg,
    })
}

fn value_of(
    sample: &observatory_core::telemetry::MissionSample,
    name: &str,
    role: &str,
    model: &str,
) -> Option<f64> {
    sample
        .counters
        .iter()
        .find(|point| {
            point.name == name
                && point.labels.contains(&(Label::Role, role.to_owned()))
                && point.labels.contains(&(Label::Model, model.to_owned()))
        })
        .map(|point| point.value)
}

#[test]
fn the_telemetry_sample_carries_cache_write_tokens_per_role_and_model() {
    let sample = sample_of(&cache_write_events());

    let name = "observatory.tokens.cache_write";
    assert_eq!(
        value_of(&sample, name, "worker", "claude-sonnet-5-5"),
        Some(400_000.0)
    );
    assert_eq!(
        value_of(&sample, name, "validator", "claude-opus-5-5"),
        Some(100_000.0)
    );
    let plan = SeriesPlan::from(&Config::default().telemetry);
    let planned = plan
        .instruments
        .iter()
        .find(|instrument| instrument.name == name)
        .expect("planned");
    for point in sample.counters.iter().filter(|point| point.name == name) {
        let keys: Vec<Label> = point.labels.iter().map(|(label, _)| *label).collect();
        assert_eq!(keys, planned.labels, "the planned labels");
    }
}

#[test]
fn cost_usd_in_the_telemetry_sample_includes_the_cache_write_term() {
    let sample = sample_of(&cache_write_events());

    // Worker: 1 M input at 2.0 and 0.4 M cache write at 2.5; validator: 0.1 M cache write at 5.0.
    let name = "observatory.cost_usd";
    let worker = value_of(&sample, name, "worker", "claude-sonnet-5-5").expect("worker point");
    assert!((worker - 3.0).abs() < 1e-9, "{worker}");
    let validator =
        value_of(&sample, name, "validator", "claude-opus-5-5").expect("validator point");
    assert!((validator - 0.5).abs() < 1e-9, "{validator}");
}

#[test]
fn the_m0_sample_cache_write_total_is_the_sum_of_the_session_reports() {
    let events = m0_events();
    let cfg = Config::default().telemetry;
    let sample = mission_sample(&SampleInput {
        mission: "m0",
        events: &events,
        template: &CycleTemplate::engineering(),
        notes: &Notes::default(),
        prices: &prices(),
        end: end_of(&events),
        cfg: &cfg,
    });
    let reported: u64 = events
        .iter()
        .filter(|event| event.kind == EventKind::UsageReported)
        .filter_map(|event| event.usage)
        .map(|usage| usage.cache_write)
        .sum();
    let exported: f64 = sample
        .counters
        .iter()
        .filter(|point| point.name == "observatory.tokens.cache_write")
        .map(|point| point.value)
        .sum();
    assert!(reported > 0);
    assert_eq!(exported, reported as f64);
}
