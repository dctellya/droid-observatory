use std::sync::Arc;

use leptos::prelude::*;
use leptos::task::spawn_local;
use observatory_core::ingest::MemoryReplay;

use crate::backend;
use crate::board::Board;
use crate::library::Library;

/// Root: the mission library until a capture is opened, then its board in Replay mode.
#[component]
pub fn App() -> impl IntoView {
    let rows = RwSignal::new(backend::load_library());
    // Captures opened since the page loaded stay in memory, so their library line can reopen.
    let opened_here: RwSignal<Vec<Arc<MemoryReplay>>> = RwSignal::new(Vec::new());
    let current: RwSignal<Option<Arc<MemoryReplay>>> = RwSignal::new(None);
    let status = RwSignal::new(None::<String>);
    let failure = RwSignal::new(None::<String>);

    let pick = Callback::new(move |files: Vec<web_sys::File>| {
        failure.set(None);
        status.set(Some(format!("Loading {} files…", files.len())));
        spawn_local(async move {
            match backend::open_selection(files).await {
                Ok(replay) => {
                    let replay = Arc::new(replay);
                    rows.update(|rows| {
                        backend::upsert(rows, replay.summary.library_row());
                        backend::save_library(rows);
                    });
                    opened_here.update(|all| {
                        all.retain(|r| r.summary.mission_id != replay.summary.mission_id);
                        all.push(replay.clone());
                    });
                    current.set(Some(replay));
                }
                Err(message) => failure.set(Some(message)),
            }
            status.set(None);
        });
    });
    let reopen = Callback::new(move |mission_id: String| {
        let found = opened_here
            .read()
            .iter()
            .find(|replay| replay.summary.mission_id.0 == mission_id)
            .cloned();
        current.set(found);
    });
    let close = Callback::new(move |()| current.set(None));
    let library_rows = Signal::derive(move || rows.get());
    let reopenable = Signal::derive(move || {
        opened_here
            .read()
            .iter()
            .map(|replay| replay.summary.mission_id.0.clone())
            .collect::<Vec<String>>()
    });

    view! {
        <main class="app" aria-label="observatory">
            {move || match current.get() {
                Some(replay) => view! { <Board replay=replay on_close=close /> }.into_any(),
                None => {
                    view! {
                        <Library
                            rows=library_rows
                            reopenable=reopenable
                            status=status
                            failure=failure
                            on_pick=pick
                            on_open=reopen
                        />
                    }
                        .into_any()
                }
            }}
        </main>
    }
}
