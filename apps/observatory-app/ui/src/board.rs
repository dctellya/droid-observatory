use std::sync::Arc;

use leptos::prelude::*;
use observatory_core::config::MissionConfig;
use observatory_core::ingest::MemoryReplay;
use observatory_core::model::{EventId, Timestamp};
use observatory_core::view::{
    BoardView, DeclaredModels, Filter, FilterField, FilterOptions, Scrubber, ViewConfig,
    WorkerGrouping, branch_of,
};

use crate::lanes::Lanes;
use crate::panel::{OverlayList, Panel};
use crate::tray::Tray;

/// A capture in Replay mode: header, the six filters, the scrubber, the Panel, the seven phase
/// lanes, the human-overlay list and the unmapped tray. Every figure comes from
/// `BoardView::build`; the components only draw it.
#[component]
pub fn Board(replay: Arc<MemoryReplay>, on_close: Callback<()>) -> impl IntoView {
    let mission = MissionConfig::default();
    let declared = DeclaredModels::from_config(&mission);
    let branch = branch_of(&mission, &replay.summary.name);
    let options = Arc::new(FilterOptions::of(&replay.events));
    let scrubber = Arc::new(Scrubber::new(&replay.events));

    let filter = RwSignal::new(Filter::default());
    let grouping = RwSignal::new(WorkerGrouping::PerWorker);
    let at = RwSignal::new(None::<Timestamp>);
    let selected = RwSignal::new(None::<EventId>);

    let board = Memo::new({
        let replay = replay.clone();
        move |_| {
            let cfg = ViewConfig {
                filter: filter.get(),
                grouping: grouping.get(),
                at: at.get(),
            };
            BoardView::build(&replay.events, &cfg, &declared, &branch)
        }
    });

    let summary = replay.summary.clone();
    view! {
        <div class="board" aria-label="board">
            <header class="top">
                <h1 aria-label="mission-name">{summary.name.clone()}</h1>
                <span class="mode" aria-label="mode">"Replay"</span>
                <dl class="facts">
                    <dt>"Events"</dt>
                    <dd aria-label="event-count">{summary.event_count}</dd>
                    <dt>"Shown"</dt>
                    <dd aria-label="events-shown">{move || board.with(|b| b.event_count)}</dd>
                    <dt>"Unknown"</dt>
                    <dd aria-label="unknown-count">{summary.unknown_count}</dd>
                    <dt>"Sessions"</dt>
                    <dd aria-label="session-count">{summary.session_count}</dd>
                    <dt>"Mapping"</dt>
                    <dd aria-label="mapping-version">{summary.mapping_version.clone()}</dd>
                </dl>
                <button aria-label="back-to-library" on:click=move |_| on_close.run(())>
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
            <FilterBar filter=filter options=options.clone() />
            <ScrubberBar scrubber=scrubber.clone() at=at />
            <Panel board=board />
            <Lanes board=board grouping=grouping />
            <div class="bottom">
                <OverlayList board=board />
                <Tray board=board replay=replay.clone() selected=selected />
            </div>
        </div>
    }
}

/// FR-B5: the six filters as labelled controls.
#[component]
fn FilterBar(filter: RwSignal<Filter>, options: Arc<FilterOptions>) -> impl IntoView {
    let controls = FilterField::ALL
        .into_iter()
        .map(|field| {
            let choices = options.of_field(field).to_vec();
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
                        {choices
                            .into_iter()
                            .map(|choice| {
                                view! { <option value=choice.value>{choice.label}</option> }
                            })
                            .collect_view()}
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

/// FR-B6: the replay slider, the timestamp it stands at, and a field to type one.
#[component]
fn ScrubberBar(scrubber: Arc<Scrubber>, at: RwSignal<Option<Timestamp>>) -> impl IntoView {
    let last = scrubber.last_index();
    let slider = scrubber.clone();
    let label = scrubber.clone();
    let value = scrubber.clone();
    view! {
        <div class="scrubber" role="group" aria-label="scrubber-bar">
            <label>
                "Replay position "
                <input
                    type="range"
                    min="0"
                    max=last.to_string()
                    step="1"
                    aria-label="scrubber"
                    prop:value=move || value.index_of(at.get()).to_string()
                    on:input=move |ev| {
                        let index = event_target_value(&ev).parse().unwrap_or(last);
                        at.set(slider.at(index));
                    }
                />
            </label>
            <output aria-label="scrubber-timestamp">{move || label.label(at.get())}</output>
            <label>
                "Go to time "
                <input
                    type="text"
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
                {move || if at.get().is_some() { "scrubbed" } else { "end of replay" }}
            </span>
        </div>
    }
}
