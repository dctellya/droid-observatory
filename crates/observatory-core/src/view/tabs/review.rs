//! Review notes (FR-D2): the scrutiny findings and the user-testing scenario results of one
//! milestone, with the evidence the validators recorded.

use std::collections::HashMap;

use super::{milestone_events, tabs_end};
use crate::metrics::{Notes, rework_windows};
use crate::model::{CycleTemplate, Event, EventId, EventKind, FeatureId, MilestoneId, Timestamp};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FindingRow {
    pub event_id: EventId,
    pub feature: Option<FeatureId>,
    /// `blocking` or `non-blocking`.
    pub severity: String,
    /// `resolved` once `rework.resolved` closes the finding's window, else `open`; a
    /// non-blocking finding opens no window and has no resolution event, so it stays `open`.
    pub status: String,
    pub at: Timestamp,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScenarioRow {
    pub event_id: EventId,
    pub feature: Option<FeatureId>,
    /// The scenario id (`VAL-A-001`) when the notes name it.
    pub key: Option<String>,
    /// `passed` or `failed`.
    pub result: String,
    pub at: Timestamp,
    /// The `qa.evidence` events kept for the result.
    pub evidence: Vec<EventId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewTab {
    pub milestone: MilestoneId,
    pub findings: Vec<FindingRow>,
    pub scenarios: Vec<ScenarioRow>,
    /// Evidence of a card that has no scenario result.
    pub loose_evidence: Vec<EventId>,
}

pub fn review_tab(
    events: &[Event],
    milestone: &MilestoneId,
    template: &CycleTemplate,
    notes: &Notes,
) -> ReviewTab {
    let own = milestone_events(events, milestone);
    let closed: HashMap<EventId, bool> = tabs_end(events)
        .map(|end| {
            rework_windows(events, template, notes, end)
                .into_iter()
                .map(|window| (window.opened_by, window.closed_by.is_some()))
                .collect()
        })
        .unwrap_or_default();

    let findings = own
        .iter()
        .filter_map(|event| {
            let severity = match event.kind {
                EventKind::FindingBlocking => "blocking",
                EventKind::FindingNonBlocking => "non-blocking",
                _ => return None,
            };
            let resolved = closed.get(&event.id).copied().unwrap_or(false);
            Some(FindingRow {
                event_id: event.id.clone(),
                feature: event.feature_id.clone(),
                severity: severity.to_owned(),
                status: if resolved { "resolved" } else { "open" }.to_owned(),
                at: event.at,
            })
        })
        .collect();

    let mut scenarios: Vec<ScenarioRow> = own
        .iter()
        .filter_map(|event| {
            let result = match event.kind {
                EventKind::QaScenarioPassed => "passed",
                EventKind::QaScenarioFailed => "failed",
                _ => return None,
            };
            Some(ScenarioRow {
                event_id: event.id.clone(),
                feature: event.feature_id.clone(),
                key: notes.key(event).map(str::to_owned),
                result: result.to_owned(),
                at: event.at,
                evidence: Vec::new(),
            })
        })
        .collect();
    let mut loose_evidence = Vec::new();
    for evidence in own.iter().filter(|e| e.kind == EventKind::QaEvidence) {
        let of_card = |row: &&mut ScenarioRow| row.feature == evidence.feature_id;
        let index = scenarios
            .iter_mut()
            .enumerate()
            .filter(|(_, row)| of_card(row))
            .map(|(index, row)| (index, row.at))
            .reduce(|kept, next| if kept.1 >= evidence.at { kept } else { next })
            .map(|(index, _)| index);
        match index {
            Some(index) => scenarios[index].evidence.push(evidence.id.clone()),
            None => loose_evidence.push(evidence.id.clone()),
        }
    }
    ReviewTab {
        milestone: milestone.clone(),
        findings,
        scenarios,
        loose_evidence,
    }
}
