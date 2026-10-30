use leptos::prelude::*;
use observatory_core::metrics::{CostReport, Throttled, TokenCounts, WasteCause};
use observatory_core::model::CycleTemplate;
use observatory_core::view::PanelMetrics;

const NO_MILESTONE: &str = "(none)";
const UNKNOWN_MODEL: &str = "(unknown)";
const NOT_AVAILABLE: &str = "n/a";

fn cost_cell(amount: f64, price_table: &str) -> AnyView {
    view! { <td class="n" title=price_table.to_owned()>{format!("{amount:.2}")}</td> }.into_any()
}

fn token_cells(tokens: TokenCounts) -> AnyView {
    view! {
        <td class="n">{tokens.input}</td>
        <td class="n">{tokens.output}</td>
        <td class="n">{tokens.cache_read}</td>
    }
    .into_any()
}

fn roles_table(cost: &CostReport) -> AnyView {
    let rows = cost
        .tokens_by_role
        .iter()
        .map(|row| {
            let priced = cost.cost_by_role.iter().find(|c| c.role == row.role);
            let multiplier = priced.map_or_else(
                || NOT_AVAILABLE.to_owned(),
                |c| format!("{} x{}", c.multiplier_key, c.multiplier),
            );
            let amount = priced.map(|c| c.cost.amount);
            view! {
                <tr aria-label=format!("usage-role-{}", row.role.as_str())>
                    <td>{row.role.as_str()}</td>
                    {token_cells(row.tokens)}
                    {match amount {
                        Some(amount) => cost_cell(amount, &cost.price_table),
                        None => view! { <td class="n">{NOT_AVAILABLE}</td> }.into_any(),
                    }}
                    <td class="mono">{multiplier}</td>
                </tr>
            }
        })
        .collect_view();
    view! {
        <h4>"Tokens and cost per role"</h4>
        <div class="lib"><div class="scroll">
        <table aria-label="usage-roles">
            <thead>
                <tr>
                    <th>"Role"</th>
                    <th>"Input"</th>
                    <th>"Output"</th>
                    <th>"Cache read"</th>
                    <th>"Cost"</th>
                    <th>"Multiplier"</th>
                </tr>
            </thead>
            <tbody>{rows}</tbody>
        </table>
        </div></div>
    }
    .into_any()
}

fn lanes_table(cost: &CostReport, template: RwSignal<CycleTemplate>) -> AnyView {
    let rows = cost
        .tokens_by_lane
        .iter()
        .map(|row| {
            let name = template.with(|t| t.name_of(&row.lane));
            view! {
                <tr aria-label=format!("usage-lane-{}", row.lane)>
                    <td>{name}</td>
                    {token_cells(row.tokens)}
                    {cost_cell(row.cost.amount, &cost.price_table)}
                </tr>
            }
        })
        .collect_view();
    let other = (!cost.tokens_other_lanes.is_zero()).then(|| {
        view! {
            <tr aria-label="usage-lane-other">
                <td>"other lanes"</td>
                {token_cells(cost.tokens_other_lanes)}
                <td class="n">{NOT_AVAILABLE}</td>
            </tr>
        }
    });
    view! {
        <h4>"Tokens and cost per lane"</h4>
        <div class="lib"><div class="scroll">
        <table aria-label="usage-lanes">
            <thead>
                <tr>
                    <th>"Lane"</th>
                    <th>"Input"</th>
                    <th>"Output"</th>
                    <th>"Cache read"</th>
                    <th>"Cost"</th>
                </tr>
            </thead>
            <tbody>{rows}{other}</tbody>
        </table>
        </div></div>
    }
    .into_any()
}

fn milestones_table(cost: &CostReport) -> AnyView {
    let rows = cost
        .tokens_by_milestone
        .iter()
        .map(|row| {
            let name = row
                .milestone
                .as_ref()
                .map_or_else(|| NO_MILESTONE.to_owned(), |m| m.0.clone());
            let label = format!("usage-milestone-{name}");
            view! {
                <tr aria-label=label>
                    <td class="mono">{name}</td>
                    {token_cells(row.tokens)}
                    {cost_cell(row.cost.amount, &cost.price_table)}
                </tr>
            }
        })
        .collect_view();
    view! {
        <h4>"Tokens and cost per milestone"</h4>
        <div class="lib"><div class="scroll">
        <table aria-label="usage-milestones">
            <thead>
                <tr>
                    <th>"Milestone"</th>
                    <th>"Input"</th>
                    <th>"Output"</th>
                    <th>"Cache read"</th>
                    <th>"Cost"</th>
                </tr>
            </thead>
            <tbody>{rows}</tbody>
        </table>
        </div></div>
    }
    .into_any()
}

fn waste_tokens(cost: &CostReport) -> AnyView {
    let waste = &cost.tokens_in_waste_by_cause;
    let rows = WasteCause::ALL
        .into_iter()
        .map(|cause| {
            let tokens = waste.by_cause.get(&cause).copied().unwrap_or_default();
            view! {
                <tr aria-label=format!("usage-waste-{}", cause.name())>
                    <td class="mono">{cause.name()}</td>
                    {token_cells(tokens)}
                </tr>
            }
        })
        .collect_view();
    let throttled = format!(
        "Throttled (apart from the eight causes): {} / {} / {}",
        waste.throttled.input, waste.throttled.output, waste.throttled.cache_read
    );
    view! {
        <h4>"Tokens burnt in waste"</h4>
        <div class="lib"><div class="scroll">
        <table aria-label="usage-waste-tokens">
            <thead>
                <tr>
                    <th>"Cause"</th>
                    <th>"Input"</th>
                    <th>"Output"</th>
                    <th>"Cache read"</th>
                </tr>
            </thead>
            <tbody>{rows}</tbody>
        </table>
        </div></div>
        <p aria-label="usage-throttled-tokens">{throttled}</p>
    }
    .into_any()
}

fn throttled_block(throttled: &Throttled) -> AnyView {
    let milestones = if throttled.per_milestone.is_empty() {
        view! { <p aria-label="throttled-milestones-none">"none"</p> }.into_any()
    } else {
        let rows = throttled
            .per_milestone
            .iter()
            .map(|(milestone, minutes)| {
                view! {
                    <tr aria-label=format!("throttled-milestone-{}", milestone.0)>
                        <td class="mono">{milestone.0.clone()}</td>
                        <td class="n">{*minutes}</td>
                    </tr>
                }
            })
            .collect_view();
        view! {
            <div class="lib"><div class="scroll">
            <table aria-label="throttled-milestones">
                <thead>
                    <tr>
                        <th>"Milestone"</th>
                        <th>"Minutes"</th>
                    </tr>
                </thead>
                <tbody>{rows}</tbody>
            </table>
            </div></div>
        }
        .into_any()
    };
    let intervals = if throttled.intervals.is_empty() {
        view! { <p aria-label="throttled-intervals-none">"none"</p> }.into_any()
    } else {
        let rows = throttled
            .intervals
            .iter()
            .enumerate()
            .map(|(index, interval)| {
                let milestone = interval
                    .milestone
                    .as_ref()
                    .map_or_else(|| NO_MILESTONE.to_owned(), |m| m.0.clone());
                view! {
                    <tr aria-label=format!("throttled-interval-{index}")>
                        <td class="mono">{interval.from.to_rfc3339_millis()}</td>
                        <td class="mono">{interval.to.to_rfc3339_millis()}</td>
                        <td>{interval.reason.clone()}</td>
                        <td class="n">{interval.minutes}</td>
                        <td class="mono">{milestone}</td>
                    </tr>
                }
            })
            .collect_view();
        view! {
            <div class="lib"><div class="scroll">
            <table aria-label="throttled-intervals">
                <thead>
                    <tr>
                        <th>"From"</th>
                        <th>"To"</th>
                        <th>"Reason"</th>
                        <th>"Minutes"</th>
                        <th>"Milestone"</th>
                    </tr>
                </thead>
                <tbody>{rows}</tbody>
            </table>
            </div></div>
        }
        .into_any()
    };
    view! {
        <div aria-label="panel-throttled" data-testid="panel-throttled">
            <h4>"Throttled (not a waste group)"</h4>
            <p aria-label="throttled-run">{format!("Run: {} min", throttled.mission_minutes)}</p>
            {milestones}
            {intervals}
        </div>
    }
    .into_any()
}

fn budget_line(cost: &CostReport) -> AnyView {
    let budget = &cost.budget;
    let text = match (budget.enabled, budget.budget, budget.fraction) {
        (true, Some(limit), Some(fraction)) => format!(
            "Cost so far {:.2} of budget {:.2} {} ({:.1}%) price table {}",
            budget.cost_so_far,
            limit,
            budget.currency,
            fraction * 100.0,
            budget.price_table
        ),
        _ => "Budget: off (no budget set)".to_owned(),
    };
    view! { <p aria-label="usage-budget">{text}</p> }.into_any()
}

fn comparison_table(cost: &CostReport) -> AnyView {
    let rows = cost
        .model_comparison
        .iter()
        .map(|row| {
            let model = row
                .model
                .as_ref()
                .map_or_else(|| UNKNOWN_MODEL.to_owned(), |m| m.0.clone());
            let per_feature = row
                .cost_per_completed_feature
                .map_or_else(|| NOT_AVAILABLE.to_owned(), |v| format!("{v:.2}"));
            let first_pass = row
                .first_pass_active_s_per_completed_feature
                .map_or_else(|| NOT_AVAILABLE.to_owned(), |v| format!("{v:.1}"));
            let label = format!("usage-comparison-{}-{}", row.role.as_str(), model);
            view! {
                <tr aria-label=label>
                    <td>{row.role.as_str()}</td>
                    <td class="mono">{model}</td>
                    {cost_cell(row.cost.amount, &cost.price_table)}
                    <td class="n">{per_feature}</td>
                    <td class="n">{first_pass}</td>
                    <td class="n">{row.completed_features}</td>
                </tr>
            }
        })
        .collect_view();
    view! {
        <h4>"Model comparison per role and model"</h4>
        <div class="lib"><div class="scroll">
        <table aria-label="usage-model-comparison">
            <thead>
                <tr>
                    <th>"Role"</th>
                    <th>"Model"</th>
                    <th>"Cost"</th>
                    <th>"Cost per completed feature"</th>
                    <th>"First-pass active s per completed feature"</th>
                    <th>"Completed features"</th>
                </tr>
            </thead>
            <tbody>{rows}</tbody>
        </table>
        </div></div>
    }
    .into_any()
}

/// The Panel's usage block: tokens and cost per role, lane and milestone, tokens burnt in waste,
/// throttled time, the budget meter and the model comparison.
#[component]
pub fn UsageBlock(metrics: Memo<Option<PanelMetrics>>) -> impl IntoView {
    let template = expect_context::<RwSignal<CycleTemplate>>();
    view! {
        <section aria-label="panel-usage" data-testid="panel-usage">
            <h3>"Usage"</h3>
            {move || {
                metrics
                    .with(|found| match found {
                        None => view! { <p>"No metrics yet"</p> }.into_any(),
                        Some(panel) => {
                            let cost = &panel.cost;
                            view! {
                                <p aria-label="usage-price-table">
                                    {format!("Price table: {} ({})", cost.price_table, cost.currency)}
                                </p>
                                {roles_table(cost)}
                                {lanes_table(cost, template)}
                                {milestones_table(cost)}
                                {waste_tokens(cost)}
                                {throttled_block(&panel.throttled)}
                                {budget_line(cost)}
                                {comparison_table(cost)}
                            }
                            .into_any()
                        }
                    })
            }}
        </section>
    }
}
