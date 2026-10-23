use std::sync::Arc;

use leptos::prelude::*;
use observatory_core::ingest::MemoryReplay;
use observatory_core::model::{EventId, MilestoneId};
use observatory_core::view::{TimelineRow, details};

use crate::raw::RawLine;

/// FR-D1: the Details of one milestone, opened from its card or from any event row. M1 has the
/// Timeline tab only; the other tabs arrive with M5.
#[component]
pub fn DetailsView(
    replay: Arc<MemoryReplay>,
    milestone: RwSignal<Option<MilestoneId>>,
) -> impl IntoView {
    let selected = RwSignal::new(None::<EventId>);
    Effect::new(move |_| {
        milestone.track();
        selected.set(None);
    });
    let events = replay.clone();
    let model = Memo::new(move |_| {
        milestone
            .get()
            .map(|milestone| details(&events.events, &milestone))
    });
    move || {
        model.get().map(|model| {
            let rows = model
                .timeline
                .iter()
                .map(|row| timeline_row(row, selected))
                .collect_view();
            view! {
                <section class="details" role="dialog" aria-label="details">
                    <header>
                        <h2>"Details: " <span aria-label="details-milestone">{model.milestone.0.clone()}</span></h2>
                        <div role="tablist" aria-label="details-tabs">
                            <button type="button" role="tab" aria-selected="true">"Timeline"</button>
                        </div>
                        <button type="button" aria-label="details-close" on:click=move |_| milestone.set(None)>
                            "Close details"
                        </button>
                    </header>
                    <p>
                        "Timeline rows: "
                        <span aria-label="details-row-count">{model.timeline.len()}</span>
                    </p>
                    <RawLine replay=replay.clone() selected=selected />
                    <table aria-label="details-timeline">
                        <thead>
                            <tr>
                                <th>"Time"</th>
                                <th>"Event"</th>
                                <th>"Phase"</th>
                                <th>"Overlay"</th>
                                <th>"Kind"</th>
                                <th>"Role"</th>
                                <th>"Model"</th>
                                <th>"Duration"</th>
                                <th>"Raw line"</th>
                            </tr>
                        </thead>
                        <tbody>{rows}</tbody>
                    </table>
                </section>
            }
        })
    }
}

fn timeline_row(row: &TimelineRow, selected: RwSignal<Option<EventId>>) -> AnyView {
    let id = row.event_id.clone();
    let open = row.event_id.clone();
    view! {
        <tr aria-label="timeline-row" class:unmapped=row.unmapped>
            <td>{row.at.to_rfc3339_millis()}</td>
            <td class="event-id">{id.0}</td>
            <td>{row.lane.name()}</td>
            <td>{row.overlay.map(|overlay| overlay.as_str())}</td>
            <td>{row.kind.clone()}</td>
            <td>{row.role.as_str()}</td>
            <td>{row.model.as_ref().map(|model| model.0.clone())}</td>
            <td>{row.duration_ms.map(|ms| format!("{ms} ms"))}</td>
            <td>
                <button
                    type="button"
                    aria-label="raw-line-open"
                    on:click=move |_| selected.set(Some(open.clone()))
                >
                    "Open"
                </button>
            </td>
        </tr>
    }
    .into_any()
}
