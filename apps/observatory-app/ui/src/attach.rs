use leptos::prelude::*;
use observatory_core::ingest::AttachStatus;
use observatory_core::model::SessionId;

use crate::shell;

/// What the shell reports about attach, provided by the root so the library and the board
/// show the same figures.
#[derive(Clone, Copy)]
pub struct AttachCtx {
    pub status: RwSignal<AttachStatus>,
    /// Opens the mission a watched session belongs to (the `attach` command).
    pub open: Callback<SessionId>,
}

/// FR-L1/FR-L2: the sessions being watched (named by tag, else by session id), how many were
/// found and not watched, and the warnings. In the browser host there is nothing to attach to,
/// and the panel says so.
#[component]
pub fn AttachPanel() -> impl IntoView {
    let ctx = expect_context::<AttachCtx>();
    let status = ctx.status;
    if !shell::is_tauri() {
        return view! {
            <section class="attach" aria-label="attach">
                <h2>"Attach"</h2>
                <p aria-label="attach-unavailable" role="status">
                    "Attach not available in browser host: open the Tauri window (make app-dev) to watch sessions."
                </p>
            </section>
        }
        .into_any();
    }
    view! {
        <section class="attach" aria-label="attach">
            <h2>"Attach"</h2>
            <p aria-label="attach-summary">
                {move || {
                    let watched = status.with(|s| s.watched.len());
                    format!("Watching {watched} sessions")
                }}
            </p>
            <Show when=move || status.with(|s| s.unwatched > 0)>
                <p class="warning" role="status" aria-label="attach-unwatched">
                    {move || format!("{} sessions not watched", status.with(|s| s.unwatched))}
                </p>
            </Show>
            <For
                each=move || status.with(|s| s.warnings.clone())
                key=|warning| warning.clone()
                let(warning)
            >
                <p class="warning" role="status" aria-label="attach-warning">{warning}</p>
            </For>
            {move || {
                status
                    .with(|s| s.error.clone())
                    .map(|error| {
                        view! {
                            <p class="error" role="alert" aria-label="attach-error">{error}</p>
                        }
                    })
            }}
            <ul aria-label="watched-sessions">
                <For
                    each=move || status.with(|s| s.watched.clone())
                    key=|row| (row.session_id.clone(), row.name.clone())
                    let(row)
                >
                    <li aria-label="watched-session">
                        <span aria-label="watched-name">{row.name.clone()}</span>
                        " "
                        <code>{row.session_id.0.clone()}</code>
                        " "
                        <button
                            aria-label="attach-session"
                            on:click={
                                let id = row.session_id.clone();
                                move |_| ctx.open.run(id.clone())
                            }
                        >
                            "Open"
                        </button>
                    </li>
                </For>
            </ul>
        </section>
    }
    .into_any()
}
