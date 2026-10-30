//! The oracle diff: the engine's CSV against a reference CSV within 1 s, 1 token and 0.01 USD
//! (architecture §9, SPEC §3 M5). The reference spreadsheet is the owner's; these tests prove
//! the comparison itself on a CSV the engine produced.

mod metrics_support;

use metrics_support::{EventExt, ev, prices, t};
use observatory_core::config::OverheadApportionment;
use observatory_core::metrics::{
    CSV_HEADER, CostInput, Deviation, Notes, OracleError, Tolerance, compare_csv, metrics_report,
};
use observatory_core::model::{CycleTemplate, EventKind, Role};

fn produced() -> String {
    let events = vec![
        ev("fd", EventKind::FeatureDefined, 0)
            .feature("f")
            .milestone("m1")
            .by(Role::Orchestrator),
        ev("work", EventKind::FileTouched, 100)
            .feature("f")
            .milestone("m1")
            .lasting(60),
        ev("ask", EventKind::QuestionAsked, 200)
            .feature("f")
            .milestone("m1")
            .by(Role::Orchestrator),
        ev("answer", EventKind::IntentAnswered, 500)
            .feature("f")
            .milestone("m1")
            .by(Role::Human),
    ];
    metrics_report(&CostInput {
        events: &events,
        template: &CycleTemplate::engineering(),
        notes: &Notes::default(),
        prices: &prices(),
        end: t(600),
        budget: 0.0,
        launch_budget_usd: None,
        apportionment: OverheadApportionment::Separate,
    })
    .to_csv()
}

/// `csv` with `column` of the row `(scope, id)` replaced by `value`.
fn with_cell(csv: &str, scope: &str, id: &str, column: &str, value: &str) -> String {
    let at = CSV_HEADER.iter().position(|c| *c == column).unwrap();
    csv.lines()
        .map(|line| {
            let mut cells: Vec<String> = line.split(',').map(str::to_owned).collect();
            if cells[0] == scope && cells[1] == id {
                cells[at] = value.to_owned();
            }
            cells.join(",")
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

fn cell(csv: &str, scope: &str, id: &str, column: &str) -> String {
    let at = CSV_HEADER.iter().position(|c| *c == column).unwrap();
    csv.lines()
        .map(|line| line.split(',').collect::<Vec<_>>())
        .find(|cells| cells[0] == scope && cells[1] == id)
        .map(|cells| cells[at].to_owned())
        .unwrap()
}

#[test]
fn the_oracle_accepts_the_engines_own_csv_and_counts_the_rows_compared() {
    let csv = produced();

    let diff = compare_csv(&csv, &csv, &Tolerance::default()).unwrap();

    assert!(diff.deviations.is_empty(), "{:?}", diff.deviations);
    assert_eq!(
        diff.rows_compared, 2,
        "one feature row and one milestone row"
    );
    assert!(diff.max_by_column.values().all(|max| *max == 0.0));
}

#[test]
fn the_oracle_accepts_a_cell_inside_the_tolerance_and_names_each_one_outside() {
    let csv = produced();
    let cycle: u64 = cell(&csv, "feature", "f", "cycle_s").parse().unwrap();
    let inside = with_cell(&csv, "feature", "f", "cycle_s", &(cycle + 1).to_string());
    let inside = with_cell(&inside, "feature", "f", "tokens_input", "1");
    let inside = with_cell(&inside, "feature", "f", "cost_usd", "0.0099");
    assert!(
        compare_csv(&inside, &csv, &Tolerance::default())
            .unwrap()
            .deviations
            .is_empty(),
        "1 s, 1 token and 0.0099 USD are inside"
    );

    let outside = with_cell(&csv, "feature", "f", "cycle_s", &(cycle + 2).to_string());
    let outside = with_cell(&outside, "feature", "f", "tokens_output", "2");
    let outside = with_cell(&outside, "feature", "f", "cost_usd", "0.0200");
    let outside = with_cell(&outside, "milestone", "m1", "rework_windows", "3");
    let outside = with_cell(&outside, "milestone", "m1", "throttled_minutes", "2");
    let outside = with_cell(&outside, "milestone", "m1", "price_table", "2026-10-29");

    let diff = compare_csv(&outside, &csv, &Tolerance::default()).unwrap();

    let named: Vec<(&str, &str, &str)> = diff
        .deviations
        .iter()
        .map(|d| (d.id.as_str(), d.column.as_str(), d.kind.as_str()))
        .collect();
    for expected in [
        ("f", "cycle_s", "outside"),
        ("f", "tokens_output", "outside"),
        ("f", "cost_usd", "outside"),
        ("m1", "rework_windows", "outside"),
        ("m1", "throttled_minutes", "outside"),
        ("m1", "price_table", "outside"),
    ] {
        assert!(named.contains(&expected), "{expected:?} in {named:?}");
    }
    assert_eq!(diff.deviations.len(), 6, "{named:?}");
    assert_eq!(diff.max_by_column["cycle_s"], 2.0);
    assert!((diff.max_by_column["cost_usd"] - 0.02).abs() < 1e-9);
}

#[test]
fn the_oracle_reports_a_row_missing_on_either_side() {
    let csv = produced();
    let without_milestone: String = csv
        .lines()
        .filter(|line| !line.starts_with("milestone,"))
        .map(|line| format!("{line}\n"))
        .collect();
    let extra = format!("{csv}feature,g,g,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0.0000,2026-10-22\n");

    let missing = compare_csv(&extra, &csv, &Tolerance::default()).unwrap();
    let added = compare_csv(&without_milestone, &csv, &Tolerance::default()).unwrap();

    let kinds = |diff: &[Deviation]| -> Vec<String> {
        diff.iter()
            .map(|d| format!("{} {} {}", d.kind.as_str(), d.scope, d.id))
            .collect()
    };
    assert_eq!(kinds(&missing.deviations), ["missing feature g"]);
    assert_eq!(kinds(&added.deviations), ["extra milestone m1"]);
}

#[test]
fn the_oracle_refuses_a_reference_whose_header_is_not_the_contract() {
    let csv = produced();
    let wrong = csv.replacen("throttled_minutes", "throttled", 1);

    let error = compare_csv(&wrong, &csv, &Tolerance::default()).unwrap_err();

    assert!(matches!(error, OracleError::Header { .. }), "{error}");
    assert!(error.to_string().contains("throttled"));
}

#[test]
fn the_oracle_reads_quoted_cells_and_crlf_line_ends() {
    let csv = produced();
    let reference = csv.replace('\n', "\r\n").replace(",f,f,", ",f,\"f\",");

    let diff = compare_csv(&reference, &csv, &Tolerance::default()).unwrap();

    assert!(diff.deviations.is_empty(), "{:?}", diff.deviations);
}
