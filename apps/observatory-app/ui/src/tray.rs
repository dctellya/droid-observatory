use std::sync::Arc;

use leptos::prelude::*;
use observatory_core::ingest::MemoryReplay;
use observatory_core::model::EventId;
use observatory_core::view::BoardView;

use crate::raw::RawLine;

/// FR-B7: events the mapping did not know, counted and listed; an entry opens its raw line
/// read-only (B-006 masking applied at render).
#[component]
pub fn Tray(
    board: Memo<BoardView>,
    replay: Arc<MemoryReplay>,
    selected: RwSignal<Option<EventId>>,
) -> impl IntoView {
    view! {
        <section class="tray" aria-label="tray">
            <h2>
                "Unmapped tray "
                <span class="count" aria-label="tray-count">
                    {move || board.with(|b| b.tray_unknown)}
                </span>
            </h2>
            <ul>
                {move || {
                    board
                        .with(|b| {
                            b.tray
                                .iter()
                                .map(|entry| {
                                    let id = entry.event_id.clone();
                                    view! {
                                        <li>
                                            <button
                                                type="button"
                                                aria-label="tray-entry"
                                                on:click=move |_| selected.set(Some(id.clone()))
                                            >
                                                {entry.raw_type.clone()} " " {entry.event_id.0.clone()}
                                            </button>
                                        </li>
                                    }
                                })
                                .collect_view()
                        })
                }}
            </ul>
            <RawLine replay=replay selected=selected />
        </section>
    }
}
