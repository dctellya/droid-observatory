//! The Panel, the Details Cost tab and the library headline read the figures of
//! `observatory-cli metrics --csv` for the same scope (VAL-CROSS-003, FR-M1 to FR-M7).

mod metrics_support;

use metrics_support::{EventExt, cache_write, ev, m0_events, prices, turn, usage_report};
use observatory_core::config::OverheadApportionment;
use observatory_core::metrics::{
    MetricsReport, Notes, ReportSetup, RowScope, WasteCause, report_at_last_event,
};
use observatory_core::model::{CycleTemplate, EventKind, MilestoneId, Role};
use observatory_core::view::{cost_tab, panel_metrics};

fn report() -> MetricsReport {
    let template = CycleTemplate::engineering();
    report_at_last_event(
        &m0_events(),
        &ReportSetup {
            template: &template,
            prices: &prices(),
            notes: &Notes::default(),
            budget: 0.0,
            launch_budget_usd: None,
            apportionment: OverheadApportionment::Separate,
        },
    )
    .expect("the M0 capture holds events")
}

#[test]
fn the_cost_tab_of_each_milestone_equals_its_csv_row() {
    let report = report();
    let template = CycleTemplate::engineering();
    let events = m0_events();
    let milestones: Vec<_> = report
        .rows
        .iter()
        .filter(|row| row.scope == RowScope::Milestone)
        .collect();
    assert!(milestones.len() >= 2, "M0 has the milestones m0 and s");
    for row in milestones {
        let tab = cost_tab(
            &events,
            &MilestoneId(row.id.clone()),
            &template,
            &prices(),
            &Notes::default(),
        );
        assert_eq!(tab.cycle_s, row.cycle_s, "{} cycle_s", row.id);
        assert_eq!(
            tab.rework_windows, row.rework_windows,
            "{} rework_windows",
            row.id
        );
        for waste in &tab.waste {
            assert_eq!(
                Some(&waste.seconds),
                row.waste_s.get(&waste.cause),
                "{} {}",
                row.id,
                waste.name
            );
        }
        assert_eq!(tab.waste.len(), WasteCause::ALL.len());
        assert_eq!(tab.totals.tokens.input, row.tokens_input, "{}", row.id);
        assert_eq!(tab.totals.tokens.output, row.tokens_output, "{}", row.id);
        assert_eq!(
            tab.totals.tokens.cache_read, row.tokens_cache_read,
            "{}",
            row.id
        );
        assert_eq!(tab.totals.cache_write, row.tokens_cache_write, "{}", row.id);
        assert!(
            (tab.totals.cost.amount - row.cost_usd).abs() < 0.005,
            "{}: {} vs {}",
            row.id,
            tab.totals.cost.amount,
            row.cost_usd
        );
        assert_eq!(tab.price_table, row.price_table);
    }
}

#[test]
fn the_panel_milestone_figures_and_waste_bars_equal_the_csv_rows() {
    let report = report();
    let panel = panel_metrics(
        &m0_events(),
        &CycleTemplate::engineering(),
        &prices(),
        &Notes::default(),
        0.0,
    )
    .expect("the M0 capture holds events");
    let milestones: Vec<_> = report
        .rows
        .iter()
        .filter(|row| row.scope == RowScope::Milestone)
        .collect();
    assert_eq!(panel.milestones.len(), milestones.len());
    for row in &milestones {
        let figures = panel
            .milestones
            .iter()
            .find(|figures| figures.milestone.0 == row.id)
            .unwrap_or_else(|| panic!("no Panel figures for {}", row.id));
        assert_eq!(figures.cycle_s, row.cycle_s, "{}", row.id);
        assert_eq!(figures.rework_windows, row.rework_windows, "{}", row.id);
        assert_eq!(
            figures.throttled_minutes, row.throttled_minutes,
            "{}",
            row.id
        );
        let bar = panel
            .waste
            .bars
            .iter()
            .find(|bar| bar.label == row.id)
            .unwrap_or_else(|| panic!("no waste bar for {}", row.id));
        assert_eq!(bar.total_s, row.waste_s.values().sum::<u64>(), "{}", row.id);
    }
    let run = &panel.waste.bars[0];
    let features: u64 = report
        .rows
        .iter()
        .filter(|row| row.scope == RowScope::Feature)
        .map(|row| row.waste_s.values().sum::<u64>())
        .sum();
    assert_eq!(run.total_s, features);
    assert_eq!(run.total_s, report.mission.waste_s.values().sum::<u64>());
}

#[test]
fn the_headline_reads_the_engineering_clock_and_the_csv_waste() {
    let report = report();
    let panel = panel_metrics(
        &m0_events(),
        &CycleTemplate::engineering(),
        &prices(),
        &Notes::default(),
        0.0,
    )
    .expect("the M0 capture holds events");
    let headline = panel.headline();
    assert_eq!(headline.cycle_s, report.mission.cycle.seconds);
    assert_eq!(
        headline.waste_s,
        report.mission.waste_s.values().sum::<u64>()
    );
    let milestone_cost: f64 = report
        .rows
        .iter()
        .filter(|row| row.scope == RowScope::Milestone)
        .map(|row| row.cost_usd)
        .sum();
    assert!((headline.cost - milestone_cost).abs() < 0.01);
}

#[test]
fn the_cost_tab_the_rows_and_the_panel_headline_name_the_same_cost_with_cache_write() {
    let events = vec![
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
            [1_000_000, 100_000, 2_000_000],
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
    ];
    let template = CycleTemplate::engineering();
    // 1 M input at 2.0, 0.1 M output at 10.0, 2 M cache read at 0.10, 0.4 M cache write at 2.5.
    let by_hand = 2.0 + 1.0 + 0.2 + 1.0;

    let report = report_at_last_event(
        &events,
        &ReportSetup {
            template: &template,
            prices: &prices(),
            notes: &Notes::default(),
            budget: 0.0,
            launch_budget_usd: None,
            apportionment: OverheadApportionment::Separate,
        },
    )
    .expect("the events are real");
    let row = report
        .rows
        .iter()
        .find(|row| row.scope == RowScope::Milestone && row.id == "m1")
        .expect("the milestone row");
    let tab = cost_tab(
        &events,
        &MilestoneId("m1".to_owned()),
        &template,
        &prices(),
        &Notes::default(),
    );
    let panel = panel_metrics(&events, &template, &prices(), &Notes::default(), 0.0)
        .expect("the events are real");

    assert!((row.cost_usd - by_hand).abs() < 0.01, "{}", row.cost_usd);
    assert!(
        (tab.totals.cost.amount - row.cost_usd).abs() < 0.005,
        "tab {} row {}",
        tab.totals.cost.amount,
        row.cost_usd
    );
    assert_eq!(tab.totals.cache_write, row.tokens_cache_write);
    let feature_costs: f64 = tab.features.iter().map(|f| f.cost.amount).sum();
    assert!((feature_costs - row.cost_usd).abs() < 0.005);
    let role_costs: f64 = tab.roles.iter().map(|r| r.cost.amount).sum();
    assert!((role_costs - row.cost_usd).abs() < 0.005);
    assert!((panel.headline().cost - row.cost_usd).abs() < 0.005);
    assert!(
        (panel.sparkline.total - panel.cost.cost_so_far.amount).abs() < 1e-9,
        "the cumulative cost line ends at the cost so far"
    );
    assert!((panel.cost.cost_so_far.amount - row.cost_usd).abs() < 0.005);
}
