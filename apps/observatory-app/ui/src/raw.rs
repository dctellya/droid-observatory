use leptos::prelude::*;
use observatory_core::model::EventId;
use observatory_core::view::pointer_text;

use crate::capture::Capture;

/// The raw JSONL line of the selected event, read-only: re-read by pointer (from the picked
/// files in the browser host, by the shell in the Tauri window) and masked for display (B-006).
/// The selection itself is never changed.
#[component]
pub fn RawLine(capture: Capture, selected: RwSignal<Option<EventId>>) -> impl IntoView {
    let line = LocalResource::new(move || {
        let capture = capture.clone();
        let id = selected.get();
        async move {
            match id {
                Some(id) => {
                    let result = capture.raw_line(&id).await;
                    Some((id, result))
                }
                None => None,
            }
        }
    });
    move || {
        line.get().flatten().map(|(id, result)| {
            let (pointer, body) = match result {
                Ok(line) => (
                    pointer_text(&line.pointer),
                    view! {
                        <p aria-label="raw-line-kind">{line.kind.clone()}</p>
                        <pre aria-label="raw-line" class="raw mono">{line.line.text}</pre>
                        <p aria-label="raw-line-masked">{line.line.masked} " token shapes masked"</p>
                    }
                    .into_any(),
                ),
                Err(message) => (
                    String::new(),
                    view! { <p role="alert">{message}</p> }.into_any(),
                ),
            };
            view! {
                <section class="raw-view" aria-label="raw-line-view">
                    <h3>"Raw line (read-only)"</h3>
                    <p aria-label="raw-line-event">{id.0}</p>
                    <p aria-label="raw-line-pointer">{pointer}</p>
                    {body}
                    <button type="button" on:click=move |_| selected.set(None)>
                        "Close"
                    </button>
                </section>
            }
        })
    }
}
