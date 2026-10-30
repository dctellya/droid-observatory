use leptos::prelude::*;
use observatory_core::model::EventId;
use observatory_core::view::ReviewTab;

fn raw_line_button(id: EventId, selected: RwSignal<Option<EventId>>) -> AnyView {
    let text = id.to_string();
    view! {
        <button
            type="button"
            class="mono"
            aria-label="raw-line-open"
            on:click=move |_| selected.set(Some(id.clone()))
        >
            {text}
        </button>
    }
    .into_any()
}

#[component]
pub fn ReviewTabView(tab: ReviewTab, selected: RwSignal<Option<EventId>>) -> impl IntoView {
    let findings_count = tab.findings.len();
    let scenarios_count = tab.scenarios.len();

    let finding_rows = tab
        .findings
        .into_iter()
        .map(|row| {
            let severity_class = if row.severity == "blocking" {
                "chip bad"
            } else {
                "chip pause"
            };
            let status_class = if row.status == "resolved" {
                "chip run"
            } else {
                "chip pause"
            };
            view! {
                <tr aria-label="finding-row">
                    <td class="mono">{row.at.to_rfc3339_millis()}</td>
                    <td class="mono">{row.feature.as_ref().map(|f| f.to_string())}</td>
                    <td>
                        <span class=severity_class>{row.severity.clone()}</span>
                    </td>
                    <td>
                        <span class=status_class>{row.status.clone()}</span>
                    </td>
                    <td>{raw_line_button(row.event_id.clone(), selected)}</td>
                </tr>
            }
        })
        .collect_view();

    let scenario_rows = tab
        .scenarios
        .into_iter()
        .map(|row| {
            let result_class = if row.result == "passed" {
                "chip run"
            } else {
                "chip bad"
            };
            let evidence = if row.evidence.is_empty() {
                view! { <span>"none"</span> }.into_any()
            } else {
                row.evidence
                    .iter()
                    .map(|id| raw_line_button(id.clone(), selected))
                    .collect_view()
                    .into_any()
            };
            view! {
                <tr aria-label="scenario-row">
                    <td class="mono">{row.at.to_rfc3339_millis()}</td>
                    <td class="mono">{row.feature.as_ref().map(|f| f.to_string())}</td>
                    <td class="mono">{row.key.clone().unwrap_or_else(|| "-".to_owned())}</td>
                    <td>
                        <span class=result_class>{row.result.clone()}</span>
                    </td>
                    <td>{evidence}</td>
                    <td>{raw_line_button(row.event_id.clone(), selected)}</td>
                </tr>
            }
        })
        .collect_view();

    let loose = if tab.loose_evidence.is_empty() {
        None
    } else {
        let buttons = tab
            .loose_evidence
            .iter()
            .map(|id| raw_line_button(id.clone(), selected))
            .collect_view();
        Some(view! {
            <p aria-label="review-loose-evidence">
                "Evidence without a scenario result:" {buttons}
            </p>
        })
    };

    view! {
        <section class="details-tab" aria-label="details-review-notes">
            <h3>"Scrutiny findings"</h3>
            <p>
                "Findings: "
                <span class="mono" aria-label="review-findings-count">
                    {findings_count}
                </span>
            </p>
            {if findings_count == 0 {
                view! { <p>"none"</p> }.into_any()
            } else {
                view! {
                    <div class="lib">
                        <div class="scroll">
                            <table aria-label="review-findings">
                                <thead>
                                    <tr>
                                        <th>"Time"</th>
                                        <th>"Feature"</th>
                                        <th>"Severity"</th>
                                        <th>"Status"</th>
                                        <th>"Event"</th>
                                    </tr>
                                </thead>
                                <tbody>{finding_rows}</tbody>
                            </table>
                        </div>
                    </div>
                }
                .into_any()
            }}
            <h3>"User-testing scenarios"</h3>
            <p>
                "Scenario results: "
                <span class="mono" aria-label="review-scenarios-count">
                    {scenarios_count}
                </span>
            </p>
            {if scenarios_count == 0 {
                view! { <p>"none"</p> }.into_any()
            } else {
                view! {
                    <div class="lib">
                        <div class="scroll">
                            <table aria-label="review-scenarios">
                                <thead>
                                    <tr>
                                        <th>"Time"</th>
                                        <th>"Feature"</th>
                                        <th>"Scenario"</th>
                                        <th>"Result"</th>
                                        <th>"Evidence"</th>
                                        <th>"Event"</th>
                                    </tr>
                                </thead>
                                <tbody>{scenario_rows}</tbody>
                            </table>
                        </div>
                    </div>
                }
                .into_any()
            }}
            {loose}
        </section>
    }
}
