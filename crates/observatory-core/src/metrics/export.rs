//! Export of the metrics rows (architecture §9): the CSV (RFC 4180) and the JSON that
//! `observatory-cli metrics --csv|--json` prints, as text the browser downloads and the shell
//! writes. One function makes the report at the mission's last event, so a capture gives the same
//! text on every surface and on any day.

use serde::{Deserialize, Serialize};

use super::mission_report::{MetricsReport, metrics_report};
use super::notes::Notes;
use super::price::PriceTable;
use super::report::CostInput;
use crate::config::OverheadApportionment;
use crate::model::{CycleTemplate, Event};

/// What the report reads besides the events.
pub struct ReportSetup<'a> {
    pub template: &'a CycleTemplate,
    pub prices: &'a PriceTable,
    pub notes: &'a Notes,
    /// `[metrics].budget`.
    pub budget: f64,
    /// The launch form's `budget_usd`.
    pub launch_budget_usd: Option<f64>,
    pub apportionment: OverheadApportionment,
}

/// The report at the last event that is not `turn.usage`; `None` when there is no such event.
pub fn report_at_last_event(events: &[Event], setup: &ReportSetup<'_>) -> Option<MetricsReport> {
    let end = events
        .iter()
        .filter(|event| !event.kind.is_panel_only())
        .map(|event| event.at)
        .max()?;
    Some(metrics_report(&CostInput {
        events,
        template: setup.template,
        notes: setup.notes,
        prices: setup.prices,
        end,
        budget: setup.budget,
        launch_budget_usd: setup.launch_budget_usd,
        apportionment: setup.apportionment,
    }))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportFormat {
    Csv,
    Json,
}

impl ExportFormat {
    /// The order the Panel offers them.
    pub const ALL: [ExportFormat; 2] = [ExportFormat::Csv, ExportFormat::Json];

    pub fn label(self) -> &'static str {
        match self {
            ExportFormat::Csv => "Export CSV",
            ExportFormat::Json => "Export JSON",
        }
    }

    pub fn aria_label(self) -> &'static str {
        match self {
            ExportFormat::Csv => "export-csv",
            ExportFormat::Json => "export-json",
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            ExportFormat::Csv => "csv",
            ExportFormat::Json => "json",
        }
    }

    pub fn mime(self) -> &'static str {
        match self {
            ExportFormat::Csv => "text/csv",
            ExportFormat::Json => "application/json",
        }
    }
}

/// One export: the text, the data rows it holds and the name a download or a save dialog offers.
#[derive(Clone, Debug, PartialEq)]
pub struct ExportFile {
    pub format: ExportFormat,
    pub file_name: String,
    pub mime: &'static str,
    pub text: String,
    /// Feature and milestone rows: the CSV's data lines, the JSON's `rows` entries.
    pub rows: usize,
}

/// `name` reduced to what a file name can hold on every host.
fn file_stem(name: &str) -> String {
    let mut stem = String::new();
    for c in name.chars() {
        let kept = if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
            c
        } else {
            '-'
        };
        if kept != '-' || !stem.ends_with('-') {
            stem.push(kept);
        }
    }
    let stem = stem.trim_matches('-');
    if stem.is_empty() {
        "mission".to_owned()
    } else {
        stem.to_owned()
    }
}

impl MetricsReport {
    /// The report as `observatory-cli metrics --json` prints it: pretty, one line end at the close.
    pub fn to_json(&self) -> String {
        let mut text = serde_json::to_string_pretty(self).unwrap_or_default();
        text.push('\n');
        text
    }

    pub fn export(&self, format: ExportFormat, mission_name: &str) -> ExportFile {
        let text = match format {
            ExportFormat::Csv => self.to_csv(),
            ExportFormat::Json => self.to_json(),
        };
        ExportFile {
            format,
            file_name: format!("{}-metrics.{}", file_stem(mission_name), format.extension()),
            mime: format.mime(),
            text,
            rows: self.rows.len(),
        }
    }
}
