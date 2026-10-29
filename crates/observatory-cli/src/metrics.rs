//! `metrics <file> --json`: the cost half of the metrics report over a capture.

use std::error::Error;
use std::io::Write;
use std::path::Path;

use observatory_core::config::Config;
use observatory_core::ingest::mission_events;
use observatory_core::metrics::{CostInput, Notes, PriceTable, cost_report};
use observatory_core::model::CycleTemplate;

/// What the report reads besides the capture.
pub struct Setup<'a> {
    pub config: &'a Config,
    pub template: &'a CycleTemplate,
    pub price_table: Option<&'a Path>,
    pub budget_usd: Option<f64>,
}

/// Writes the report as JSON; the capture's warnings go to `warnings`. The report is made at the
/// mission's last event, so the same capture gives the same figures on any day.
pub fn metrics(
    file: &Path,
    setup: &Setup<'_>,
    out: &mut impl Write,
    warnings: &mut impl Write,
) -> Result<(), Box<dyn Error>> {
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
    let report = cost_report(&CostInput {
        events: &events,
        template: setup.template,
        notes: &Notes::default(),
        prices: &prices,
        end,
        budget: setup.config.metrics.budget,
        launch_budget_usd: setup.budget_usd,
        apportionment: setup.config.metrics.overhead_apportionment,
    });
    serde_json::to_writer_pretty(&mut *out, &report)?;
    writeln!(out)?;
    Ok(())
}
