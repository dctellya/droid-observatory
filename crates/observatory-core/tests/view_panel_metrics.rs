//! The Panel's charts and usage block, and the library's headline: figures the UI draws as they
//! are, from `core::view` over the events held (FR-M5, FR-M6, FR-L4).

use std::collections::BTreeSet;
use std::path::Path;

use observatory_core::metrics::{
    CostInput, Notes, PriceTable, WasteCause, WasteGroup, cost_report,
};
use observatory_core::model::{CycleTemplate, Event, EventKind, MilestoneId, MissionId};
use observatory_core::view::{MissionRow, cost_tab, library, panel_metrics};

const PRICES: &str = include_str!("../../../config/prices/2026-10-22.toml");
const FIVE_GROUPS: [&str; 5] = [
    "Waiting on human",
    "Waiting on validation",
    "Rework",
    "Retry",
    "Idle",
];

fn golden() -> Vec<Event> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/streams/2026-10-22-observatory-v1-m0.events.json");
    let text = std::fs::read_to_string(path).expect("the golden file is committed");
    serde_json::from_str(&text).expect("the golden file is a list of events")
}

fn prices() -> PriceTable {
    PriceTable::parse(PRICES).expect("the committed price table parses")
}

fn end_of(events: &[Event]) -> observatory_core::model::Timestamp {
    events
        .iter()
        .filter(|event| !event.kind.is_panel_only())
        .map(|event| event.at)
        .max()
        .expect("events")
}

fn metrics(events: &[Event]) -> observatory_core::view::PanelMetrics {
    panel_metrics(
        events,
        &CycleTemplate::engineering(),
        &prices(),
        &Notes::default(),
        0.0,
    )
    .expect("the golden holds events")
}

#[test]
fn waste_bars_use_only_the_five_groups_and_throttled_is_apart() {
    let panel = metrics(&golden());

    assert!(!panel.waste.legend.is_empty());
    for label in &panel.waste.legend {
        assert!(
            FIVE_GROUPS.contains(label),
            "{label} is not one of the five groups"
        );
    }
    for bar in &panel.waste.bars {
        for segment in &bar.segments {
            assert!(FIVE_GROUPS.contains(&segment.label));
        }
    }
    assert_eq!(panel.waste.groups.len(), 5);
    let report_minutes = panel.throttled.mission_minutes;
    assert!(
        report_minutes > 0,
        "the M0 capture's overnight stop is measured"
    );
    assert!(
        panel
            .waste
            .legend
            .iter()
            .all(|label| !label.to_lowercase().contains("throttled")),
        "throttled is never a legend entry"
    );
}

#[test]
fn waste_bars_equal_the_cost_tab_per_milestone_and_the_tokens_in_waste_of_the_report() {
    let events = golden();
    let template = CycleTemplate::engineering();
    let panel = metrics(&events);
    let report = cost_report(&CostInput {
        events: &events,
        template: &template,
        notes: &Notes::default(),
        prices: &prices(),
        end: end_of(&events),
        budget: 0.0,
        launch_budget_usd: None,
        apportionment: observatory_core::config::OverheadApportionment::Separate,
    });

    let milestones: BTreeSet<MilestoneId> = events
        .iter()
        .filter(|event| event.feature_id.is_some())
        .filter_map(|event| event.milestone_id.clone())
        .collect();
    for milestone in &milestones {
        let tab = cost_tab(&events, milestone, &template, &prices(), &Notes::default());
        let bar = panel
            .waste
            .bars
            .iter()
            .find(|bar| bar.label == milestone.0)
            .unwrap_or_else(|| panic!("a bar for {milestone:?}"));
        for group in WasteGroup::ALL {
            let expected: u64 = tab
                .waste
                .iter()
                .filter(|row| row.cause.group() == group)
                .map(|row| row.seconds)
                .sum();
            let drawn: u64 = bar
                .segments
                .iter()
                .filter(|segment| segment.group == group)
                .map(|segment| segment.seconds)
                .sum();
            assert_eq!(drawn, expected, "{milestone:?} {group:?}");
        }
    }

    for group in &panel.waste.groups {
        let expected: u64 = WasteCause::ALL
            .into_iter()
            .filter(|cause| cause.group() == group.group)
            .map(|cause| report.tokens_in_waste_by_cause.by_cause[&cause].total())
            .sum();
        assert_eq!(group.tokens.total(), expected, "{:?}", group.group);
    }
    assert_eq!(
        panel.cost.tokens_in_waste_by_cause,
        report.tokens_in_waste_by_cause
    );
    assert_eq!(panel.cost.tokens_by_role, report.tokens_by_role);
    assert_eq!(panel.cost.tokens_by_lane, report.tokens_by_lane);
    assert_eq!(panel.cost.cost_by_role, report.cost_by_role);
    assert!(panel.throttled.mission_minutes > 0);
}

#[test]
fn the_timeline_has_one_row_per_feature_and_the_sparkline_one_path() {
    let events = golden();
    let panel = metrics(&events);
    let features: BTreeSet<_> = events
        .iter()
        .filter_map(|event| event.feature_id.clone())
        .collect();

    assert_eq!(panel.timeline.rows.len(), features.len());
    for row in &panel.timeline.rows {
        assert!(!row.bands.is_empty(), "{:?} has time states", row.feature);
        for band in &row.bands {
            assert!((0.0..=1.0).contains(&band.x));
            assert!(band.x + band.width <= 1.0 + 1e-9);
        }
    }

    assert!(panel.sparkline.points.len() >= 2);
    let costs: Vec<f64> = panel.sparkline.points.iter().map(|p| p.cost).collect();
    assert!(
        costs.windows(2).all(|pair| pair[0] <= pair[1]),
        "cumulative cost never falls"
    );
    let last = *costs.last().expect("a point");
    assert!((last - panel.cost.cost_so_far.amount).abs() < 1e-6);
    let d = panel.sparkline.path(100.0, 20.0);
    assert!(d.starts_with('M') && d.contains('L'), "{d}");
}

#[test]
fn the_headline_cost_equals_the_sum_of_the_milestone_rows_and_the_library_keeps_the_event_count() {
    let events = golden();
    let panel = metrics(&events);
    let headline = panel.headline();

    let by_milestone: f64 = panel
        .cost
        .tokens_by_milestone
        .iter()
        .filter(|row| row.milestone.is_some())
        .map(|row| row.cost.amount)
        .sum();
    assert!(
        (headline.cost - by_milestone).abs() < 0.01,
        "{} vs {by_milestone}",
        headline.cost
    );
    assert_eq!(headline.price_table, "2026-10-22");
    assert_eq!(
        headline.tokens_total + headline.overhead_tokens,
        panel.cost.total_tokens.total()
    );
    assert!((headline.cost + headline.overhead_cost - panel.cost.cost_so_far.amount).abs() < 0.01);
    assert!(headline.cycle_s > 0);

    let row = MissionRow {
        id: MissionId("m0".to_owned()),
        name: "observatory-v1-m0".to_owned(),
        state: "replay".to_owned(),
        started_at: "2026-10-22T14:54:47.240Z".to_owned(),
        completed_at: None,
        event_count: 1279,
        headline: None,
    };
    let before = library(std::slice::from_ref(&row));
    assert_eq!(before[0].event_count, 1279);
    assert!(before[0].headline.is_none());

    let with = MissionRow {
        headline: Some(headline.clone()),
        ..row
    };
    let after = library(&[with]);
    assert_eq!(after[0].event_count, 1279);
    assert_eq!(after[0].headline.as_ref(), Some(&headline));
}

#[test]
fn a_capture_with_no_event_has_no_panel_metrics() {
    let none: Vec<Event> = golden()
        .into_iter()
        .filter(|event| matches!(event.kind, EventKind::TurnUsage { .. }))
        .collect();
    assert!(
        panel_metrics(
            &none,
            &CycleTemplate::engineering(),
            &prices(),
            &Notes::default(),
            0.0
        )
        .is_none()
    );
}
