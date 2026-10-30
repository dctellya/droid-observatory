use leptos::prelude::*;
use observatory_core::model::EventId;
use observatory_core::view::CriteriaTab;

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
pub fn CriteriaTabView(tab: CriteriaTab, selected: RwSignal<Option<EventId>>) -> impl IntoView {
    let feature_count = tab.features.len();

    let features = tab
        .features
        .into_iter()
        .map(|feature| {
            let section_label = format!("criteria-feature-{}", feature.feature.0);
            let plan_line = feature.defined_by.clone().map(|id| {
                view! {
                    <button
                        type="button"
                        aria-label="plan-line-open"
                        on:click=move |_| selected.set(Some(id.clone()))
                    >
                        "Plan line"
                    </button>
                }
            });
            let rows = feature
                .criteria
                .into_iter()
                .map(|row| {
                    let status_class = if row.status == "satisfied" {
                        "chip run"
                    } else {
                        "chip bad"
                    };
                    let satisfied_by = if row.satisfied_by.is_empty() {
                        view! { <span aria-label="criterion-unsatisfied">"unsatisfied"</span> }
                            .into_any()
                    } else {
                        row.satisfied_by
                            .iter()
                            .map(|id| raw_line_button(id.clone(), selected))
                            .collect_view()
                            .into_any()
                    };
                    view! {
                        <tr aria-label="criterion-row">
                            <td>{row.label.clone()}</td>
                            <td>
                                <span class=status_class>{row.status.clone()}</span>
                            </td>
                            <td>{satisfied_by}</td>
                        </tr>
                    }
                })
                .collect_view();
            view! {
                <section aria-label=section_label>
                    <h3>
                        <span class="mono">{feature.feature.0.clone()}</span>
                        " "
                        {plan_line}
                    </h3>
                    <div class="lib">
                        <div class="scroll">
                            <table aria-label="criteria-table">
                                <thead>
                                    <tr>
                                        <th>"Criterion"</th>
                                        <th>"Status"</th>
                                        <th>"Satisfied by"</th>
                                    </tr>
                                </thead>
                                <tbody>{rows}</tbody>
                            </table>
                        </div>
                    </div>
                </section>
            }
        })
        .collect_view();

    view! {
        <section class="details-tab" aria-label="details-validation-criteria">
            <p aria-label="criteria-note">
                "Criteria are the checkpoints the cycle records per feature; the plan's own text is on the plan line."
            </p>
            <p>
                "Features: "
                <span class="mono" aria-label="criteria-feature-count">
                    {feature_count}
                </span>
            </p>
            {if feature_count == 0 { Some(view! { <p>"none"</p> }) } else { None }}
            {features}
        </section>
    }
}
