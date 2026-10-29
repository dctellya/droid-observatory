use leptos::prelude::*;
use observatory_core::model::{CycleTemplate, LaneGrouping};
use observatory_core::view::{
    BoardView, Card, GateGroupView, GateVerdict, LaneView, RowGroup, TimelineRow, WorkerGrouping,
    session_name,
};

use crate::controls::{FeatureMarks, stall_marker};
use crate::ctx::BoardCtx;
use crate::gate::GateGroupBox;

/// The template's lanes, left to right (F-028, D-024, F-041).
#[component]
pub fn Lanes(
    board: Memo<BoardView>,
    grouping: RwSignal<WorkerGrouping>,
    gate: Signal<GateGroupView>,
) -> impl IntoView {
    let template = expect_context::<RwSignal<CycleTemplate>>();
    view! {
        <div class="lanes">
            {move || {
                board
                    .with(|b| {
                        let command_lane = b
                            .lanes
                            .iter()
                            .find(|lane| lane.grouping == LaneGrouping::Milestones)
                            .map(|lane| lane.name.clone())
                            .unwrap_or_default();
                        let final_lane = template.with(|template| template.final_lane().clone());
                        b.lanes
                            .iter()
                            .enumerate()
                            .map(|(index, lane)| {
                                let gate = (lane.lane == final_lane).then_some(gate);
                                lane_column(lane, index + 1, &command_lane, grouping, gate)
                            })
                            .collect_view()
                    })
            }}
        </div>
    }
}

fn lane_column(
    lane: &LaneView,
    number: usize,
    command_lane: &str,
    grouping: RwSignal<WorkerGrouping>,
    gate: Option<Signal<GateGroupView>>,
) -> AnyView {
    let toggle = (lane.grouping == LaneGrouping::Sessions).then(|| {
        view! {
            <button
                type="button"
                class="toggle"
                aria-label=format!("lane-{number}-grouping-toggle")
                on:click=move |_| {
                    grouping
                        .update(|g| {
                            *g = match g {
                                WorkerGrouping::PerWorker => WorkerGrouping::PerFeature,
                                WorkerGrouping::PerFeature => WorkerGrouping::PerWorker,
                            };
                        });
                }
            >
                {move || match grouping.get() {
                    WorkerGrouping::PerWorker => "Per worker (switch to per feature)",
                    WorkerGrouping::PerFeature => "Per feature (switch to per worker)",
                }}
            </button>
        }
    });
    let cards = lane.cards.iter().map(card_view).collect_view();
    let placeholder = lane.placeholder.as_ref().map(|p| {
        let verdict = match p.last_gate_command {
            Some(GateVerdict::Passed) => "passed",
            Some(GateVerdict::Failed) => "failed",
            None => "none yet",
        };
        let clocks = p
            .clocks
            .iter()
            .map(|clock| {
                view! {
                    <li aria-label="gate-clock">
                        {clock.label.clone()} ": " {clock.value.clone().unwrap_or_default()}
                    </li>
                }
            })
            .collect_view();
        view! {
            <div class="gate-placeholder" aria-label="gate-placeholder">
                <p class="gate-state">{p.text.clone()}</p>
                <p aria-label="gate-branch">"mission branch: " {p.branch.clone()}</p>
                <p aria-label="gate-last-verdict">
                    {format!("last gate command ({command_lane}): ")} {verdict}
                </p>
                <ul>{clocks}</ul>
            </div>
        }
    });
    let groups = lane
        .groups
        .iter()
        .map(|group| group_view(lane, group))
        .collect_view();
    view! {
        <section class="lane" aria-label=lane.aria_label.clone() data-lane=lane.lane.0.clone()>
            <h2 class="lane-name">
                {lane.name.clone()} " "
                <span
                    class="count mono"
                    aria-label=format!("lane-{number}-count")
                    data-testid=format!("lane-count-{number}")
                >{lane.event_count}</span>
            </h2>
            {toggle}
            {gate.map(|gate| view! { <GateGroupBox view=gate /> })}
            <div class="cards">{cards}</div>
            {placeholder}
            <div class="rows">{groups}</div>
        </section>
    }
    .into_any()
}

fn group_view(lane: &LaneView, group: &RowGroup) -> AnyView {
    let rows = group.rows.iter().map(row_view).collect_view();
    if !lane.grouped {
        return view! { <ul class="rows-list">{rows}</ul> }.into_any();
    }
    let kind = if lane.grouping == LaneGrouping::Sessions {
        "sublane"
    } else {
        "milestone"
    };
    let role = group.role.map(|role| role.as_str());
    let ctx = expect_context::<BoardCtx>();
    let group_for_title = group.clone();
    view! {
        <section class="sublane" aria-label=format!("{kind}-{}", group.key)>
            <h3 class="mono">
                {move || ctx.sessions.with(|sessions| group_for_title.title(sessions))} " "
                <span class="count">{group.rows.len()}</span>
                {role.map(|role| view! { <span class=format!("role mono role-{role}")>{role}</span> })}
            </h3>
            <ul class="rows-list">{rows}</ul>
        </section>
    }
    .into_any()
}

fn row_view(row: &TimelineRow) -> AnyView {
    let time = row.at.to_rfc3339_millis();
    let overlay = row.overlay.map(|overlay| overlay.as_str());
    let ctx = expect_context::<BoardCtx>();
    let milestone = row.milestone.clone();
    let kind = match milestone {
        Some(milestone) => view! {
            <button
                type="button"
                class="kind row-open mono"
                data-testid="row-kind"
                aria-label=format!("open-details-{}", row.event_id.0)
                on:click=move |_| ctx.open_details.run(milestone.clone())
            >
                {row.kind.clone()}
            </button>
        }
        .into_any(),
        None => view! { <span class="kind mono" data-testid="row-kind">{row.kind.clone()}</span> }
            .into_any(),
    };
    view! {
        <li
            class="row"
            data-testid="timeline-row"
            class:unmapped=row.unmapped
            data-event-id=row.event_id.0.clone()
            data-role=row.role.as_str()
            data-model=row.model.as_ref().map(|m| m.0.clone())
            data-feature=row.feature.as_ref().map(|f| f.0.clone())
        >
            <time class="mono" data-testid="row-time">{time[11..23].to_owned()}</time>
            <span class=format!("role mono role-{}", row.role.as_str())>{row.role.as_str()}</span>
            {kind}
            {row.model.as_ref().map(|model| view! { <span class="model mono">{model.0.clone()}</span> })}
            {overlay
                .map(|overlay| {
                    view! {
                        <span class=format!("overlay mono overlay-{overlay}") aria-label="overlay-marker">
                            {overlay}
                        </span>
                    }
                })}
        </li>
    }
    .into_any()
}

fn card_view(card: &Card) -> AnyView {
    let badge = card.validation;
    let features = card
        .features
        .iter()
        .map(|f| f.0.clone())
        .collect::<Vec<_>>()
        .join(", ");
    let ctx = expect_context::<BoardCtx>();
    let now = match (&card.now, card.idle) {
        (Some(now), _) => {
            let model = now
                .model
                .as_ref()
                .map_or("model n/a", |model| model.0.as_str());
            let feature = now.feature.as_ref().map_or("no feature", |f| f.0.as_str());
            Some(format!(
                "now: {} {} {} [{}] on {}",
                now.role.as_str(),
                model,
                ctx.sessions
                    .with(|sessions| session_name(sessions, &now.session_id)),
                now.session_id.0.get(..8).unwrap_or(&now.session_id.0),
                feature
            ))
        }
        (None, Some(idle)) => Some(format!("now: idle, {}", idle.label())),
        (None, None) => None,
    }
    .map(|text| view! { <p class="now">{text}</p> });
    let milestone = card.milestone.clone();
    let details = card.milestone.clone();
    let worktrees = ctx.worktrees.clone();
    let named = card.milestone.clone();
    let worktree = move || {
        worktrees.with(|all| {
            all.iter()
                .find(|worktree| worktree.milestone == named)
                .map(|worktree| {
                    view! {
                        <p
                            class="mono"
                            aria-label="card-worktree"
                            data-testid="card-worktree"
                            title=worktree.path.clone()
                        >
                            {worktree.name.clone()}
                        </p>
                    }
                })
        })
    };
    let marker = stall_marker(card.features.clone());
    let marks = view! { <FeatureMarks features=card.features.clone() /> };
    let template = expect_context::<RwSignal<CycleTemplate>>();
    let arc = template
        .with(|template| card.return_arc(template))
        .map(|arc| {
            view! {
                <p
                    class=if arc.active { "return-arc active" } else { "return-arc" }
                    aria-label="return-arc"
                    data-testid="return-arc"
                    data-from=arc.from.0.clone()
                    data-to=arc.to.0.clone()
                >
                    <svg viewBox="0 0 24 12" width="24" height="12" aria-hidden="true">
                        <path
                            d="M22 2 C22 11 2 11 2 4 M2 4 L5 2 M2 4 L0 7"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="1.5"
                        />
                    </svg>
                    <span class="mono" data-testid="return-arc-lanes">
                        {format!("{} → {}", arc.from_name, arc.to_name)}
                    </span>
                    <span class="chip mono" data-testid="card-cycle">{arc.label.clone()}</span>
                </p>
            }
        });
    view! {
        <article class="card" aria-label=format!("card-{}", card.milestone)>
            <h3 class="mono">{card.milestone.0.clone()}</h3>
            <span
                class=format!("badge mono badge-{}", badge.as_str())
                aria-label="validation-badge"
            >
                {badge.label()}
            </span>
            <p class="meta" aria-label="card-events">{card.event_count} " events"</p>
            <p class="meta" aria-label="card-features">
                {card.features.len()} " features" {(!features.is_empty()).then(|| format!(": {features}"))}
            </p>
            {marker}
            {worktree}
            {arc}
            {now}
            {marks}
            <button
                type="button"
                aria-label=format!("details-{}", details)
                on:click=move |_| ctx.open_details.run(milestone.clone())
            >
                "Details"
            </button>
        </article>
    }
    .into_any()
}
