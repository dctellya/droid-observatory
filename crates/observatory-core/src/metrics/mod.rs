//! Time states, rework windows and throttled time (architecture §9, PRD §10.1 and §10.2).
//!
//! Everything here is pure over mapped events: no store, no clock, no I/O. Lanes are addressed
//! by the cycle template (F-041); a rework window's cause is the lane id of the lane that holds
//! the event that opened it. Compiled to wasm with the rest of the model (architecture I9).

mod notes;
mod rework;
mod span;
mod state;
mod throttled;
mod waste;

pub use notes::{Notes, events_of_feature};
pub use rework::{
    ReworkKind, ReworkSpan, ReworkWindow, first_pass_and_rework, fix_loops, merge_windows,
    rework_causes, rework_windows,
};
pub use span::{Span, event_span};
pub use state::{
    Interval, PartitionContext, TimeState, WaitKind, active_spans, partition, partition_in,
};
pub use throttled::{Throttled, ThrottledInterval, UNKNOWN_REASON, throttled};
pub use waste::{WasteCause, WasteGroup, WasteTotals, waste_by_cause};
