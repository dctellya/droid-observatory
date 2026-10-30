use leptos::prelude::*;
use observatory_core::model::{EventId, FeatureId};
use observatory_core::view::{CommitsTab, EventRow};

fn feature_text(feature: &Option<FeatureId>) -> String {
    feature
        .as_ref()
        .map_or_else(|| "-".to_owned(), |feature| feature.0.clone())
}

fn raw_line_button(id: &EventId, selected: RwSignal<Option<EventId>>) -> AnyView {
    let open = id.clone();
    view! {
        <button
            type="button"
            class="mono"
            aria-label="raw-line-open"
            on:click=move |_| selected.set(Some(open.clone()))
        >
            {id.0.clone()}
        </button>
    }
    .into_any()
}

fn event_section(
    title: &'static str,
    table_label: &'static str,
    row_label: &'static str,
    rows: &[EventRow],
    selected: RwSignal<Option<EventId>>,
) -> AnyView {
    let body = rows
        .iter()
        .map(|row| {
            view! {
                <tr aria-label=row_label>
                    <td class="mono">{row.at.to_rfc3339_millis()}</td>
                    <td class="mono">{feature_text(&row.feature)}</td>
                    <td>{row.lane.clone()}</td>
                    <td>{row.text.clone()}</td>
                    <td>{raw_line_button(&row.event_id, selected)}</td>
                </tr>
            }
        })
        .collect_view();
    let none = rows.is_empty().then(|| view! { <p>"none"</p> });
    view! {
        <h3>{title}</h3>
        <div class="lib"><div class="scroll">
            <table aria-label=table_label>
                <thead>
                    <tr>
                        <th>"Time"</th>
                        <th>"Feature"</th>
                        <th>"Phase"</th>
                        <th>"Entry"</th>
                        <th>"Event"</th>
                    </tr>
                </thead>
                <tbody>{body}</tbody>
            </table>
        </div></div>
        {none}
    }
    .into_any()
}

/// FR-D4: the commits of one milestone, its worktree, integration into the mission branch,
/// conflicts and fast checks.
#[component]
pub fn CommitsTabView(tab: CommitsTab, selected: RwSignal<Option<EventId>>) -> impl IntoView {
    let worktree = match &tab.worktree {
        Some(name) => view! { <p aria-label="commits-worktree">"Worktree: " <span class="mono">{name.clone()}</span></p> }
            .into_any(),
        None => view! { <p aria-label="commits-worktree">"Worktree: none listed"</p> }.into_any(),
    };
    let commit_rows = tab
        .commits
        .iter()
        .map(|row| {
            view! {
                <tr aria-label="commit-row">
                    <td class="mono">{row.at.to_rfc3339_millis()}</td>
                    <td class="mono">{feature_text(&row.feature)}</td>
                    <td class="mono">{row.reference.clone()}</td>
                    <td>{raw_line_button(&row.event_id, selected)}</td>
                </tr>
            }
        })
        .collect_view();
    let no_commits = tab.commits.is_empty().then(|| view! { <p>"none"</p> });
    view! {
        <section class="details-tab" aria-label="details-commits-integration">
            {worktree}
            <p aria-label="commits-branch">"Mission branch: " <span class="mono">{tab.branch.clone()}</span></p>
            <h3>"Commits"</h3>
            <p>"Commits: " <span class="mono" aria-label="commits-count">{tab.commits.len()}</span></p>
            <div class="lib"><div class="scroll">
                <table aria-label="commits-table">
                    <thead>
                        <tr>
                            <th>"Time"</th>
                            <th>"Feature"</th>
                            <th>"Reference"</th>
                            <th>"Event"</th>
                        </tr>
                    </thead>
                    <tbody>{commit_rows}</tbody>
                </table>
            </div></div>
            {no_commits}
            {event_section("Integration", "integration-rows", "integration-row", &tab.integration, selected)}
            {event_section("Conflicts", "conflict-rows", "conflict-row", &tab.conflicts, selected)}
            {event_section("Fast checks", "check-rows", "check-row", &tab.checks, selected)}
        </section>
    }
}
