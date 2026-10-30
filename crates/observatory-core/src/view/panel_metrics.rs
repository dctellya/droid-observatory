//! The Panel's charts and usage block (FR-M5, FR-M6): waste by the five display groups, a
//! timeline of time states and rework per feature, the cumulative cost, and the cost half of the
//! metrics report beside them. Geometry is fractions of the chart (0 to 1); the UI scales it.
//! `throttled` is none of the five groups: it is its own field of the block.

use std::collections::{BTreeMap, BTreeSet};

use super::library::Headline;
use super::tabs::tabs_end;
use crate::config::OverheadApportionment;
use crate::metrics::{
    CardTime, CostInput, CostReport, Notes, PriceTable, Throttled, TimeState, TokenCounts,
    WasteCause, WasteGroup, card_times, cost_report, throttled, turns,
};
use crate::model::{CycleTemplate, Event, FeatureId, MilestoneId, Timestamp};

/// The label of the bar that stacks the whole run.
pub const RUN_LABEL: &str = "run";

/// The group's name on the legend and the bars.
fn group_label(group: WasteGroup) -> &'static str {
    match group {
        WasteGroup::WaitingOnHuman => "Waiting on human",
        WasteGroup::WaitingOnValidation => "Waiting on validation",
        WasteGroup::Rework => "Rework",
        WasteGroup::Retry => "Retry",
        WasteGroup::Idle => "Idle",
    }
}

/// The CSS class of the group's colour.
fn group_class(group: WasteGroup) -> &'static str {
    match group {
        WasteGroup::WaitingOnHuman => "waiting-on-human",
        WasteGroup::WaitingOnValidation => "waiting-on-validation",
        WasteGroup::Rework => "rework",
        WasteGroup::Retry => "retry",
        WasteGroup::Idle => "idle",
    }
}

/// One group of the run: its seconds, and the tokens of the turns inside its bands.
#[derive(Clone, Debug, PartialEq)]
pub struct GroupTotal {
    pub group: WasteGroup,
    pub label: &'static str,
    pub class: &'static str,
    pub seconds: u64,
    pub tokens: TokenCounts,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WasteSegment {
    pub group: WasteGroup,
    pub label: &'static str,
    pub class: &'static str,
    pub seconds: u64,
    /// Distance of the segment's bottom from the axis, as a fraction of the tallest bar.
    pub y: f64,
    pub height: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WasteBar {
    /// `run`, or a milestone id.
    pub label: String,
    pub total_s: u64,
    /// The non-zero groups, bottom first.
    pub segments: Vec<WasteSegment>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WasteChart {
    /// The groups the run has time in, in the order of the five.
    pub legend: Vec<&'static str>,
    /// The five groups of the run, zero ones too, with their tokens.
    pub groups: Vec<GroupTotal>,
    /// The run first, then one bar per milestone.
    pub bars: Vec<WasteBar>,
    pub max_s: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StateBand {
    /// The time state: `active` or the waste cause's name.
    pub state: &'static str,
    /// `active`, or the class of the cause's group.
    pub class: &'static str,
    pub from: Timestamp,
    pub to: Timestamp,
    pub x: f64,
    pub width: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReworkBand {
    pub from: Timestamp,
    pub to: Timestamp,
    pub unresolved: bool,
    pub x: f64,
    pub width: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FeatureTimeline {
    pub feature: FeatureId,
    pub milestone: Option<MilestoneId>,
    pub bands: Vec<StateBand>,
    pub rework: Vec<ReworkBand>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TimelineChart {
    /// One row per feature, earliest start first.
    pub rows: Vec<FeatureTimeline>,
    pub from: Timestamp,
    pub to: Timestamp,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SparkPoint {
    pub at: Timestamp,
    /// Cumulative cost so far.
    pub cost: f64,
    pub x: f64,
    /// Cumulative cost as a fraction of the total.
    pub y: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Sparkline {
    pub points: Vec<SparkPoint>,
    pub total: f64,
    pub currency: String,
    pub price_table: String,
}

impl Sparkline {
    /// The path of the line in a `width` by `height` box, the total at the top.
    pub fn path(&self, width: f64, height: f64) -> String {
        let mut d = String::new();
        for (index, point) in self.points.iter().enumerate() {
            d.push_str(if index == 0 { "M" } else { " L" });
            d.push_str(&format!(
                "{:.2} {:.2}",
                point.x * width,
                (1.0 - point.y) * height
            ));
        }
        d
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PanelMetrics {
    /// Tokens and cost per role, lane and milestone, tokens in waste, the budget meter and the
    /// model comparison: the figures of `observatory-cli metrics --json`.
    pub cost: CostReport,
    /// `throttled` apart from the groups: the run's minutes, each milestone's, the intervals.
    pub throttled: Throttled,
    pub waste: WasteChart,
    pub timeline: TimelineChart,
    pub sparkline: Sparkline,
    /// First event to last, in whole seconds.
    pub cycle_s: u64,
}

impl PanelMetrics {
    /// The library's line for the mission.
    pub fn headline(&self) -> Headline {
        let run = self.waste.bars.first();
        Headline {
            cycle_s: self.cycle_s,
            waste_s: run.map_or(0, |bar| bar.total_s),
            tokens_total: self.cost.total_tokens.total(),
            cost: self.cost.cost_so_far.amount,
            currency: self.cost.currency.clone(),
            price_table: self.cost.price_table.clone(),
        }
    }
}

/// The Panel's figures over the events held, made at the last event. `None` when no event
/// besides `turn.usage` is held. `budget` is `[metrics].budget`.
pub fn panel_metrics(
    events: &[Event],
    template: &CycleTemplate,
    prices: &PriceTable,
    notes: &Notes,
    budget: f64,
) -> Option<PanelMetrics> {
    let end = tabs_end(events)?;
    let start = events
        .iter()
        .filter(|event| !event.kind.is_panel_only())
        .map(|event| event.at)
        .min()?;
    let cost = cost_report(&CostInput {
        events,
        template,
        notes,
        prices,
        end,
        budget,
        launch_budget_usd: None,
        apportionment: OverheadApportionment::Separate,
    });
    let stops = throttled(events, notes, end);
    let cards = card_times(events, template, notes, end, &stops.spans());
    Some(PanelMetrics {
        waste: waste_chart(events, &cards, &cost),
        timeline: timeline_chart(events, &cards, end),
        sparkline: sparkline(events, prices, &cost),
        cycle_s: u64::try_from((end.0 - start.0).whole_seconds()).unwrap_or(0),
        throttled: stops,
        cost,
    })
}

fn waste_chart(
    events: &[Event],
    cards: &BTreeMap<Option<FeatureId>, CardTime>,
    cost: &CostReport,
) -> WasteChart {
    let mut milestones: BTreeMap<MilestoneId, BTreeSet<FeatureId>> = BTreeMap::new();
    for event in events {
        if let (Some(milestone), Some(feature)) = (&event.milestone_id, &event.feature_id) {
            milestones
                .entry(milestone.clone())
                .or_default()
                .insert(feature.clone());
        }
    }
    let seconds_of = |group: WasteGroup, own: &[&CardTime]| -> u64 {
        own.iter().map(|card| card.waste.group_seconds(group)).sum()
    };
    let all: Vec<&CardTime> = cards.values().collect();
    let mut stacks: Vec<(String, Vec<(WasteGroup, u64)>)> = vec![(
        RUN_LABEL.to_owned(),
        WasteGroup::ALL
            .into_iter()
            .map(|group| (group, seconds_of(group, &all)))
            .collect(),
    )];
    for (milestone, features) in &milestones {
        let own: Vec<&CardTime> = features
            .iter()
            .filter_map(|feature| cards.get(&Some(feature.clone())))
            .collect();
        stacks.push((
            milestone.0.clone(),
            WasteGroup::ALL
                .into_iter()
                .map(|group| (group, seconds_of(group, &own)))
                .collect(),
        ));
    }
    let max_s = stacks
        .iter()
        .map(|(_, stack)| stack.iter().map(|(_, seconds)| seconds).sum::<u64>())
        .max()
        .unwrap_or(0);
    let scale = |seconds: u64| {
        if max_s == 0 {
            0.0
        } else {
            seconds as f64 / max_s as f64
        }
    };
    let bars: Vec<WasteBar> = stacks
        .into_iter()
        .map(|(label, stack)| {
            let mut below = 0;
            let segments: Vec<WasteSegment> = stack
                .into_iter()
                .filter(|(_, seconds)| *seconds > 0)
                .map(|(group, seconds)| {
                    let segment = WasteSegment {
                        group,
                        label: group_label(group),
                        class: group_class(group),
                        seconds,
                        y: scale(below),
                        height: scale(seconds),
                    };
                    below += seconds;
                    segment
                })
                .collect();
            WasteBar {
                label,
                total_s: below,
                segments,
            }
        })
        .collect();
    let groups: Vec<GroupTotal> = WasteGroup::ALL
        .into_iter()
        .map(|group| GroupTotal {
            group,
            label: group_label(group),
            class: group_class(group),
            seconds: seconds_of(group, &all),
            tokens: WasteCause::ALL
                .into_iter()
                .filter(|cause| cause.group() == group)
                .filter_map(|cause| cost.tokens_in_waste_by_cause.by_cause.get(&cause))
                .fold(TokenCounts::default(), |mut sum, tokens| {
                    sum += *tokens;
                    sum
                }),
        })
        .collect();
    WasteChart {
        legend: groups
            .iter()
            .filter(|group| group.seconds > 0)
            .map(|group| group.label)
            .collect(),
        groups,
        bars,
        max_s,
    }
}

fn fraction(at: Timestamp, from: Timestamp, span_s: f64) -> f64 {
    if span_s <= 0.0 {
        return 0.0;
    }
    ((at.0 - from.0).as_seconds_f64() / span_s).clamp(0.0, 1.0)
}

fn timeline_chart(
    events: &[Event],
    cards: &BTreeMap<Option<FeatureId>, CardTime>,
    end: Timestamp,
) -> TimelineChart {
    let mut milestone_of: BTreeMap<&FeatureId, &MilestoneId> = BTreeMap::new();
    for event in events {
        if let (Some(feature), Some(milestone)) = (&event.feature_id, &event.milestone_id) {
            milestone_of.entry(feature).or_insert(milestone);
        }
    }
    let intervals = || {
        cards
            .iter()
            .filter(|(key, _)| key.is_some())
            .flat_map(|(_, card)| card.intervals.iter())
    };
    let Some(from) = intervals().map(|interval| interval.from).min() else {
        return TimelineChart {
            rows: Vec::new(),
            from: end,
            to: end,
        };
    };
    let to = intervals()
        .map(|interval| interval.to)
        .max()
        .unwrap_or(from);
    let span_s = (to.0 - from.0).as_seconds_f64();
    let mut rows: Vec<FeatureTimeline> = cards
        .iter()
        .filter_map(|(key, card)| Some((key.as_ref()?, card)))
        .map(|(feature, card)| FeatureTimeline {
            feature: feature.clone(),
            milestone: milestone_of.get(feature).map(|m| (*m).clone()),
            bands: card
                .intervals
                .iter()
                .map(|interval| {
                    let (x, end) = (
                        fraction(interval.from, from, span_s),
                        fraction(interval.to, from, span_s),
                    );
                    StateBand {
                        state: interval
                            .state
                            .waste_cause()
                            .map_or("active", WasteCause::name),
                        class: match interval.state {
                            TimeState::Active => "active",
                            state => state
                                .waste_cause()
                                .map_or("active", |cause| group_class(cause.group())),
                        },
                        from: interval.from,
                        to: interval.to,
                        x,
                        width: end - x,
                    }
                })
                .collect(),
            rework: card
                .rework
                .iter()
                .map(|span| {
                    let (x, end) = (
                        fraction(span.from, from, span_s),
                        fraction(span.to, from, span_s),
                    );
                    ReworkBand {
                        from: span.from,
                        to: span.to,
                        unresolved: span.unresolved,
                        x,
                        width: end - x,
                    }
                })
                .collect(),
        })
        .collect();
    rows.sort_by_key(|row| row.bands.first().map(|band| band.from));
    TimelineChart { rows, from, to }
}

fn sparkline(events: &[Event], prices: &PriceTable, cost: &CostReport) -> Sparkline {
    let mut priced: Vec<(Timestamp, f64)> = turns(events)
        .map(|turn| {
            let usd = prices
                .turn_usd(
                    turn.event.role,
                    turn.model.map(|model| model.0.as_str()),
                    &turn.tokens,
                )
                .unwrap_or(0.0);
            (turn.log_ts, usd)
        })
        .collect();
    priced.sort_by_key(|(at, _)| *at);
    let total: f64 = priced.iter().map(|(_, usd)| usd).sum();
    let mut points = Vec::new();
    if let (Some((first, _)), Some((last, _))) = (priced.first().copied(), priced.last().copied()) {
        let span_s = (last.0 - first.0).as_seconds_f64();
        let y_of = |cost: f64| if total > 0.0 { cost / total } else { 0.0 };
        points.push(SparkPoint {
            at: first,
            cost: 0.0,
            x: 0.0,
            y: 0.0,
        });
        let mut running = 0.0;
        for (at, usd) in priced {
            running += usd;
            points.push(SparkPoint {
                at,
                cost: running,
                x: fraction(at, first, span_s),
                y: y_of(running),
            });
        }
    }
    Sparkline {
        points,
        total,
        currency: cost.currency.clone(),
        price_table: cost.price_table.clone(),
    }
}
