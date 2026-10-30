//! Export of the metrics rows (architecture §9, §7 `export`): the CSV and the JSON the UI downloads
//! and the shell writes are the text of `observatory-cli metrics --csv|--json`.

mod metrics_support;

use metrics_support::{m0_events, prices, turn};
use observatory_core::config::OverheadApportionment;
use observatory_core::metrics::{
    CSV_HEADER, ExportFormat, MetricsReport, Notes, ReportSetup, report_at_last_event,
};
use observatory_core::model::{CycleTemplate, Role};
use serde_json::Value;

fn m0_report() -> MetricsReport {
    let template = CycleTemplate::engineering();
    let prices = prices();
    report_at_last_event(
        &m0_events(),
        &ReportSetup {
            template: &template,
            prices: &prices,
            notes: &Notes::default(),
            budget: 0.0,
            launch_budget_usd: None,
            apportionment: OverheadApportionment::Separate,
        },
    )
    .expect("the M0 capture holds events")
}

#[test]
fn the_csv_export_is_the_report_csv_and_counts_its_data_rows() {
    let report = m0_report();
    let file = report.export(ExportFormat::Csv, "observatory-v1");
    assert_eq!(file.text, report.to_csv());
    assert_eq!(file.rows, report.rows.len());
    assert_eq!(file.text.lines().count(), file.rows + 1);
    assert!(file.text.starts_with(&CSV_HEADER.join(",")));
    assert_eq!(file.mime, "text/csv");
    assert_eq!(file.file_name, "observatory-v1-metrics.csv");
}

#[test]
fn the_json_export_parses_and_carries_the_csv_rows() {
    let report = m0_report();
    let file = report.export(ExportFormat::Json, "observatory-v1");
    assert_eq!(file.text, report.to_json());
    assert!(file.text.ends_with('\n'));
    let parsed: Value = serde_json::from_str(&file.text).expect("the export parses");
    let rows = parsed["rows"].as_array().expect("rows is an array");
    assert_eq!(rows.len(), file.rows);
    assert_eq!(file.rows, report.rows.len());
    assert_eq!(parsed["price_table"].as_str(), Some("2026-10-22"));
    assert_eq!(file.mime, "application/json");
    assert_eq!(file.file_name, "observatory-v1-metrics.json");
}

#[test]
fn a_file_name_keeps_only_safe_characters_of_the_mission_name() {
    let report = m0_report();
    let file = report.export(ExportFormat::Csv, "my mission/../v1: \"x\"");
    assert!(
        file.file_name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c)),
        "{}",
        file.file_name
    );
    assert!(!file.file_name.contains(".."), "{}", file.file_name);
    assert!(file.file_name.ends_with("-metrics.csv"));
}

#[test]
fn the_formats_are_listed_in_the_order_the_panel_offers_them() {
    assert_eq!(ExportFormat::ALL, [ExportFormat::Csv, ExportFormat::Json]);
    assert_eq!(ExportFormat::Csv.label(), "Export CSV");
    assert_eq!(ExportFormat::Json.label(), "Export JSON");
    assert_eq!(ExportFormat::Csv.aria_label(), "export-csv");
    assert_eq!(ExportFormat::Json.aria_label(), "export-json");
}

#[test]
fn no_report_is_made_when_only_turn_usage_is_held() {
    let template = CycleTemplate::engineering();
    let prices = prices();
    let only_turn = vec![turn(
        "t1",
        0,
        Role::Worker,
        "claude-sonnet-5-5",
        "build",
        [1, 1, 0],
    )];
    let setup = ReportSetup {
        template: &template,
        prices: &prices,
        notes: &Notes::default(),
        budget: 0.0,
        launch_budget_usd: None,
        apportionment: OverheadApportionment::Separate,
    };
    assert!(report_at_last_event(&only_turn, &setup).is_none());
    assert!(report_at_last_event(&[], &setup).is_none());
}
