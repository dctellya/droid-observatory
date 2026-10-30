//! Cost and waste (FR-D6): tokens and cost per feature and role of one milestone, its waste by
//! the eight causes and its rework windows.

use std::collections::{BTreeMap, BTreeSet};

use super::tabs_end;
use crate::config::OverheadApportionment;
use crate::metrics::{
    Cost, CostInput, CostUnit, Notes, PriceTable, ReworkKind, RowScope, TokenCounts, WasteCause,
    card_times, metrics_report, throttled, turns,
};
use crate::model::{
    CycleTemplate, Event, EventId, EventKind, FeatureId, MilestoneId, Role, Timestamp,
};

const MISSION_LEVEL: &str = "(mission level)";

#[derive(Clone, Debug, PartialEq)]
pub struct UsageRow {
    pub label: String,
    /// The usage log's turns: input, output and cache read.
    pub tokens: TokenCounts,
    /// The sidecar's `usage.reported`; the usage log carries no cache-write count.
    pub cache_write: u64,
    pub cost: Cost,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WasteRow {
    pub cause: WasteCause,
    pub name: &'static str,
    pub seconds: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReworkRow {
    pub feature: Option<FeatureId>,
    pub kind: ReworkKind,
    pub key: Option<String>,
    /// The name of the lane that holds the event that opened the window.
    pub cause: String,
    pub opened_by: EventId,
    pub closed_by: Option<EventId>,
    pub opened: Timestamp,
    /// `None` while no event closes it.
    pub closed: Option<Timestamp>,
    pub unresolved: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CostTab {
    pub milestone: MilestoneId,
    /// The engineering cycle time of the milestone (the `cycle_s` of its CSV row).
    pub cycle_s: u64,
    /// The rework windows of the milestone (the `rework_windows` of its CSV row).
    pub rework_windows: usize,
    /// The date every cost on the tab names.
    pub price_table: String,
    pub currency: String,
    pub totals: UsageRow,
    pub features: Vec<UsageRow>,
    pub roles: Vec<UsageRow>,
    /// Tokens of models the table has no row for: counted, not priced (rule 18).
    pub unpriced: TokenCounts,
    /// The eight causes, in the order of the CSV columns, as the milestone's CSV row holds them.
    pub waste: Vec<WasteRow>,
    pub rework: Vec<ReworkRow>,
}

#[derive(Default)]
struct Tally {
    tokens: TokenCounts,
    cache_write: u64,
    usd: f64,
}

impl Tally {
    fn row(&self, label: String, prices: &PriceTable) -> UsageRow {
        UsageRow {
            label,
            tokens: self.tokens,
            cache_write: self.cache_write,
            cost: prices.cost(self.usd, CostUnit::Usd),
        }
    }
}

pub fn cost_tab(
    events: &[Event],
    milestone: &MilestoneId,
    template: &CycleTemplate,
    prices: &PriceTable,
    notes: &Notes,
) -> CostTab {
    let own = |event: &Event| event.milestone_id.as_ref() == Some(milestone);
    let features: BTreeSet<&FeatureId> = events
        .iter()
        .filter(|event| own(event))
        .filter_map(|event| event.feature_id.as_ref())
        .collect();

    let mut total = Tally::default();
    let mut by_feature: BTreeMap<Option<&FeatureId>, Tally> = features
        .iter()
        .map(|f| (Some(*f), Tally::default()))
        .collect();
    let mut by_role: BTreeMap<usize, Tally> = BTreeMap::new();
    let mut unpriced = TokenCounts::default();
    let rank = |role: Role| {
        Role::ALL
            .iter()
            .position(|r| *r == role)
            .unwrap_or(usize::MAX)
    };
    for turn in turns(events).filter(|turn| own(turn.event)) {
        let usd = prices.turn_usd(
            turn.event.role,
            turn.model.map(|m| m.0.as_str()),
            &turn.tokens,
        );
        if usd.is_none() {
            unpriced += turn.tokens;
        }
        let usd = usd.unwrap_or(0.0);
        for tally in [
            &mut total,
            by_feature
                .entry(turn.event.feature_id.as_ref())
                .or_default(),
            by_role.entry(rank(turn.event.role)).or_default(),
        ] {
            tally.tokens += turn.tokens;
            tally.usd += usd;
        }
    }
    for event in events
        .iter()
        .filter(|e| own(e) && e.kind == EventKind::UsageReported)
    {
        let Some(usage) = event.usage else { continue };
        let usd = prices
            .cache_write_usd(
                event.role,
                event.model.as_ref().map(|m| m.0.as_str()),
                usage.cache_write,
            )
            .unwrap_or(0.0);
        for tally in [
            &mut total,
            by_feature.entry(event.feature_id.as_ref()).or_default(),
            by_role.entry(rank(event.role)).or_default(),
        ] {
            tally.cache_write += usage.cache_write;
            tally.usd += usd;
        }
    }

    let end = tabs_end(events);
    let rework = end.map_or_else(Vec::new, |end| {
        rework_rows(events, milestone, &features, template, notes, end)
    });
    // The milestone's CSV row, made as `observatory-cli metrics` makes it, so the tab's cycle,
    // rework count and waste are the figures of the CSV and not a second reading of the events.
    let row = end.and_then(|end| {
        metrics_report(&CostInput {
            events,
            template,
            notes,
            prices,
            end,
            budget: 0.0,
            launch_budget_usd: None,
            apportionment: OverheadApportionment::Separate,
        })
        .rows
        .into_iter()
        .find(|row| row.scope == RowScope::Milestone && row.id == milestone.0)
    });
    let waste = WasteCause::ALL
        .into_iter()
        .map(|cause| {
            waste_row(
                cause,
                row.as_ref()
                    .and_then(|row| row.waste_s.get(&cause).copied())
                    .unwrap_or(0),
            )
        })
        .collect();

    CostTab {
        milestone: milestone.clone(),
        cycle_s: row.as_ref().map_or(0, |row| row.cycle_s),
        rework_windows: row.as_ref().map_or(0, |row| row.rework_windows),
        price_table: prices.effective().to_owned(),
        currency: prices.currency().to_owned(),
        totals: total.row(milestone.0.clone(), prices),
        features: by_feature
            .iter()
            .filter(|(feature, tally)| {
                feature.is_some() || !tally.tokens.is_zero() || tally.cache_write > 0
            })
            .map(|(feature, tally)| {
                let label = feature.map_or_else(|| MISSION_LEVEL.to_owned(), |f| f.0.clone());
                tally.row(label, prices)
            })
            .collect(),
        roles: by_role
            .iter()
            .filter_map(|(rank, tally)| {
                Some(tally.row(Role::ALL.get(*rank)?.as_str().to_owned(), prices))
            })
            .collect(),
        unpriced,
        waste,
        rework,
    }
}

fn waste_row(cause: WasteCause, seconds: u64) -> WasteRow {
    WasteRow {
        cause,
        name: cause.name(),
        seconds,
    }
}

fn rework_rows(
    events: &[Event],
    milestone: &MilestoneId,
    features: &BTreeSet<&FeatureId>,
    template: &CycleTemplate,
    notes: &Notes,
    end: Timestamp,
) -> Vec<ReworkRow> {
    let stops = throttled(events, notes, end).spans();
    let cards = card_times(events, template, notes, end, &stops);
    let mut rework: Vec<ReworkRow> = cards
        .iter()
        .flat_map(|(feature, card)| card.windows.iter().map(move |w| (feature, w)))
        .filter(|(feature, window)| match feature {
            Some(feature) => features.contains(feature),
            None => window.milestone.as_ref() == Some(milestone),
        })
        .map(|(_, window)| ReworkRow {
            feature: window.feature.clone(),
            kind: window.kind,
            key: window.key.clone(),
            cause: template.name_of(&window.cause),
            opened_by: window.opened_by.clone(),
            closed_by: window.closed_by.clone(),
            opened: window.from,
            closed: window.closed_by.as_ref().map(|_| window.to),
            unresolved: window.closed_by.is_none(),
        })
        .collect();
    rework.sort_by_key(|row| row.opened);
    rework
}
