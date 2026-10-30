use leptos::prelude::*;
use observatory_core::metrics::ReworkKind;
use observatory_core::view::{CostTab, ReworkRow, UsageRow, WasteRow};

const MISSION_LEVEL: &str = "(mission level)";

fn kind_name(kind: ReworkKind) -> &'static str {
    match kind {
        ReworkKind::Finding => "finding",
        ReworkKind::Scenario => "scenario",
        ReworkKind::Conflict => "conflict",
        ReworkKind::Check => "check",
    }
}

fn usage_row(row: &UsageRow, row_label: &'static str, date: &str) -> AnyView {
    let title = date.to_owned();
    view! {
        <tr aria-label=row_label>
            <td class="mono">{row.label.clone()}</td>
            <td class="n">{row.tokens.input}</td>
            <td class="n">{row.tokens.output}</td>
            <td class="n">{row.tokens.cache_read}</td>
            <td class="n">{row.cache_write}</td>
            <td class="n" title=title>{format!("{:.2}", row.cost.amount)}</td>
        </tr>
    }
    .into_any()
}

fn usage_table(
    heading: &'static str,
    table_label: &'static str,
    row_label: &'static str,
    rows: &[UsageRow],
    tab: &CostTab,
) -> AnyView {
    let body = rows
        .iter()
        .map(|row| usage_row(row, row_label, &tab.price_table))
        .collect_view();
    let cost_head = format!("Cost ({})", tab.currency);
    view! {
        <h3>{heading}</h3>
        <div class="lib"><div class="scroll">
        <table aria-label=table_label>
            <thead>
                <tr>
                    <th>"Label"</th>
                    <th>"Input"</th>
                    <th>"Output"</th>
                    <th>"Cache read"</th>
                    <th>"Cache write"</th>
                    <th>{cost_head}</th>
                </tr>
            </thead>
            <tbody>{body}</tbody>
        </table>
        </div></div>
    }
    .into_any()
}

fn waste_row(row: &WasteRow) -> AnyView {
    view! {
        <tr aria-label="waste-row">
            <td class="mono">{row.name}</td>
            <td class="n">{row.seconds}</td>
        </tr>
    }
    .into_any()
}

fn rework_row(row: &ReworkRow) -> AnyView {
    let feature = row
        .feature
        .as_ref()
        .map_or_else(|| MISSION_LEVEL.to_owned(), |feature| feature.0.clone());
    let closed = row
        .closed
        .map_or_else(|| "unresolved".to_owned(), |at| at.to_rfc3339_millis());
    let (chip, text) = if row.unresolved {
        ("chip bad", "unresolved")
    } else {
        ("chip done", "closed")
    };
    view! {
        <tr aria-label="rework-row">
            <td class="mono">{feature}</td>
            <td>{kind_name(row.kind)}</td>
            <td>{row.cause.clone()}</td>
            <td class="mono">{row.opened.to_rfc3339_millis()}</td>
            <td class="mono">{closed}</td>
            <td><span class=chip>{text}</span></td>
        </tr>
    }
    .into_any()
}

#[component]
pub fn CostTabView(tab: CostTab) -> impl IntoView {
    let totals = usage_table(
        "Milestone total",
        "cost-totals",
        "cost-total-row",
        std::slice::from_ref(&tab.totals),
        &tab,
    );
    let features = usage_table(
        "Per feature",
        "cost-features",
        "cost-feature-row",
        &tab.features,
        &tab,
    );
    let roles = usage_table("Per role", "cost-roles", "cost-role-row", &tab.roles, &tab);
    let unpriced = (!tab.unpriced.is_zero()).then(|| {
        view! {
            <p aria-label="cost-unpriced">
                {format!(
                    "Unpriced tokens (no price row): input {}, output {}, cache read {}",
                    tab.unpriced.input, tab.unpriced.output, tab.unpriced.cache_read
                )}
            </p>
        }
    });
    let waste = tab.waste.iter().map(waste_row).collect_view();
    let rework = tab.rework.iter().map(rework_row).collect_view();
    let rework_none = tab
        .rework
        .is_empty()
        .then(|| view! { <p>"No rework windows: none"</p> });
    view! {
        <section class="details-tab" aria-label="details-cost-and-waste">
            <p aria-label="cost-price-table">
                "Price table: " <span class="mono">{tab.price_table.clone()}</span>
                {format!(" ({})", tab.currency)}
            </p>
            {totals}
            {features}
            {roles}
            {unpriced}
            <h3>"Waste by cause"</h3>
            <div class="lib"><div class="scroll">
            <table aria-label="cost-waste">
                <thead><tr><th>"Cause"</th><th>"Seconds"</th></tr></thead>
                <tbody>{waste}</tbody>
            </table>
            </div></div>
            <h3>"Rework windows"</h3>
            <p aria-label="cost-rework-count">
                "Rework windows: " <span class="mono">{tab.rework.len()}</span>
            </p>
            {rework_none}
            <div class="lib"><div class="scroll">
            <table aria-label="cost-rework">
                <thead>
                    <tr>
                        <th>"Feature"</th>
                        <th>"Kind"</th>
                        <th>"Cause"</th>
                        <th>"Opened"</th>
                        <th>"Closed"</th>
                        <th>"Status"</th>
                    </tr>
                </thead>
                <tbody>{rework}</tbody>
            </table>
            </div></div>
        </section>
    }
}
