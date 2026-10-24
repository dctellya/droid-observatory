use crate::capture::Capture;
use crate::launch::{HostedCtx, LiveOutputView};
use leptos::prelude::*;
use observatory_core::model::{SessionId, Timestamp};
use observatory_core::view::{
    AgentEntry, AgentStatus, AgentStream, BoardView, DeclaredModels, MarkerKind, TimelineRow,
    agent_entries, agent_stream,
};

fn status_text(status: AgentStatus) -> &'static str {
    match status {
        AgentStatus::Active => "active",
        AgentStatus::Idle => "idle",
        AgentStatus::Waiting => "waiting",
        AgentStatus::Finished => "finished",
    }
}

/// FR-A2: every session with role, name, status, feature, model against the declared one,
/// reasoning effort and tokens. Selecting one shows its output stream (FR-A3).
#[component]
pub fn AgentPanel(
    board: Memo<BoardView>,
    capture: Capture,
    declared: DeclaredModels,
    at: RwSignal<Option<Timestamp>>,
) -> impl IntoView {
    let selected = RwSignal::new(None::<SessionId>);
    let sessions = capture.sessions.clone();
    let entries = Memo::new(move |_| {
        board.with(|b| {
            sessions
                .with(|sessions| agent_entries(&b.agents, sessions, &declared, at.get().is_none()))
        })
    });
    view! {
        <section class="agents" aria-label="agents">
            <h2>
                "Agents "
                <span class="count" aria-label="agent-count">{move || entries.with(Vec::len)}</span>
            </h2>
            <div class="agent-table">
                <table aria-label="agent-table">
                    <thead>
                        <tr>
                            <th>"Session"</th>
                            <th>"Id"</th>
                            <th>"Role"</th>
                            <th>"Status"</th>
                            <th>"Feature"</th>
                            <th>"Model"</th>
                            <th>"Declared"</th>
                            <th>"Effort"</th>
                            <th>"Input"</th>
                            <th>"Output"</th>
                            <th>"Cache read"</th>
                            <th>"Cache write"</th>
                            <th>"Markers"</th>
                        </tr>
                    </thead>
                    <tbody>
                        {move || {
                            entries
                                .with(|entries| {
                                    entries
                                        .iter()
                                        .map(|entry| agent_row(entry, selected))
                                        .collect_view()
                                })
                        }}
                    </tbody>
                </table>
            </div>
            {move || {
                selected
                    .get()
                    .map(|session| {
                        let stream = capture
                            .events
                            .with(|events| agent_stream(events, &session, at.get()));
                        let live = use_context::<HostedCtx>()
                            .filter(|hosted| {
                                hosted
                                    .output
                                    .with(|output| output.launch_id() == Some(session.0.as_str()))
                            })
                            .map(|_| view! { <LiveOutputView /> });
                        view! {
                            {stream_view(&stream, selected, &entries.get())}
                            {live}
                        }
                    })
            }}
        </section>
    }
}

fn agent_row(entry: &AgentEntry, selected: RwSignal<Option<SessionId>>) -> AnyView {
    let id = entry.session_id.clone();
    view! {
        <tr aria-label=format!("agent-{}", entry.session_id.0) class:drift=entry.drifted>
            <td>
                <button
                    type="button"
                    aria-label=format!("agent-select-{}", entry.session_id.0)
                    on:click=move |_| selected.set(Some(id.clone()))
                >
                    {entry.name.clone()}
                </button>
            </td>
            <td class="event-id">{entry.session_id.0.clone()}</td>
            <td>{entry.role.as_str()}</td>
            <td>{status_text(entry.status)}</td>
            <td>{entry.feature.as_ref().map(|f| f.0.clone())}</td>
            <td>{entry.model.as_ref().map(|m| m.0.clone())}</td>
            <td>
                {entry.declared.as_ref().map(|m| m.0.clone())}
                {entry.drifted.then_some(" (drift)")}
            </td>
            <td>{entry.effort.clone()}</td>
            <td>{entry.tokens.map(|t| t.input)}</td>
            <td>{entry.tokens.map(|t| t.output)}</td>
            <td>{entry.tokens.map(|t| t.cache_read)}</td>
            <td>{entry.tokens.map(|t| t.cache_write)}</td>
            <td>{entry.markers}</td>
        </tr>
    }
    .into_any()
}

fn stream_view(
    stream: &AgentStream,
    selected: RwSignal<Option<SessionId>>,
    entries: &[AgentEntry],
) -> AnyView {
    let name = entries
        .iter()
        .find(|entry| entry.session_id == stream.session_id)
        .map_or_else(|| stream.session_id.0.clone(), |entry| entry.name.clone());
    let rows = stream
        .rows
        .iter()
        .map(|row| {
            let marker = stream
                .markers
                .iter()
                .find(|marker| marker.event_id == row.event_id)
                .map(|marker| marker.kind);
            stream_row(row, marker)
        })
        .collect_view();
    view! {
        <section class="agent-stream" aria-label="agent-stream">
            <h3>
                "Output stream of " <span aria-label="agent-stream-name">{name}</span>
                " " <span class="count" aria-label="agent-stream-rows">{stream.rows.len()}</span>
                " rows, " <span aria-label="agent-stream-markers">{stream.markers.len()}</span>
                " markers"
            </h3>
            <button type="button" aria-label="agent-stream-close" on:click=move |_| selected.set(None)>
                "Close stream"
            </button>
            <ol class="rows-list">{rows}</ol>
        </section>
    }
    .into_any()
}

fn stream_row(row: &TimelineRow, marker: Option<MarkerKind>) -> AnyView {
    let label = marker.map(|kind| match kind {
        MarkerKind::ModelChanged => "model.changed",
        MarkerKind::ContextCompacted => "context.compacted",
    });
    view! {
        <li class="row" class:marker=marker.is_some() data-event-id=row.event_id.0.clone()>
            <time>{row.at.to_rfc3339_millis()}</time>
            <span class=format!("role role-{}", row.role.as_str())>{row.role.as_str()}</span>
            {match label {
                Some(label) => {
                    view! { <strong class="agent-marker" aria-label=label>{label}</strong> }
                        .into_any()
                }
                None => view! { <span class="kind">{row.kind.clone()}</span> }.into_any(),
            }}
            {row.model.as_ref().map(|model| view! { <span class="model">{model.0.clone()}</span> })}
        </li>
    }
    .into_any()
}
