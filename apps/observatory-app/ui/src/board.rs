use std::sync::Arc;

use leptos::prelude::*;
use observatory_core::config::MissionConfig;
use observatory_core::model::{CycleTemplate, EventId, MilestoneId, Timestamp};
use observatory_core::view::{
    BoardView, ControlBar, DeclaredModels, Filter, FilterField, FilterOptions, Scrubber,
    ViewConfig, WorkerGrouping, branch_of, tray_count,
};

use crate::agents::AgentPanel;
use crate::attach::AttachPanel;
use crate::capture::Capture;
use crate::controls::{ControlCtx, Controls, StallBannerView, StallCtx, ThemeSwitch};
use crate::ctx::BoardCtx;
use crate::details::DetailsView;
use crate::lanes::Lanes;
use crate::launch::HostedCtx;
use crate::pagehead::PageHead;
use crate::panel::{OverlayList, Panel};
use crate::tray::Tray;

/// A capture on the board: header, the six filters, the scrubber, the Panel, the template's
/// lanes, the human-overlay list and the unmapped tray. Every figure comes from
/// `BoardView::build` over the events held, which grow while the capture is live.
#[component]
pub fn Board(capture: Capture, on_close: Callback<()>) -> impl IntoView {
    let template = expect_context::<RwSignal<CycleTemplate>>();
    let mission = MissionConfig::default();
    let declared = DeclaredModels::from_config(&mission);
    let agent_declared = declared.clone();
    let branch = branch_of(&mission, &capture.summary.name);
    let events = capture.events.clone();
    let options = Memo::new({
        let events = events.clone();
        move |_| events.with(|events| FilterOptions::of(events))
    });
    let scrubber = Memo::new_with_compare(
        {
            let events = events.clone();
            move |_| Arc::new(events.with(|events| Scrubber::new(events)))
        },
        |_, _| false,
    );

    let filter = RwSignal::new(Filter::default());
    let grouping = RwSignal::new(WorkerGrouping::PerWorker);
    let at = RwSignal::new(None::<Timestamp>);
    let selected = RwSignal::new(None::<EventId>);
    let details = RwSignal::new(None::<MilestoneId>);
    let board = Memo::new({
        let events = events.clone();
        move |_| {
            let cfg = ViewConfig {
                filter: filter.get(),
                grouping: grouping.get(),
                at: at.get(),
            };
            template.with(|template| {
                events.with(|events| BoardView::build(events, &cfg, &declared, &branch, template))
            })
        }
    });
    let polled = capture.gate.clone();
    let gate = Signal::derive(move || {
        let group = board.with(|board| board.gate.clone());
        let group = polled.with(|polled| match polled {
            Some(polled) => group.with_polled(polled),
            None => group,
        });
        template.with(|template| group.view(template))
    });
    provide_context(BoardCtx {
        open_details: Callback::new(move |milestone| details.set(Some(milestone))),
        sessions: capture.sessions.clone(),
        gate,
        worktrees: capture.worktrees.clone(),
    });

    provide_context(crate::gate::MergeState::new());

    let tauri = crate::shell::is_tauri();
    let hosted = expect_context::<HostedCtx>();
    let mission = capture.mission_id().clone();
    let launch_id = hosted
        .launched
        .get_untracked()
        .filter(|launched| launched.session_id == mission.0)
        .map(|launched| launched.launch_id);
    let stalls = expect_context::<StallCtx>();
    let stalled = Signal::derive({
        let mission = mission.0.clone();
        move || {
            stalls
                .0
                .with(|all| all.get(&mission).cloned().unwrap_or_default())
        }
    });
    let live_now = capture.live.clone();
    let hosted_launch = launch_id.is_some();
    let enabled = Signal::derive(move || tauri && (hosted_launch || live_now.get()));
    let control = ControlCtx {
        mission,
        launch_id: launch_id.clone(),
        hosted,
        result: RwSignal::new(None),
        paste: RwSignal::new(None),
        stalled,
        enabled,
    };
    provide_context(control.clone());
    let bar = Signal::derive({
        let events = events.clone();
        move || match (&launch_id, hosted.status.get()) {
            (Some(_), Some(status)) => {
                events.with(|events| ControlBar::hosted(&status, Some(events)))
            }
            _ => events.with(|events| ControlBar::attached(events)),
        }
    });

    let summary = capture.summary.clone();
    let live = capture.live.clone();
    let mode_live = live.clone();
    let state_live = live.clone();
    let event_total = events.clone();
    let unknown_total = events.clone();
    let session_total = capture.sessions.clone();
    let eyebrow = format!(
        "Observatory › mission {}",
        summary
            .mission_id
            .0
            .get(..8)
            .unwrap_or(&summary.mission_id.0)
    );
    let subtitle = Signal::derive(move || {
        template.with(|template| format!("cycle template {}", template.name))
    });
    view! {
        <div class="board" aria-label="board" data-testid="board">
            <PageHead eyebrow=eyebrow subtitle=subtitle>
                <h1 aria-label="mission-name" data-testid="mission-name">{summary.name.clone()}</h1>
            </PageHead>
            <header class="top">
                <span class="mode mono" aria-label="mode">
                    {move || if mode_live.get() { "Live" } else { "Replay" }}
                </span>
                <dl class="facts">
                    <dt>"Events"</dt>
                    <dd class="mono" aria-label="event-count" data-testid="event-count">{move || event_total.with(Vec::len)}</dd>
                    <dt>"Shown"</dt>
                    <dd class="mono" aria-label="events-shown">{move || board.with(|b| b.event_count)}</dd>
                    <dt>"Unknown"</dt>
                    <dd class="mono" aria-label="unknown-count" data-testid="unknown-count">
                        {move || unknown_total.with(|events| tray_count(events))}
                    </dd>
                    <dt>"Sessions"</dt>
                    <dd class="mono" aria-label="session-count">{move || session_total.with(Vec::len)}</dd>
                    <dt>"Mapping"</dt>
                    <dd class="mono" aria-label="mapping-version">{summary.mapping_version.clone()}</dd>
                </dl>
                <ThemeSwitch />
                <button aria-label="back-to-library" data-testid="back-to-library" on:click=move |_| on_close.run(())>
                    "Library"
                </button>
            </header>
            {summary
                .warning
                .clone()
                .map(|warning| {
                    view! {
                        <p class="warning" role="status" aria-label="mapping-warning">{warning}</p>
                    }
                })}
            <AttachPanel />
            <StallBannerView flags=stalled />
            {move || enabled.get().then(|| view! { <Controls ctx=control.clone() bar=bar /> })}
            <FilterBar filter=filter options=options />
            <ScrubberBar scrubber=scrubber at=at live=state_live />
            <Panel board=board />
            <AgentPanel board=board capture=capture.clone() declared=agent_declared at=at />
            <DetailsView capture=capture.clone() milestone=details />
            <Lanes board=board grouping=grouping gate=gate />
            <div class="bottom">
                <OverlayList board=board />
                <Tray board=board capture=capture.clone() selected=selected />
            </div>
        </div>
    }
}

/// FR-B5: the six filters as labelled controls.
#[component]
fn FilterBar(filter: RwSignal<Filter>, options: Memo<FilterOptions>) -> impl IntoView {
    let controls = FilterField::ALL
        .into_iter()
        .map(|field| {
            view! {
                <label class="filter">
                    {field.label()}
                    <select
                        aria-label=field.aria_label()
                        prop:value=move || filter.with(|f| f.value(field))
                        on:change=move |ev| {
                            let value = event_target_value(&ev);
                            filter.update(|f| f.set(field, &value));
                        }
                    >
                        <option value="">"All"</option>
                        {move || {
                            options
                                .with(|o| o.of_field(field).to_vec())
                                .into_iter()
                                .map(|choice| {
                                    view! { <option value=choice.value>{choice.label}</option> }
                                })
                                .collect_view()
                        }}
                    </select>
                </label>
            }
        })
        .collect_view();
    view! {
        <form class="filters" aria-label="filters" on:submit=|ev| ev.prevent_default()>
            {controls}
            <button type="button" aria-label="filters-clear" on:click=move |_| filter.set(Filter::default())>
                "Clear filters"
            </button>
        </form>
    }
}

/// FR-B6: the replay slider, the timestamp it stands at, and a field to type one. The slider
/// follows the events held, so it grows while the capture is live.
#[component]
fn ScrubberBar(
    scrubber: Memo<Arc<Scrubber>>,
    at: RwSignal<Option<Timestamp>>,
    live: ArcRwSignal<bool>,
) -> impl IntoView {
    view! {
        <div class="scrubber" role="group" aria-label="scrubber-bar">
            <label>
                "Replay position "
                <input
                    type="range"
                    min="0"
                    max=move || scrubber.with(|s| s.last_index()).to_string()
                    step="1"
                    aria-label="scrubber"
                    prop:value=move || scrubber.with(|s| s.index_of(at.get())).to_string()
                    on:input=move |ev| {
                        let index = event_target_value(&ev)
                            .parse()
                            .unwrap_or_else(|_| scrubber.with(|s| s.last_index()));
                        at.set(scrubber.with(|s| s.at(index)));
                    }
                />
            </label>
            <output class="mono" aria-label="scrubber-timestamp">
                {move || scrubber.with(|s| s.label(at.get()))}
            </output>
            <label>
                "Go to time "
                <input
                    type="text"
                    class="mono"
                    aria-label="scrubber-time"
                    placeholder="2026-10-22T10:18:11.451Z"
                    on:change=move |ev| {
                        let text = event_target_value(&ev);
                        let text = text.trim();
                        if text.is_empty() {
                            at.set(None);
                        } else if let Ok(parsed) = Timestamp::parse(text) {
                            at.set(Some(parsed));
                        }
                    }
                />
            </label>
            <button type="button" aria-label="scrubber-end" on:click=move |_| at.set(None)>
                "End"
            </button>
            <span class="live" aria-label="scrubber-state">
                {move || {
                    if at.get().is_some() {
                        "scrubbed"
                    } else if live.get() {
                        "live"
                    } else {
                        "end of replay"
                    }
                }}
            </span>
        </div>
    }
}
