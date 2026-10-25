use leptos::prelude::*;
use observatory_core::model::{CycleTemplate, EventId, MilestoneId};
use observatory_core::view::{TimelineRow, details};

use crate::capture::Capture;
use crate::raw::RawLine;

/// FR-D1: the Details of one milestone, opened from its card or from any event row. M1 has the
/// Timeline tab only; the other tabs arrive with M5.
#[component]
pub fn DetailsView(capture: Capture, milestone: RwSignal<Option<MilestoneId>>) -> impl IntoView {
    let selected = RwSignal::new(None::<EventId>);
    Effect::new(move |_| {
        milestone.track();
        selected.set(None);
    });
    let events = capture.events.clone();
    let model = Memo::new(move |_| {
        milestone
            .get()
            .map(|milestone| events.with(|events| details(events, &milestone)))
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
                    <RawLine capture=capture.clone() selected=selected />
                    <div class="lib"><div class="scroll">
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
                    </div></div>
                </section>
            }
        })
    }
}

fn timeline_row(row: &TimelineRow, selected: RwSignal<Option<EventId>>) -> AnyView {
    let template = expect_context::<RwSignal<CycleTemplate>>();
    let id = row.event_id.clone();
    let open = row.event_id.clone();
    view! {
        <tr aria-label="timeline-row" class:unmapped=row.unmapped>
            <td class="mono">{row.at.to_rfc3339_millis()}</td>
            <td class="event-id mono">{id.0}</td>
            <td>{template.with(|template| template.name_of(&row.lane))}</td>
            <td>{row.overlay.map(|overlay| overlay.as_str())}</td>
            <td class="mono">{row.kind.clone()}</td>
            <td>{row.role.as_str()}</td>
            <td class="mono">{row.model.as_ref().map(|model| model.0.clone())}</td>
            <td class="n">{row.duration_ms.map(|ms| format!("{ms} ms"))}</td>
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
