use std::sync::Arc;

use leptos::prelude::*;
use leptos::task::spawn_local;
use observatory_core::ingest::{AttachStatus, EventBatch, MemoryReplay, OpenedMission, SessionRow};
use observatory_core::model::{CycleTemplate, SessionId};
use observatory_core::view::{HostedStatus, LiveOutput, MissionRow, OutputLines, StallBatch};
use serde_json::{Value, json};

use crate::attach::AttachCtx;
use crate::backend;
use crate::board::Board;
use crate::capture::Capture;
use crate::controls::StallCtx;
use crate::launch::{HostedCtx, LaunchScreen};
use crate::library::Library;
use crate::perf;
use crate::shell;

/// Refresh state of the library rows: nothing running, a read running, a read running that a
/// later batch has made stale.
const IDLE: u8 = 0;
const READING: u8 = 1;
const STALE: u8 = 2;

/// Tells the shell, once the reactive graph has drawn it, that the events of a hosted run are
/// on screen: the render half of the 1 s measurement.
fn report_rendered(batch: &EventBatch, board_open: bool) {
    let mission_id = batch.mission_id.to_string();
    let event_ids: Vec<String> = batch
        .events
        .iter()
        .map(|event| event.id.to_string())
        .collect();
    spawn_local(async move {
        leptos::task::tick().await;
        let ms = perf::batch_drawn();
        let args = json!({
            "mission_id": mission_id,
            "event_ids": event_ids,
            "open": board_open,
            "ms": ms,
        });
        let _ = shell::invoke::<_, ()>("panel_rendered", &args).await;
    });
}

fn refresh_rows(rows: RwSignal<Vec<MissionRow>>, state: RwSignal<u8>) {
    if state.get_untracked() != IDLE {
        state.set(STALE);
        return;
    }
    state.set(READING);
    spawn_local(async move {
        loop {
            if let Ok(list) = shell::invoke::<_, Vec<MissionRow>>("list_missions", &json!({})).await
            {
                rows.set(list);
            }
            if state.get_untracked() == STALE {
                state.set(READING);
                continue;
            }
            state.set(IDLE);
            break;
        }
    });
}

/// Root: the mission library until a capture is opened, then its board. In the Tauri window the
/// library is the store's, the shell's batches grow the open board, and attach is live; in the
/// browser host the file picker feeds an in-memory replay.
#[component]
pub fn App() -> impl IntoView {
    let tauri = shell::is_tauri();
    let rows = RwSignal::new(if tauri {
        Vec::new()
    } else {
        backend::load_library()
    });
    let rows_state = RwSignal::new(IDLE);
    // Captures opened since the page loaded stay in memory, so their library line can reopen.
    let opened_here: RwSignal<Vec<Arc<MemoryReplay>>, LocalStorage> =
        RwSignal::new_local(Vec::new());
    let current: RwSignal<Option<Capture>, LocalStorage> = RwSignal::new_local(None);
    // Batches that arrive while a mission is being fetched; they are folded in once it is held.
    let pending: RwSignal<Option<Vec<EventBatch>>, LocalStorage> = RwSignal::new_local(None);
    let status = RwSignal::new(None::<String>);
    let failure = RwSignal::new(None::<String>);
    let attach_status = RwSignal::new(AttachStatus::default());
    let hosted = HostedCtx::new();
    provide_context(hosted);
    let stalls = StallCtx(RwSignal::new(std::collections::HashMap::new()));
    provide_context(stalls);
    let launching = RwSignal::new(false);

    let open_shell = Callback::new(move |(command, args): (&'static str, Value)| {
        perf::panel_render_started();
        failure.set(None);
        status.set(Some("Opening…".to_owned()));
        pending.set(Some(Vec::new()));
        spawn_local(async move {
            let result = shell::invoke::<_, OpenedMission>(command, &args).await;
            let buffered = pending
                .try_update(Option::take)
                .flatten()
                .unwrap_or_default();
            match result {
                Ok(opened) => {
                    let capture = Capture::of_shell(opened);
                    for batch in buffered {
                        if batch.mission_id == *capture.mission_id() {
                            capture.merge(batch);
                        }
                    }
                    let held = capture.clone();
                    spawn_local(async move { held.refresh_sessions().await });
                    current.set(Some(capture));
                    refresh_rows(rows, rows_state);
                }
                Err(message) => failure.set(Some(message)),
            }
            status.set(None);
        });
    });
    // The template is data the shell read at start-up; the browser host has no shell and keeps
    // the compiled-in engineering cycle.
    let template = RwSignal::new(CycleTemplate::engineering());
    provide_context(template);
    provide_context(AttachCtx {
        status: attach_status,
        open: Callback::new(move |session_id: SessionId| {
            open_shell.run(("attach", json!({ "session_id": session_id })));
        }),
    });

    if tauri {
        refresh_rows(rows, rows_state);
        spawn_local(async move {
            if let Ok(now) = shell::invoke::<_, AttachStatus>("list_sessions", &json!({})).await {
                attach_status.set(now);
            }
        });
        spawn_local(async move {
            if let Ok(now) = shell::invoke::<_, CycleTemplate>("cycle_template", &json!({})).await {
                template.set(now);
            }
        });
        spawn_local(async move {
            if let Ok(config) =
                shell::invoke::<_, observatory_core::config::Config>("app_config", &json!({})).await
            {
                hosted
                    .output
                    .set(LiveOutput::new(config.ui.live_output_lines));
            }
        });
        shell::listen::<HostedStatus>("hosted_status", move |now| {
            hosted.status.set(Some(now));
        });
        shell::listen::<StallBatch>("stall_flag", move |batch| {
            stalls.0.update(|all| {
                all.insert(batch.mission_id, batch.flags);
            });
        });
        shell::listen::<OutputLines>("live_output", move |lines| {
            hosted.output.update(|output| output.push(lines));
        });
        shell::listen::<AttachStatus>("attach_status", move |now| attach_status.set(now));
        shell::listen::<SessionRow>("session_discovered", move |_| {
            refresh_rows(rows, rows_state);
        });
        shell::listen::<EventBatch>("events_appended", move |batch| {
            perf::batch_received();
            if pending.with_untracked(Option::is_some) {
                pending.update(|held| held.get_or_insert_with(Vec::new).push(batch));
                return;
            }
            let open = current
                .get_untracked()
                .filter(|capture| *capture.mission_id() == batch.mission_id);
            let hosted_here = hosted
                .launched
                .get_untracked()
                .is_some_and(|launched| launched.session_id == batch.mission_id.to_string());
            if hosted_here {
                report_rendered(&batch, open.is_some());
            }
            match open {
                Some(capture) => {
                    if capture.merge(batch) {
                        spawn_local(async move { capture.refresh_sessions().await });
                    }
                }
                None => refresh_rows(rows, rows_state),
            }
        });
    }

    let pick = Callback::new(move |files: Vec<web_sys::File>| {
        perf::panel_render_started();
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
                    current.set(Some(Capture::of_memory(replay)));
                }
                Err(message) => failure.set(Some(message)),
            }
            status.set(None);
        });
    });
    let reopen = Callback::new(move |mission_id: String| {
        if tauri {
            open_shell.run(("open_mission", json!({ "mission_id": mission_id })));
            return;
        }
        let found = opened_here
            .read()
            .iter()
            .find(|replay| replay.summary.mission_id.0 == mission_id)
            .cloned();
        current.set(found.map(Capture::of_memory));
    });
    let open_path = Callback::new(move |path: String| {
        open_shell.run(("open_replay", json!({ "path": path.trim() })));
    });
    let close = Callback::new(move |()| {
        current.set(None);
        if tauri {
            refresh_rows(rows, rows_state);
        }
    });
    let library_rows = Signal::derive(move || rows.get());
    let reopenable = Signal::derive(move || {
        if tauri {
            rows.read()
                .iter()
                .map(|row| row.id.0.clone())
                .collect::<Vec<String>>()
        } else {
            opened_here
                .read()
                .iter()
                .map(|replay| replay.summary.mission_id.0.clone())
                .collect::<Vec<String>>()
        }
    });

    view! {
        <main class="observatory" aria-label="observatory">
            {move || match current.get() {
                None if launching.get() => {
                    view! { <LaunchScreen on_close=Callback::new(move |()| launching.set(false)) /> }
                        .into_any()
                }
                Some(capture) => view! { <Board capture=capture on_close=close /> }.into_any(),
                None => {
                    view! {
                        <Library
                            rows=library_rows
                            reopenable=reopenable
                            status=status
                            failure=failure
                            tauri=tauri
                            on_pick=pick
                            on_open=reopen
                            on_open_path=open_path
                            on_launch=Callback::new(move |()| launching.set(true))
                        />
                    }
                        .into_any()
                }
            }}
        </main>
    }
}
