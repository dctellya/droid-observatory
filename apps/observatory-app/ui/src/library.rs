use leptos::html::Input;
use leptos::prelude::*;
use observatory_core::view::{MissionRow, library};

use crate::attach::AttachPanel;
use crate::backend::files_of;

/// FR-L4: the captures known to this host (the store's in the Tauri window, those opened in this
/// browser otherwise) with name, date and event count, and the controls that open a capture. One selection holds the root `.jsonl`, `<slug>.children/*.jsonl`, the
/// `.settings.json` sidecars and the `.capture.json`.
#[component]
pub fn Library(
    #[prop(into)] rows: Signal<Vec<MissionRow>>,
    #[prop(into)] reopenable: Signal<Vec<String>>,
    #[prop(into)] status: Signal<Option<String>>,
    #[prop(into)] failure: Signal<Option<String>>,
    tauri: bool,
    on_pick: Callback<Vec<web_sys::File>>,
    on_open: Callback<String>,
    on_open_path: Callback<String>,
    on_launch: Callback<()>,
) -> impl IntoView {
    let path = RwSignal::new(String::new());
    let folder: NodeRef<Input> = NodeRef::new();
    Effect::new(move |_| {
        if let Some(input) = folder.get() {
            let _ = input.set_attribute("webkitdirectory", "");
        }
    });
    let entries = move || library(&rows.get());

    view! {
        <section class="library" aria-label="library">
            <h1>"Mission library"</h1>
            <Show
                when=move || !entries().is_empty()
                fallback=|| {
                    view! {
                        <p aria-label="library-empty">
                            "No capture opened yet. Open one below."
                        </p>
                    }
                }
            >
                <table aria-label="library-table">
                    <thead>
                        <tr>
                            <th>"Name"</th>
                            <th>"Date"</th>
                            <th>"Events"</th>
                            <th></th>
                        </tr>
                    </thead>
                    <tbody>
                        <For
                            each=entries
                            key=|entry| (entry.mission_id.clone(), entry.event_count)
                            let(entry)
                        >
                            {
                                let id = entry.mission_id.0.clone();
                                let open_id = id.clone();
                                view! {
                                    <tr aria-label="library-row" data-testid="library-row">
                                        <td aria-label="library-name" data-testid="library-name">{entry.name.clone()}</td>
                                        <td aria-label="library-date">{entry.date.clone()}</td>
                                        <td aria-label="library-events">{entry.event_count}</td>
                                        <td>
                                            <Show
                                                when=move || reopenable.get().contains(&id)
                                                fallback=|| {
                                                    view! { <span>"pick its files again to open"</span> }
                                                }
                                            >
                                                <button
                                                    aria-label="library-open" data-testid="library-open"
                                                    on:click={
                                                        let open_id = open_id.clone();
                                                        move |_| on_open.run(open_id.clone())
                                                    }
                                                >
                                                    "Open"
                                                </button>
                                            </Show>
                                        </td>
                                    </tr>
                                }
                            }
                        </For>
                    </tbody>
                </table>
            </Show>

            <button type="button" aria-label="open-launch" data-testid="open-launch" on:click=move |_| on_launch.run(())>
                "Launch a mission"
            </button>

            <AttachPanel />

            {tauri.then(|| {
                view! {
                    <h2>"Open a capture"</h2>
                    <p>
                        "Type the path of the root session .jsonl (or its .capture.json); its child sessions are read with it."
                    </p>
                    <form
                        class="picker"
                        on:submit=move |ev| {
                            ev.prevent_default();
                            on_open_path.run(path.get_untracked());
                        }
                    >
                        <label>
                            "Capture path "
                            <input
                                type="text"
                                aria-label="open-path" data-testid="open-path"
                                prop:value=move || path.get()
                                on:input=move |ev| path.set(event_target_value(&ev))
                            />
                        </label>
                        <button type="submit" aria-label="open-path-go" data-testid="open-path-go">"Open"</button>
                    </form>
                }
            })}
            {(!tauri).then(|| {
                view! {
                    <h2>"Open a capture"</h2>
                    <p>
                        "Pick the root session .jsonl together with its .children/*.jsonl, the .settings.json sidecars and the .capture.json, or pick the folder that holds them."
                    </p>
                    <label class="picker">
                        "Capture files "
                        <input
                            type="file"
                            multiple
                            accept=".jsonl,.json"
                            aria-label="open-files"
                            on:change=move |ev| on_pick.run(files_of(&ev))
                        />
                    </label>
                    <label class="picker">
                        "Capture folder "
                        <input
                            node_ref=folder
                            type="file"
                            multiple
                            aria-label="open-folder"
                            on:change=move |ev| on_pick.run(files_of(&ev))
                        />
                    </label>
                }
            })}
            <p role="status" aria-label="load-status">{move || status.get()}</p>
            <Show when=move || failure.get().is_some()>
                <p role="alert" class="error" aria-label="load-error">
                    {move || failure.get()}
                </p>
            </Show>
        </section>
    }
}
