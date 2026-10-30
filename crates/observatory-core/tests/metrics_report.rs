//! `MetricsReport`: the rows of the CSV and the JSON, the engineering cycle time (F-028), the
//! mission overhead line and `throttled` per row (architecture §9).

mod metrics_support;

use metrics_support::{EventExt, ev, m0_events, prices, t, turn};
use observatory_core::config::OverheadApportionment;
use observatory_core::metrics::{
    CSV_HEADER, CostInput, CycleEnd, MetricsReport, MetricsRow, Notes, RowScope, WasteCause,
    metrics_report,
};
use observatory_core::model::{CycleTemplate, Event, EventKind, Role, Timestamp};

fn report_with(
    events: &[Event],
    end: Timestamp,
    apportionment: OverheadApportionment,
) -> MetricsReport {
    metrics_report(&CostInput {
        events,
        template: &CycleTemplate::engineering(),
        notes: &Notes::default(),
        prices: &prices(),
        end,
        budget: 0.0,
        launch_budget_usd: None,
        apportionment,
    })
}

fn report(events: &[Event], end: Timestamp) -> MetricsReport {
    report_with(events, end, OverheadApportionment::Separate)
}

fn row<'a>(report: &'a MetricsReport, scope: RowScope, id: &str) -> &'a MetricsRow {
    report
        .rows
        .iter()
        .find(|row| row.scope == scope && row.id == id)
        .unwrap_or_else(|| panic!("no {scope:?} row {id}"))
}

fn merged_pr(id: &str, seconds: i64) -> Event {
    ev(
        id,
        EventKind::HandedToCd {
            merge_sha: "abc123".to_owned(),
            pr_number: 1,
            attestation_ids: Vec::new(),
        },
        seconds,
    )
    .by(Role::Human)
}

#[test]
fn cycle_time_ends_at_handed_to_cd_and_ignores_anything_after() {
    let events = vec![
        ev("fd", EventKind::FeatureDefined, 0)
            .feature("f")
            .milestone("m1")
            .by(Role::Orchestrator),
        ev("ts", EventKind::TaskStarted, 100)
            .feature("f")
            .milestone("m1")
            .lasting(50),
        ev("done", EventKind::TaskDone, 600)
            .feature("f")
            .milestone("m1"),
        merged_pr("pr", 1000).milestone("m1"),
        ev("late", EventKind::FileTouched, 5000)
            .feature("f")
            .milestone("m1")
            .lasting(10),
    ];

    let found = report(&events, t(5000));

    let feature = row(&found, RowScope::Feature, "f");
    assert_eq!(feature.cycle_s, 1000, "feature.defined to handed_to_cd");
    assert_eq!(feature.cycle.ends_at, CycleEnd::HandedToCd);
    assert!(!feature.cycle.fallback);
    assert_eq!(row(&found, RowScope::Milestone, "m1").cycle_s, 1000);
    assert_eq!(found.mission.cycle.seconds, 1000);
    assert_eq!(found.mission.cycle.ends_at, CycleEnd::HandedToCd);
    assert_eq!(
        found.ignored_after_handed_to_cd, 1,
        "what follows the hand-over is counted, never read"
    );
    assert!(
        feature.waste_s[&WasteCause::IdleUnscheduled] < 1000,
        "no idle time is counted past the hand-over"
    );
}

#[test]
fn cycle_time_falls_back_to_first_task_started_and_counts_the_fallback() {
    let events = vec![
        ev("fd", EventKind::FeatureDefined, 0)
            .feature("f1")
            .by(Role::Orchestrator),
        ev("a", EventKind::TaskStarted, 50)
            .feature("f1")
            .lasting(10),
        ev("b", EventKind::TaskStarted, 200)
            .feature("f2")
            .lasting(10),
    ];

    let found = report(&events, t(1000));

    let defined = row(&found, RowScope::Feature, "f1");
    assert_eq!(defined.cycle_s, 1000);
    assert_eq!(
        defined.cycle.ends_at,
        CycleEnd::Open,
        "no handed_to_cd yet: reads now"
    );
    assert!(!defined.cycle.fallback);
    let fallback = row(&found, RowScope::Feature, "f2");
    assert_eq!(fallback.cycle_s, 800, "first task.started is the start");
    assert!(fallback.cycle.fallback);
    assert_eq!(found.mission.features, 2);
    assert_eq!(found.mission.fallback_features, 1);
}

#[test]
fn a_milestone_cycle_ends_at_its_integration_while_no_pr_exists() {
    let events = vec![
        ev("fd", EventKind::FeatureDefined, 10)
            .feature("f")
            .milestone("m1")
            .by(Role::Orchestrator),
        ev("ts", EventKind::TaskStarted, 100)
            .feature("f")
            .milestone("m1")
            .lasting(10),
        ev("merged", EventKind::IntegrationMerged, 700).milestone("m1"),
        ev("tail", EventKind::FileTouched, 900)
            .feature("f")
            .milestone("m1")
            .lasting(10),
    ];

    let found = report(&events, t(1000));

    let milestone = row(&found, RowScope::Milestone, "m1");
    assert_eq!(milestone.cycle_s, 690);
    assert_eq!(milestone.cycle.ends_at, CycleEnd::Integration);
    assert_eq!(
        row(&found, RowScope::Feature, "f").cycle.ends_at,
        CycleEnd::Open,
        "a feature's clock stops only at handed_to_cd"
    );
}

fn overhead_events() -> Vec<Event> {
    vec![
        ev("a", EventKind::FileTouched, 100)
            .feature("f1")
            .milestone("m1")
            .lasting(10),
        ev("b", EventKind::FileTouched, 200)
            .feature("f2")
            .milestone("m1")
            .lasting(10),
        ev("c", EventKind::PlanRevised, 300)
            .by(Role::Orchestrator)
            .lasting(60),
        turn(
            "t1",
            150,
            Role::Orchestrator,
            "claude-fable-5.1",
            "plan",
            [1_000_000, 0, 0],
        ),
        turn(
            "t2",
            160,
            Role::Worker,
            "claude-sonnet-5-5",
            "build",
            [7, 0, 0],
        )
        .feature("f1")
        .milestone("m1"),
    ]
}

#[test]
fn overhead_is_reported_separately_unless_apportionment_is_equal() {
    let events = overhead_events();

    let separate = report_with(&events, t(400), OverheadApportionment::Separate);
    let equal = report_with(&events, t(400), OverheadApportionment::Equal);

    assert_eq!(separate.overhead.label, "separate");
    assert!((separate.overhead.cost.amount - 10.0).abs() < 1e-9);
    assert!((separate.overhead.active_s - 60.0).abs() < 1e-9);
    let f1 = row(&separate, RowScope::Feature, "f1");
    assert_eq!(
        f1.first_pass_active_s, 10,
        "overhead is in no feature's columns"
    );
    assert_eq!(f1.tokens_input, 7);
    assert!(f1.cost_usd < 0.001);

    assert_eq!(equal.overhead.label, "apportioned");
    assert!(
        (equal.overhead.active_s - separate.overhead.active_s).abs() < 1e-9,
        "the overhead seconds are the same in both"
    );
    let (e1, e2) = (
        row(&equal, RowScope::Feature, "f1"),
        row(&equal, RowScope::Feature, "f2"),
    );
    assert_eq!(
        e1.first_pass_active_s, 40,
        "10 s of its own and half of 60 s"
    );
    assert_eq!(e2.first_pass_active_s, 40);
    assert_eq!(e1.tokens_input, 7 + 500_000);
    assert_eq!(e2.tokens_input, 500_000);
    assert!((e1.cost_usd - 5.0).abs() < 1e-3 && (e2.cost_usd - 5.0).abs() < 1e-3);
    let milestone = row(&equal, RowScope::Milestone, "m1");
    assert_eq!(
        milestone.first_pass_active_s,
        e1.first_pass_active_s + e2.first_pass_active_s
    );
    let given: u64 = equal
        .rows
        .iter()
        .filter(|r| r.scope == RowScope::Feature)
        .map(|r| r.first_pass_active_s)
        .sum::<u64>()
        - separate
            .rows
            .iter()
            .filter(|r| r.scope == RowScope::Feature)
            .map(|r| r.first_pass_active_s)
            .sum::<u64>();
    assert_eq!(given, 60, "all of the overhead went to the features");
}

fn question_events() -> Vec<Event> {
    vec![
        ev("fd", EventKind::FeatureDefined, 0)
            .feature("f")
            .milestone("m1")
            .by(Role::Orchestrator),
        ev("start", EventKind::TaskStarted, 20)
            .feature("f")
            .milestone("m1")
            .lasting(20),
        ev("ask", EventKind::QuestionAsked, 100)
            .feature("f")
            .milestone("m1")
            .by(Role::Orchestrator),
        ev("answer", EventKind::IntentAnswered, 400)
            .feature("f")
            .milestone("m1")
            .by(Role::Human),
        ev("work", EventKind::FileTouched, 500)
            .feature("f")
            .milestone("m1")
            .lasting(60),
    ]
}

#[test]
fn stage_times_and_bottleneck_follow_the_template_lanes() {
    let template = CycleTemplate::engineering();
    let events = question_events();

    let found = report(&events, t(600));

    let feature = row(&found, RowScope::Feature, "f");
    assert_eq!(feature.stage_s.len(), template.lanes.len());
    let lanes: Vec<&str> = feature.stage_s.iter().map(|s| s.lane.as_str()).collect();
    let ids: Vec<&str> = template.lanes.iter().map(|l| l.id.as_str()).collect();
    assert_eq!(lanes, ids, "one stage per lane, in the template's order");
    let build = template.lane_of(&EventKind::TaskStarted, Role::Worker);
    let stage = feature.stage_s.iter().find(|s| s.lane == build).unwrap();
    assert_eq!(
        stage.seconds, 480,
        "first to last event in the lane: {stage:?}"
    );
    let bottleneck = feature.bottleneck.as_ref().expect("the feature waited");
    assert_eq!(
        bottleneck.lane,
        template.lane_of(&EventKind::QuestionAsked, Role::Orchestrator)
    );
    assert_eq!(bottleneck.waiting_s, 300);
    assert_eq!(
        found.mission.bottleneck.as_ref().unwrap().lane,
        bottleneck.lane
    );
}

fn split(line: &str) -> Vec<&str> {
    line.split(',').collect()
}

#[test]
fn the_csv_has_the_column_contract_and_a_row_per_feature_and_milestone() {
    let found = report(&question_events(), t(600));

    let csv = found.to_csv();
    let mut lines = csv.lines();
    let header = lines.next().unwrap();
    assert_eq!(header, CSV_HEADER.join(","));
    assert_eq!(CSV_HEADER.len(), 22);
    assert_eq!(
        CSV_HEADER
            .iter()
            .filter(|c| c.starts_with("waste_"))
            .count(),
        8
    );
    let rows: Vec<Vec<&str>> = lines.map(split).collect();
    assert_eq!(rows.len(), 2, "one feature, one milestone");
    let column = |name: &str| CSV_HEADER.iter().position(|c| *c == name).unwrap();
    let feature = rows.iter().find(|r| r[0] == "feature").unwrap();
    assert_eq!(feature[1], "f");
    assert_eq!(feature[column("cycle_s")], "600");
    assert_eq!(feature[column("waste_waiting_human_question_s")], "300");
    assert_eq!(feature[column("price_table")], "2026-10-22");
    assert!(feature[column("cost_usd")].parse::<f64>().is_ok());
    let milestone = rows.iter().find(|r| r[0] == "milestone").unwrap();
    assert_eq!(milestone[1], "m1");
    assert_eq!(milestone[column("waste_waiting_human_question_s")], "300");
    for r in &rows {
        for name in [
            "first_pass_active_s",
            "rework_s",
            "rework_windows",
            "throttled_minutes",
            "tokens_input",
            "tokens_output",
            "tokens_cache_read",
            "tokens_cache_write",
        ] {
            assert!(
                r[column(name)].parse::<u64>().is_ok(),
                "{name}: {}",
                r[column(name)]
            );
        }
    }
}

#[test]
fn a_csv_name_with_a_comma_or_quote_is_quoted_by_rfc_4180() {
    let mut found = report(&question_events(), t(600));
    found.rows[0].name = "plan, \"v2\"".to_owned();

    let csv = found.to_csv();

    assert!(csv.lines().nth(1).unwrap().contains("\"plan, \"\"v2\"\"\""));
}

#[test]
fn the_json_rows_equal_the_csv_rows_and_carry_overhead_and_throttled() {
    let found = report(&overhead_events(), t(400));
    let json = serde_json::to_value(&found).unwrap();

    let rows = json["rows"].as_array().unwrap();
    let csv = found.to_csv();
    let mut compared = 0;
    for line in csv.lines().skip(1) {
        let cells = split(line);
        let counterpart = rows
            .iter()
            .find(|r| r["scope"] == cells[0] && r["id"] == cells[1])
            .expect("every csv row has a json row");
        for (name, cell) in CSV_HEADER.iter().zip(&cells).skip(3) {
            if name.starts_with("waste_") {
                let cause = name.trim_start_matches("waste_").trim_end_matches("_s");
                assert_eq!(counterpart["waste_s"][cause].to_string(), *cell, "{name}");
            } else if *name == "price_table" {
                assert_eq!(counterpart[name], *cell);
            } else if *name == "cost_usd" {
                let value = counterpart[name].as_f64().unwrap();
                assert!((value - cell.parse::<f64>().unwrap()).abs() < 1e-9);
            } else {
                assert_eq!(counterpart[name].to_string(), *cell, "{name}");
            }
        }
        compared += 1;
    }
    assert_eq!(compared, rows.len());
    assert!(
        json["overhead"].is_object(),
        "the overhead line sits beside the rows"
    );
    assert_eq!(json["throttled"]["mission_minutes"], 0);
    assert!(
        json["throttled"]["intervals"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(json["price_table"], "2026-10-22");
}

#[test]
fn throttled_minutes_per_row_is_the_overlap_with_its_cycle_and_apart_from_waste() {
    let events = vec![
        ev("fd", EventKind::FeatureDefined, 0)
            .feature("f")
            .milestone("m1")
            .by(Role::Orchestrator),
        ev("work", EventKind::FileTouched, 100)
            .feature("f")
            .milestone("m1")
            .lasting(50),
        ev("stop", EventKind::MissionPaused, 200).by(Role::Human),
        ev("go", EventKind::MissionResumed, 1400).by(Role::Human),
        ev("again", EventKind::FileTouched, 1500)
            .feature("f")
            .milestone("m1")
            .lasting(50),
    ];

    let found = report(&events, t(1500));

    assert_eq!(found.throttled.intervals.len(), 1);
    assert_eq!(found.throttled.mission_minutes, 20);
    assert_eq!(found.mission.throttled_minutes, 20);
    for scope in [RowScope::Feature, RowScope::Milestone] {
        let id = if scope == RowScope::Feature {
            "f"
        } else {
            "m1"
        };
        let one = row(&found, scope, id);
        assert_eq!(one.throttled_minutes, 20);
        let waste: u64 = WasteCause::ALL
            .into_iter()
            .filter(|cause| *cause != WasteCause::Rework)
            .map(|cause| one.waste_s[&cause])
            .sum();
        assert!(
            one.first_pass_active_s + one.rework_s + waste + 60 * one.throttled_minutes
                <= one.cycle_s,
            "{one:?}"
        );
    }
}

#[test]
fn the_m0_rows_cover_every_feature_and_milestone_and_tokens_add_up() {
    let events = m0_events();
    let end = events
        .iter()
        .filter(|e| !e.kind.is_panel_only())
        .map(|e| e.at)
        .max()
        .unwrap();

    let found = report(&events, end);

    let features: std::collections::BTreeSet<_> =
        events.iter().filter_map(|e| e.feature_id.clone()).collect();
    let milestones: std::collections::BTreeSet<_> = events
        .iter()
        .filter_map(|e| e.milestone_id.clone())
        .collect();
    let count = |scope| found.rows.iter().filter(|r| r.scope == scope).count();
    assert_eq!(count(RowScope::Feature), features.len());
    assert_eq!(count(RowScope::Milestone), milestones.len());
    let by_feature = |pick: fn(&MetricsRow) -> u64| -> u64 {
        found
            .rows
            .iter()
            .filter(|r| r.scope == RowScope::Feature)
            .map(pick)
            .sum()
    };
    assert_eq!(
        by_feature(|r| r.tokens_input) + found.overhead.tokens.input,
        found.total_tokens.input
    );
    assert_eq!(
        by_feature(|r| r.tokens_output) + found.overhead.tokens.output,
        found.total_tokens.output
    );
    assert_eq!(
        found.throttled.mission_minutes,
        found.mission.throttled_minutes
    );
    assert!(found.mission.throttled_minutes > 0, "the overnight stop");
    assert_eq!(found.mission.features, features.len());
    assert!(found.rows.iter().all(|r| r.price_table == "2026-10-22"));
}
