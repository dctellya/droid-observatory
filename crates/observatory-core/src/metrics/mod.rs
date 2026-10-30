//! Time states, rework windows, throttled time, tokens and cost (architecture §9, PRD §10.1,
//! §10.2 and §10.5).
//!
//! Everything here is pure over mapped events: no store, no clock, no I/O. Lanes are addressed
//! by the cycle template (F-041); a rework window's cause is the lane id of the lane that holds
//! the event that opened it. Compiled to wasm with the rest of the model (architecture I9).

mod bottleneck;
mod budget;
mod cards;
mod compare;
mod csv;
mod cycle;
mod export;
mod intervals;
mod mission_report;
mod notes;
mod oracle;
mod price;
mod report;
mod rework;
mod rows;
mod span;
mod state;
mod throttled;
mod tokens;
mod waste;

pub use bottleneck::Bottleneck;
pub use budget::{BudgetMeter, budget_meter};
pub use cards::{CardTime, card_times};
pub use compare::{
    ModelComparisonRow, completed_features, integration_observable, model_comparison,
};
pub use csv::CSV_HEADER;
pub use cycle::{ClockKinds, Cycle, CycleEnd, Cycles, StageTime, cycles, stage_times};
pub use export::{ExportFile, ExportFormat, ReportSetup, report_at_last_event};
pub use intervals::{CardIntervals, IntervalRow, ReworkWindowRow, card_intervals};
pub use mission_report::{MetricsReport, MissionSummary, metrics_report};
pub use notes::{Notes, events_of_feature};
pub use oracle::{Deviation, DeviationKind, OracleDiff, OracleError, Tolerance, compare_csv};
pub use price::{Cost, CostUnit, PriceError, PriceTable, Rates};
pub use report::{
    CostInput, CostReport, LaneUsage, MilestoneUsage, Overhead, OverheadShare, RoleCost,
    RoleTokens, SessionCost, UnpricedTokens, cost_report,
};
pub use rework::{
    ReworkKind, ReworkSpan, ReworkWindow, first_pass_and_rework, fix_loops, merge_windows,
    rework_causes, rework_windows,
};
pub use rows::{MetricsRow, RowScope};
pub use span::{Span, event_span};
pub use state::{
    Interval, PartitionContext, TimeState, WaitKind, active_spans, partition, partition_in,
};
pub use throttled::{Throttled, ThrottledInterval, UNKNOWN_REASON, throttled};
pub use tokens::{
    Reconciliation, SessionReconciliation, SignedCounts, TokenCounts, TokensInWaste, Turn,
    UNJOINED_SESSION, reconcile, tokens_in_waste, turns,
};
pub use waste::{WasteCause, WasteGroup, WasteTotals, waste_by_cause};
