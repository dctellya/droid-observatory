//! `metrics <file> --json|--csv|--diff`: the metrics report of a capture.

use std::error::Error;
use std::io::Write;
use std::path::{Path, PathBuf};

use observatory_core::config::Config;
use observatory_core::ingest::mission_events;
use observatory_core::metrics::{
    CostInput, Notes, PriceTable, Tolerance, compare_csv, metrics_report,
};
use observatory_core::model::CycleTemplate;

/// What the report reads besides the capture.
pub struct Setup<'a> {
    pub config: &'a Config,
    pub template: &'a CycleTemplate,
    pub price_table: Option<&'a Path>,
    pub budget_usd: Option<f64>,
}

pub enum Format {
    Json,
    Csv,
    /// The CSV against this reference file.
    Diff(PathBuf),
}

/// Writes the report in `format`; the capture's warnings go to `warnings`. The report is made at
/// the mission's last event, so the same capture gives the same figures on any day. Returns
/// `false` when a diff found a deviation (its lines are in `out`).
pub fn metrics(
    file: &Path,
    format: &Format,
    setup: &Setup<'_>,
    out: &mut impl Write,
    warnings: &mut impl Write,
) -> Result<bool, Box<dyn Error>> {
    let (events, found) = mission_events(file, &setup.config.ingest, setup.template.clone())?;
    for warning in found {
        writeln!(warnings, "observatory-cli: warning: {warning}")?;
    }
    let prices = PriceTable::load(
        setup
            .price_table
            .unwrap_or_else(|| Path::new(&setup.config.metrics.price_table)),
    )?;
    let end = events
        .iter()
        .filter(|event| !event.kind.is_panel_only())
        .map(|event| event.at)
        .max()
        .ok_or("the capture holds no event")?;
    let report = metrics_report(&CostInput {
        events: &events,
        template: setup.template,
        notes: &Notes::default(),
        prices: &prices,
        end,
        budget: setup.config.metrics.budget,
        launch_budget_usd: setup.budget_usd,
        apportionment: setup.config.metrics.overhead_apportionment,
    });
    match format {
        Format::Json => {
            serde_json::to_writer_pretty(&mut *out, &report)?;
            writeln!(out)?;
        }
        Format::Csv => out.write_all(report.to_csv().as_bytes())?,
        Format::Diff(reference) => {
            let text = std::fs::read_to_string(reference)
                .map_err(|err| format!("{}: {err}", reference.display()))?;
            let diff = compare_csv(&text, &report.to_csv(), &Tolerance::default())?;
            writeln!(out, "reference: {}", reference.display())?;
            writeln!(out, "rows compared: {}", diff.rows_compared)?;
            writeln!(out, "deviations: {}", diff.deviations.len())?;
            for d in &diff.deviations {
                writeln!(
                    out,
                    "{} {} {}: reference {}, engine {} ({})",
                    d.scope,
                    d.id,
                    d.column,
                    d.reference,
                    d.produced,
                    d.kind.as_str()
                )?;
            }
            writeln!(out, "maximum deviation per column:")?;
            for (column, max) in &diff.max_by_column {
                writeln!(out, "  {column}: {max}")?;
            }
            return Ok(diff.deviations.is_empty());
        }
    }
    Ok(true)
}
