use std::sync::Arc;

use leptos::prelude::*;
use observatory_core::ingest::MemoryReplay;
use observatory_core::model::EventId;
use observatory_core::view::{pointer_text, raw_line_view};

/// The raw JSONL line of the selected event, read-only: re-read from the selection by pointer and
/// masked for display (B-006). The selection itself is never changed.
#[component]
pub fn RawLine(replay: Arc<MemoryReplay>, selected: RwSignal<Option<EventId>>) -> impl IntoView {
    let raw = move || {
        let id = selected.get()?;
        let event = replay.events.iter().find(|event| event.id == id)?;
        let line = replay
            .raw_line(&event.source)
            .map(|raw| raw_line_view(event, raw));
        Some((id, event.source.clone(), line))
    };
    move || {
        raw().map(|(id, pointer, line)| {
            let body = match line {
                Some(line) => view! {
                    <p aria-label="raw-line-kind">{line.kind.clone()}</p>
                    <pre aria-label="raw-line" class="raw">{line.line.text}</pre>
                    <p aria-label="raw-line-masked">{line.line.masked} " token shapes masked"</p>
                }
                .into_any(),
                None => view! {
                    <p role="alert">
                        "The raw line is not available: its file is not in the selection."
                    </p>
                }
                .into_any(),
            };
            view! {
                <section class="raw-view" aria-label="raw-line-view">
                    <h3>"Raw line (read-only)"</h3>
                    <p aria-label="raw-line-event">{id.0}</p>
                    <p aria-label="raw-line-pointer">{pointer_text(&pointer)}</p>
                    {body}
                    <button type="button" on:click=move |_| selected.set(None)>
                        "Close"
                    </button>
                </section>
            }
        })
    }
}
