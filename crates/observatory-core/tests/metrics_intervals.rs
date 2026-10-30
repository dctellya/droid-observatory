//! `MetricsReport.intervals`: the partition of every card as the JSON and the Tauri `metrics`
//! command carry it, so an interval that ends at the answer's instant can be read from the report.

mod metrics_support;

use metrics_support::{EventExt, ev, prices, t};
use observatory_core::config::OverheadApportionment;
use observatory_core::metrics::{
    CSV_HEADER, CostInput, MetricsReport, Notes, WasteCause, metrics_report,
};
use observatory_core::model::{CycleTemplate, Event, EventKind, Role, Timestamp};
use serde_json::{Value, json};

fn report(events: &[Event], end: Timestamp) -> MetricsReport {
    metrics_report(&CostInput {
        events,
        template: &CycleTemplate::engineering(),
        notes: &Notes::default(),
        prices: &prices(),
        end,
        budget: 0.0,
        launch_budget_usd: None,
        apportionment: OverheadApportionment::Separate,
    })
}

fn stamp(seconds: i64) -> String {
    t(seconds).to_rfc3339_millis()
}

fn card<'a>(json: &'a Value, feature: Option<&str>) -> &'a Value {
    json["intervals"]
        .as_array()
        .expect("the report lists the intervals")
        .iter()
        .find(|card| card["feature"] == json!(feature))
        .unwrap_or_else(|| panic!("no card for {feature:?}"))
}

/// A hosted orchestrator question that names no feature, beside a feature that works.
fn hosted_question() -> Vec<Event> {
    vec![
        ev("plan", EventKind::FileTouched, 20).lasting(20),
        ev("ask", EventKind::QuestionAsked, 100).by(Role::Orchestrator),
        ev("answer", EventKind::IntentAnswered, 400).by(Role::Human),
        ev("next", EventKind::FileTouched, 500).lasting(60),
        ev("fd", EventKind::FeatureDefined, 0)
            .feature("f")
            .milestone("m1")
            .by(Role::Orchestrator),
        ev("work", EventKind::FileTouched, 300)
            .feature("f")
            .milestone("m1")
            .lasting(100),
    ]
}

#[test]
fn the_report_lists_the_waiting_human_question_interval_of_the_mission_level_card() {
    let json = serde_json::to_value(report(&hosted_question(), t(600))).unwrap();

    let level = card(&json, None);
    let waiting = level["intervals"]
        .as_array()
        .unwrap()
        .iter()
        .find(|interval| interval["state"] == json!({"waiting_human": "question"}))
        .unwrap_or_else(|| panic!("no question interval: {level}"));

    assert_eq!(waiting["from"], json!(stamp(100)), "from the ask");
    assert_eq!(waiting["to"], json!(stamp(400)), "to the answer's `at`");
    assert_eq!(waiting["cause"], json!("waiting_human_question"));
    assert_eq!(level["milestone"], Value::Null);
}

#[test]
fn the_report_lists_the_intervals_of_every_feature_card_in_partition_order() {
    let events = vec![
        ev("fd-a", EventKind::FeatureDefined, 0)
            .feature("a")
            .milestone("m1")
            .by(Role::Orchestrator),
        ev("a1", EventKind::FileTouched, 100)
            .feature("a")
            .milestone("m1")
            .lasting(50),
        ev("a2", EventKind::FileTouched, 300)
            .feature("a")
            .milestone("m1")
            .lasting(50),
        ev("fd-b", EventKind::FeatureDefined, 0)
            .feature("b")
            .milestone("m2")
            .by(Role::Orchestrator),
        ev("b1", EventKind::FileTouched, 200)
            .feature("b")
            .milestone("m2")
            .lasting(80),
    ];
    let json = serde_json::to_value(report(&events, t(400))).unwrap();

    let cards = json["intervals"].as_array().unwrap();
    let features: Vec<&Value> = cards.iter().map(|card| &card["feature"]).collect();
    assert_eq!(
        features,
        [&Value::Null, &json!("a"), &json!("b")],
        "the mission-level card first, then the features by id"
    );
    assert_eq!(card(&json, Some("a"))["milestone"], json!("m1"));
    assert_eq!(card(&json, Some("b"))["milestone"], json!("m2"));
    assert!(
        card(&json, None)["intervals"]
            .as_array()
            .unwrap()
            .is_empty(),
        "no event names no feature, so the mission-level partition is empty"
    );
    for feature in ["a", "b"] {
        let intervals = card(&json, Some(feature))["intervals"].as_array().unwrap();
        assert!(!intervals.is_empty(), "{feature} has a partition");
        for pair in intervals.windows(2) {
            assert_eq!(pair[0]["to"], pair[1]["from"], "{feature}: contiguous");
        }
        assert_eq!(
            intervals.last().unwrap()["to"],
            json!(stamp(400)),
            "{feature}: the partition runs to the end"
        );
    }
}

#[test]
fn the_json_intervals_carry_the_waste_cause_of_each_state() {
    let events = vec![
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
        ev("finding", EventKind::FindingBlocking, 450)
            .feature("f")
            .milestone("m1")
            .by(Role::ScrutinyValidator),
        ev("resolved", EventKind::ReworkResolved, 500)
            .feature("f")
            .milestone("m1")
            .by(Role::ScrutinyValidator),
        ev("work", EventKind::FileTouched, 560)
            .feature("f")
            .milestone("m1")
            .lasting(60),
    ];
    let found = report(&events, t(600));
    let json = serde_json::to_value(&found).unwrap();

    let own = card(&json, Some("f"));
    let intervals = own["intervals"].as_array().unwrap();
    let mut seconds: std::collections::BTreeMap<String, i64> = Default::default();
    for interval in intervals {
        let state = &interval["state"];
        let cause = &interval["cause"];
        if state == "active" {
            assert_eq!(cause, &Value::Null, "active time is not waste: {interval}");
        } else {
            assert!(cause.is_string(), "a waiting state has a cause: {interval}");
            let from = Timestamp::parse(interval["from"].as_str().unwrap()).unwrap();
            let to = Timestamp::parse(interval["to"].as_str().unwrap()).unwrap();
            *seconds
                .entry(cause.as_str().unwrap().to_owned())
                .or_default() += (to.0 - from.0).whole_seconds();
        }
    }
    let feature = found
        .rows
        .iter()
        .find(|row| row.id == "f")
        .expect("the feature row");
    for cause in WasteCause::ALL {
        if cause == WasteCause::Rework {
            continue;
        }
        assert_eq!(
            seconds.get(cause.name()).copied().unwrap_or(0),
            i64::try_from(feature.waste_s[&cause]).unwrap(),
            "{}: the intervals add up to the row",
            cause.name()
        );
    }

    let rework = own["rework"].as_array().unwrap();
    assert_eq!(rework.len(), 1, "the finding opened one window: {own}");
    assert_eq!(rework[0]["kind"], json!("finding"));
    assert_eq!(rework[0]["from"], json!(stamp(450)));
    assert_eq!(rework[0]["to"], json!(stamp(500)));
}

/// Characterization: the interval list is JSON only, so the CSV stays as it was (the header
/// has 22 names and a row per feature and milestone). It cannot go red before the change.
#[test]
fn the_csv_header_and_rows_do_not_change_with_the_interval_list() {
    let found = report(&hosted_question(), t(600));

    let csv = found.to_csv();
    let mut lines = csv.lines();
    assert_eq!(lines.next().unwrap(), CSV_HEADER.join(","));
    assert_eq!(CSV_HEADER.len(), 22);
    let rows: Vec<&str> = lines.collect();
    assert_eq!(
        rows.len(),
        found.rows.len(),
        "a row per feature and milestone"
    );
    assert!(
        !csv.contains("interval"),
        "the interval list is not a CSV column: {csv}"
    );
}
