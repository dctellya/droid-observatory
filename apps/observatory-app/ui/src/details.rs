use leptos::prelude::*;
use observatory_core::metrics::{Notes, PriceTable};
use observatory_core::model::{CycleTemplate, EventId, MilestoneId};
use observatory_core::view::{
    TimelineRow, commits_tab, cost_tab, criteria_tab, details, gate_tab, review_tab,
};

use crate::capture::Capture;
use crate::ctx::BoardCtx;
use crate::gate::GateGroupBox;
use crate::raw::RawLine;
use crate::tabs::commits::CommitsTabView;
use crate::tabs::cost::CostTabView;
use crate::tabs::criteria::CriteriaTabView;
use crate::tabs::gate::GateTabView;
use crate::tabs::review::ReviewTabView;

/// The price table the Details name on every cost; the browser host has no config to name another.
pub const PRICES: &str = include_str!("../../../../config/prices/2026-10-22.toml");

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Timeline,
    Cost,
    Review,
    Criteria,
    Commits,
    Gate,
}

impl Tab {
    const ALL: [Tab; 6] = [
        Tab::Timeline,
        Tab::Cost,
        Tab::Review,
        Tab::Criteria,
        Tab::Commits,
        Tab::Gate,
    ];

    fn label(self) -> &'static str {
        match self {
            Tab::Timeline => "Timeline",
            Tab::Cost => "Cost and waste",
            Tab::Review => "Review notes",
            Tab::Criteria => "Validation criteria",
            Tab::Commits => "Commits and integration",
            Tab::Gate => "Gate",
        }
    }
}

/// The Details of one milestone, opened from its card or from any event row: the Timeline (FR-D1)
/// and the tabs Cost and waste (FR-D6), Review notes (FR-D2), Validation criteria (FR-D3), Commits
/// and integration (FR-D4) and Gate (FR-D5), each from its `core::view` function.
#[component]
pub fn DetailsView(capture: Capture, milestone: RwSignal<Option<MilestoneId>>) -> impl IntoView {
    let selected = RwSignal::new(None::<EventId>);
    Effect::new(move |_| {
        milestone.track();
        selected.set(None);
    });
    let tab = RwSignal::new(Tab::Timeline);
    let ctx = expect_context::<BoardCtx>();
    let (gate, worktrees) = (ctx.gate, ctx.worktrees.clone());
    let template = expect_context::<RwSignal<CycleTemplate>>();
    let prices = PriceTable::parse(PRICES).ok();
    let events = capture.events.clone();
    let branch = capture.branch.clone();
    let model = Memo::new({
        let events = events.clone();
        move |_| {
            milestone
                .get()
                .map(|milestone| events.with(|events| details(events, &milestone)))
        }
    });
    let other_tab = {
        let events = events.clone();
        move |tab: Tab, milestone: &MilestoneId| -> AnyView {
            let notes = Notes::default();
            template.with(|template| {
                events.with(|events| match tab {
                    Tab::Timeline => ().into_any(),
                    Tab::Cost => match &prices {
                        Some(prices) => {
                            let tab = cost_tab(events, milestone, template, prices, &notes);
                            view! { <CostTabView tab=tab /> }.into_any()
                        }
                        None => view! { <p class="error" role="alert">"The price table did not load."</p> }
                            .into_any(),
                    },
                    Tab::Review => {
                        let tab = review_tab(events, milestone, template, &notes);
                        view! { <ReviewTabView tab=tab selected=selected /> }.into_any()
                    }
                    Tab::Criteria => {
                        let tab = criteria_tab(events, milestone, &notes);
                        view! { <CriteriaTabView tab=tab selected=selected /> }.into_any()
                    }
                    Tab::Commits => {
                        let worktree = worktrees
                            .with(|all| all.iter().find(|w| &w.milestone == milestone).cloned());
                        let tab =
                            commits_tab(events, milestone, template, &branch, worktree.as_ref());
                        view! { <CommitsTabView tab=tab selected=selected /> }.into_any()
                    }
                    Tab::Gate => {
                        let tab = gate_tab(&gate.get(), events);
                        view! { <GateTabView tab=tab selected=selected /> }.into_any()
                    }
                })
            })
        }
    };
    move || {
        model.get().map(|model| {
            let milestone_id = model.milestone.clone();
            let other_tab = other_tab.clone();
            let timeline = model.timeline.clone();
            view! {
                <section class="details" role="dialog" aria-label="details">
                    <header>
                        <h2>"Details: " <span class="mono" aria-label="details-milestone">{model.milestone.0.clone()}</span></h2>
                        <div role="tablist" aria-label="details-tabs">
                            {Tab::ALL
                                .into_iter()
                                .map(|this| {
                                    view! {
                                        <button
                                            type="button"
                                            role="tab"
                                            aria-selected=move || (tab.get() == this).to_string()
                                            on:click=move |_| tab.set(this)
                                        >
                                            {this.label()}
                                        </button>
                                    }
                                })
                                .collect_view()}
                        </div>
                        <button type="button" aria-label="details-close" on:click=move |_| milestone.set(None)>
                            "Close details"
                        </button>
                    </header>
                    <GateGroupBox view=gate />
                    <RawLine capture=capture.clone() selected=selected />
                    {move || (tab.get() != Tab::Timeline).then(|| other_tab(tab.get(), &milestone_id))}
                    {move || {
                        (tab.get() == Tab::Timeline).then(|| {
                            let rows = timeline
                                .iter()
                                .map(|row| timeline_row(row, selected))
                                .collect_view();
                            view! {
                    <p>
                        "Timeline rows: "
                        <span class="mono" aria-label="details-row-count">{timeline.len()}</span>
                    </p>
                    <div class="lib"><div class="scroll">
                    <table aria-label="details-timeline">
                        <thead>
                            <tr>
                                <th>"Time"</th>
                                <th>"Event"</th>
                                <th>"Phase"</th>
                                <th>"Overlay"</th>
                                <th>"Kind"</th>
                                <th>"Role"</th>
                                <th>"Model"</th>
                                <th>"Duration"</th>
                                <th>"Raw line"</th>
                            </tr>
                        </thead>
                        <tbody>{rows}</tbody>
                    </table>
                    </div></div>
                            }
                        })
                    }}
                </section>
            }
        })
    }
}

fn timeline_row(row: &TimelineRow, selected: RwSignal<Option<EventId>>) -> AnyView {
    let template = expect_context::<RwSignal<CycleTemplate>>();
    let id = row.event_id.clone();
    let open = row.event_id.clone();
    view! {
        <tr aria-label="timeline-row" class:unmapped=row.unmapped>
            <td class="mono">{row.at.to_rfc3339_millis()}</td>
            <td class="event-id mono">{id.0}</td>
            <td>{template.with(|template| template.name_of(&row.lane))}</td>
            <td>{row.overlay.map(|overlay| overlay.as_str())}</td>
            <td class="mono">{row.kind.clone()}</td>
            <td>{row.role.as_str()}</td>
            <td class="mono">{row.model.as_ref().map(|model| model.0.clone())}</td>
            <td class="n">{row.duration_ms.map(|ms| format!("{ms} ms"))}</td>
            <td>
                <button
                    type="button"
                    aria-label="raw-line-open"
                    on:click=move |_| selected.set(Some(open.clone()))
                >
                    "Open"
                </button>
            </td>
        </tr>
    }
    .into_any()
}
