//! User Timing marks and measures the e2e suite reads with `performance.getEntriesByType`
//! (VAL-E2E-003). Every function is a no-op when the page has no `Performance` object.

use wasm_bindgen::JsCast;

/// Replay or attach requested to the Panel holding its numbers.
pub const PANEL_RENDER: &str = "panel:render";
/// `events_appended` received by the window to the board having drawn the batch.
pub const FIRST_ON_BOARD: &str = "event:first_on_board";

const PANEL_START: &str = "panel:render:start";
const PANEL_END: &str = "panel:render:end";
const BATCH_RECEIVED: &str = "events_appended:received";
const BATCH_DRAWN: &str = "events_appended:drawn";

fn performance() -> Option<web_sys::Performance> {
    web_sys::window()?.performance()
}

fn has_mark(performance: &web_sys::Performance, name: &str) -> bool {
    performance
        .get_entries_by_name_with_entry_type(name, "mark")
        .length()
        > 0
}

/// Measures `name` from the mark `start` to now and returns its duration in milliseconds. The
/// start mark is cleared, so one start gives one measure.
fn measure_since(name: &str, start: &str, end: &str) -> Option<f64> {
    let performance = performance()?;
    if !has_mark(&performance, start) {
        return None;
    }
    performance.mark(end).ok()?;
    performance
        .measure_with_start_mark_and_end_mark(name, start, end)
        .ok()?;
    performance.clear_marks_with_mark_name(start);
    performance.clear_marks_with_mark_name(end);
    let entries = performance.get_entries_by_name_with_entry_type(name, "measure");
    let last = entries.get(entries.length().checked_sub(1)?);
    Some(
        last.dyn_into::<web_sys::PerformanceEntry>()
            .ok()?
            .duration(),
    )
}

/// A replay or attach is about to be requested.
pub fn panel_render_started() {
    if let Some(performance) = performance() {
        let _ = performance.mark(PANEL_START);
    }
}

/// The Panel has drawn: closes the measure the last `panel_render_started` opened, if any.
pub fn panel_rendered() {
    measure_since(PANEL_RENDER, PANEL_START, PANEL_END);
}

/// A batch of events reached the window.
pub fn batch_received() {
    if let Some(performance) = performance() {
        let _ = performance.mark(BATCH_RECEIVED);
    }
}

/// The batch received last is on the board: the measure's duration in milliseconds.
pub fn batch_drawn() -> Option<f64> {
    measure_since(FIRST_ON_BOARD, BATCH_RECEIVED, BATCH_DRAWN)
}
