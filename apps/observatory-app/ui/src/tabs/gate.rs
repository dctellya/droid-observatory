use leptos::prelude::*;
use observatory_core::model::EventId;
use observatory_core::view::GateTab;

fn conclusion_class(conclusion: &str) -> &'static str {
    match conclusion {
        "passed" => "chip run",
        "failed" | "required_missing" => "chip bad",
        "queued" | "running" => "chip pause",
        _ => "chip done",
    }
}

/// FR-D5: the mission PR, its reviews, checks and attestations, the merge policy, the merge
/// history and the clocks.
#[component]
pub fn GateTabView(tab: GateTab, selected: RwSignal<Option<EventId>>) -> impl IntoView {
    let state_class = if tab.state == "Pending" {
        "chip done"
    } else {
        "chip pause"
    };
    let pr = match &tab.pr {
        Some(pr) => {
            let url = pr
                .url
                .clone()
                .map(|url| view! { " " <span class="mono">{url}</span> });
            view! { <p aria-label="gate-tab-pr">"Mission PR: #" {pr.number} {url}</p> }.into_any()
        }
        None => view! {
            <p aria-label="gate-tab-pr">"Mission PR: none (no PR into the mission branch's target yet)"</p>
        }
        .into_any(),
    };
    let checks = tab
        .checks
        .iter()
        .map(|check| {
            view! {
                <li aria-label=format!("gate-tab-check-{}", check.name)>
                    <span class="mono">{check.name.clone()}</span>
                    " "
                    <span class=conclusion_class(&check.conclusion)>{check.conclusion.clone()}</span>
                </li>
            }
        })
        .collect_view();
    let no_checks = tab.checks.is_empty().then(|| view! { <p>"none"</p> });
    let attestations = tab
        .attestations
        .iter()
        .map(|text| view! { <li class="mono">{text.clone()}</li> })
        .collect_view();
    let no_attestations = tab.attestations.is_empty().then(|| view! { <p>"none"</p> });
    let history = tab
        .history
        .iter()
        .map(|row| {
            let open = row.event_id.clone();
            view! {
                <li aria-label="gate-tab-history-row">
                    <span class="mono">{row.at.to_rfc3339_millis()}</span>
                    " "
                    {row.text.clone()}
                    " "
                    <button
                        type="button"
                        class="mono"
                        aria-label="raw-line-open"
                        on:click=move |_| selected.set(Some(open.clone()))
                    >
                        {row.event_id.0.clone()}
                    </button>
                </li>
            }
        })
        .collect_view();
    let no_history = tab.history.is_empty().then(|| view! { <p>"none"</p> });
    let clocks = tab
        .clocks
        .iter()
        .map(|clock| {
            let value = clock.value.clone().unwrap_or_else(|| "no value".to_owned());
            view! {
                <li aria-label="gate-tab-clock">
                    {clock.label.clone()}
                    ": "
                    <span class="mono">{value}</span>
                    " (" {clock.note.clone()} ")"
                </li>
            }
        })
        .collect_view();
    view! {
        <section class="details-tab" aria-label="details-gate">
            <p>"Gate state: " <span class=state_class aria-label="gate-tab-state">{tab.state.clone()}</span></p>
            {pr}
            <h3>"Reviews"</h3>
            <p aria-label="gate-tab-reviews">{tab.reviews_text.clone()}</p>
            <h3>"Checks"</h3>
            <ul aria-label="gate-tab-checks">{checks}</ul>
            {no_checks}
            <h3>"Attestations"</h3>
            <ul aria-label="gate-tab-attestations">{attestations}</ul>
            {no_attestations}
            <h3>"Merge policy"</h3>
            <p aria-label="gate-tab-policy">{tab.policy_text.clone()}</p>
            <h3>"Merge history"</h3>
            <ul aria-label="gate-tab-history">{history}</ul>
            {no_history}
            <h3>"Clocks"</h3>
            <ul aria-label="gate-tab-clocks">{clocks}</ul>
        </section>
    }
}
