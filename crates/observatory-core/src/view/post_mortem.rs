use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::model::{CycleTemplate, Event, FeatureId, LaneId, Role, Timestamp};

/// What a post-mortem of one run starts from: one summary per template lane and one reading per
/// template clock, in template order (F-041). The input is data; the prose is the author's.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PostMortemInput {
    /// The template's name.
    pub cycle: String,
    pub lanes: Vec<LaneSummary>,
    pub clocks: Vec<ClockReading>,
    /// When the template's final event last happened; none while the cycle has not ended.
    pub final_event_at: Option<Timestamp>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaneSummary {
    pub lane: LaneId,
    pub name: String,
    pub owner: Option<Role>,
    /// Events of the lane; `turn.usage` is Panel data and not counted, as on the board.
    pub event_count: u64,
    pub first_at: Option<Timestamp>,
    pub last_at: Option<Timestamp>,
}

/// A clock read over the whole run. A clock with no `start` and no `end` is label only and
/// reads empty.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClockReading {
    pub id: String,
    pub label: String,
    /// The earliest start over the features that have one.
    pub start_at: Option<Timestamp>,
    /// The latest `end` event.
    pub end_at: Option<Timestamp>,
    /// Features that started on the clock's `start_fallback` kind because they never had its
    /// `start` kind.
    pub fallback_used: u64,
}

fn named(name: &Option<String>) -> impl Fn(&&Event) -> bool + '_ {
    move |event| name.as_deref() == Some(&*event.kind.prd_name())
}

/// The post-mortem input of `events` under `template`. Iterates `template.lanes` and
/// `template.clocks`; it names no lane.
pub fn post_mortem_input(events: &[Event], template: &CycleTemplate) -> PostMortemInput {
    let mut lanes: Vec<LaneSummary> = template
        .lanes
        .iter()
        .map(|lane| LaneSummary {
            lane: lane.id.clone(),
            name: lane.name.clone(),
            owner: lane.owner_role,
            event_count: 0,
            first_at: None,
            last_at: None,
        })
        .collect();
    let shown = || events.iter().filter(|event| !event.kind.is_panel_only());
    for event in shown() {
        if let Some(index) = template.index(&event.lane) {
            let summary = &mut lanes[index];
            summary.event_count += 1;
            summary.first_at = Some(summary.first_at.map_or(event.at, |at| at.min(event.at)));
            summary.last_at = Some(summary.last_at.map_or(event.at, |at| at.max(event.at)));
        }
    }

    let clocks = template
        .clocks
        .iter()
        .map(|clock| {
            let mut starts: BTreeMap<Option<&FeatureId>, Timestamp> = BTreeMap::new();
            let mut fallbacks: BTreeMap<&FeatureId, Timestamp> = BTreeMap::new();
            for event in shown().filter(named(&clock.start)) {
                let at = starts.entry(event.feature_id.as_ref()).or_insert(event.at);
                *at = (*at).min(event.at);
            }
            for event in shown().filter(named(&clock.start_fallback)) {
                if let Some(feature) = &event.feature_id {
                    let at = fallbacks.entry(feature).or_insert(event.at);
                    *at = (*at).min(event.at);
                }
            }
            let used: Vec<Timestamp> = fallbacks
                .iter()
                .filter(|(feature, _)| !starts.contains_key(&Some(**feature)))
                .map(|(_, at)| *at)
                .collect();
            ClockReading {
                id: clock.id.clone(),
                label: clock.label.clone(),
                start_at: starts.values().chain(&used).min().copied(),
                end_at: shown().filter(named(&clock.end)).map(|e| e.at).max(),
                fallback_used: used.len() as u64,
            }
        })
        .collect();

    PostMortemInput {
        cycle: template.name.clone(),
        lanes,
        clocks,
        final_event_at: shown()
            .filter(named(&Some(template.final_event.clone())))
            .map(|event| event.at)
            .max(),
    }
}
