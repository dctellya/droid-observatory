//! Validation criteria (FR-D3): the checkpoints of the plan per feature and the events that
//! satisfied each. The events carry no criterion text (it lives in the plan's raw line, which
//! `defined_by` opens), so the criteria are the checkpoints the cycle records for a feature.

use std::collections::BTreeSet;

use super::milestone_events;
use crate::metrics::{Notes, events_of_feature};
use crate::model::{Event, EventId, EventKind, FeatureId, MilestoneId};

type Satisfies = fn(&EventKind) -> bool;

const CHECKPOINTS: [(&str, Satisfies); 6] = [
    ("Tests run", |k| matches!(k, EventKind::TestsRun)),
    ("Committed", |k| matches!(k, EventKind::CommitCreated)),
    ("Task done", |k| matches!(k, EventKind::TaskDone)),
    ("Scrutiny passed", |k| {
        matches!(k, EventKind::ScrutinyPassed)
    }),
    ("User-testing scenario passed", |k| {
        matches!(k, EventKind::QaScenarioPassed)
    }),
    ("Gate command passed", |k| {
        matches!(k, EventKind::GateCommandPassed)
    }),
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CriterionRow {
    pub label: String,
    pub satisfied_by: Vec<EventId>,
    /// `satisfied`, or `unsatisfied` when no event satisfied it.
    pub status: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeatureCriteria {
    pub feature: FeatureId,
    /// The `feature.defined` event whose raw line holds the plan's text.
    pub defined_by: Option<EventId>,
    pub criteria: Vec<CriterionRow>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CriteriaTab {
    pub milestone: MilestoneId,
    pub features: Vec<FeatureCriteria>,
}

pub fn criteria_tab(events: &[Event], milestone: &MilestoneId, notes: &Notes) -> CriteriaTab {
    let features: BTreeSet<FeatureId> = milestone_events(events, milestone)
        .iter()
        .filter_map(|event| event.feature_id.clone())
        .collect();
    CriteriaTab {
        milestone: milestone.clone(),
        features: features
            .into_iter()
            .map(|feature| {
                let own = events_of_feature(events, &feature, notes);
                let criteria = CHECKPOINTS
                    .iter()
                    .map(|(label, satisfies)| {
                        let satisfied_by: Vec<EventId> = own
                            .iter()
                            .filter(|event| satisfies(&event.kind))
                            .map(|event| event.id.clone())
                            .collect();
                        CriterionRow {
                            label: (*label).to_owned(),
                            status: if satisfied_by.is_empty() {
                                "unsatisfied"
                            } else {
                                "satisfied"
                            }
                            .to_owned(),
                            satisfied_by,
                        }
                    })
                    .collect();
                FeatureCriteria {
                    defined_by: own
                        .iter()
                        .find(|event| event.kind == EventKind::FeatureDefined)
                        .map(|event| event.id.clone()),
                    feature,
                    criteria,
                }
            })
            .collect(),
    }
}
