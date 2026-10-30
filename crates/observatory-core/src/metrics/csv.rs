//! The CSV of the column contract (architecture §9): RFC 4180 quoting, `\n` line ends.

use super::mission_report::MetricsReport;
use super::rows::MetricsRow;
use super::waste::WasteCause;

/// The 22 names of the contract, in order: the pre-F-033 columns and `throttled_minutes` after
/// `rework_windows`. Eight of them are the waste causes.
pub const CSV_HEADER: [&str; 22] = [
    "scope",
    "id",
    "name",
    "cycle_s",
    "first_pass_active_s",
    "rework_s",
    "waste_waiting_human_question_s",
    "waste_waiting_human_permission_s",
    "waste_waiting_merge_s",
    "waste_waiting_validation_s",
    "waste_rework_s",
    "waste_retry_failure_s",
    "waste_idle_queued_s",
    "waste_idle_unscheduled_s",
    "rework_windows",
    "throttled_minutes",
    "tokens_input",
    "tokens_output",
    "tokens_cache_read",
    "tokens_cache_write",
    "cost_usd",
    "price_table",
];

fn quote(cell: &str) -> String {
    if cell.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", cell.replace('"', "\"\""))
    } else {
        cell.to_owned()
    }
}

fn cells(row: &MetricsRow) -> Vec<String> {
    let mut cells = vec![
        row.scope.as_str().to_owned(),
        quote(&row.id),
        quote(&row.name),
        row.cycle_s.to_string(),
        row.first_pass_active_s.to_string(),
        row.rework_s.to_string(),
    ];
    cells.extend(
        WasteCause::ALL
            .into_iter()
            .map(|cause| row.waste_s.get(&cause).copied().unwrap_or(0).to_string()),
    );
    cells.extend([
        row.rework_windows.to_string(),
        row.throttled_minutes.to_string(),
        row.tokens_input.to_string(),
        row.tokens_output.to_string(),
        row.tokens_cache_read.to_string(),
        row.tokens_cache_write.to_string(),
        format!("{:.4}", row.cost_usd),
        quote(&row.price_table),
    ]);
    cells
}

impl MetricsReport {
    /// The header and one line per feature and milestone row. The mission overhead is not a row
    /// (`scope` is `feature` or `milestone`): it is the `overhead` line of the JSON.
    pub fn to_csv(&self) -> String {
        let mut out = CSV_HEADER.join(",");
        out.push('\n');
        for row in &self.rows {
            out.push_str(&cells(row).join(","));
            out.push('\n');
        }
        out
    }
}
