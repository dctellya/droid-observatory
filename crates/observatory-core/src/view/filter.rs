use serde::{Deserialize, Serialize};

use std::collections::BTreeSet;

use crate::model::{Event, FeatureId, MilestoneId, ModelId, Role, Timestamp};

use super::card::ValidationBadge;
use super::gate::GateBadge;

/// FR-B2: how the Build lane splits into rows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerGrouping {
    #[default]
    PerWorker,
    PerFeature,
}

/// FR-B5. Milestone, feature, role and model narrow the events; validation narrows the cards
/// (and the events of the cards kept); gate keeps the whole board or empties it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Filter {
    pub milestone: Option<MilestoneId>,
    pub feature: Option<FeatureId>,
    pub role: Option<Role>,
    pub model: Option<ModelId>,
    pub validation: Option<ValidationBadge>,
    pub gate: Option<GateBadge>,
}

impl Filter {
    /// The event-level part of the filter.
    pub fn keeps(&self, event: &Event) -> bool {
        self.milestone
            .as_ref()
            .is_none_or(|m| event.milestone_id.as_ref() == Some(m))
            && self
                .feature
                .as_ref()
                .is_none_or(|f| event.feature_id.as_ref() == Some(f))
            && self.role.is_none_or(|r| event.role == r)
            && self
                .model
                .as_ref()
                .is_none_or(|m| event.model.as_ref() == Some(m))
    }
}

/// The six controls of the filter bar (FR-B5), in the order they are drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterField {
    Milestone,
    Feature,
    Role,
    Model,
    Validation,
    Gate,
}

impl FilterField {
    pub const ALL: [FilterField; 6] = [
        FilterField::Milestone,
        FilterField::Feature,
        FilterField::Role,
        FilterField::Model,
        FilterField::Validation,
        FilterField::Gate,
    ];

    /// Stable accessible name of the control.
    pub fn aria_label(self) -> &'static str {
        match self {
            FilterField::Milestone => "filter-milestone",
            FilterField::Feature => "filter-feature",
            FilterField::Role => "filter-role",
            FilterField::Model => "filter-model",
            FilterField::Validation => "filter-validation",
            FilterField::Gate => "filter-gate",
        }
    }

    /// Visible label of the control.
    pub fn label(self) -> &'static str {
        match self {
            FilterField::Milestone => "Milestone",
            FilterField::Feature => "Feature",
            FilterField::Role => "Role",
            FilterField::Model => "Model",
            FilterField::Validation => "Validation state",
            FilterField::Gate => "Gate state",
        }
    }
}

/// One entry of a filter control: `value` is what `Filter::set` takes back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Choice {
    pub value: String,
    pub label: String,
}

impl Choice {
    fn same(value: &str) -> Self {
        Self {
            value: value.to_owned(),
            label: value.to_owned(),
        }
    }
}

/// What each filter control offers for a set of events. The validation and gate controls
/// offer every state.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FilterOptions {
    pub milestones: Vec<Choice>,
    pub features: Vec<Choice>,
    pub roles: Vec<Choice>,
    pub models: Vec<Choice>,
    pub validation: Vec<Choice>,
    pub gate: Vec<Choice>,
}

impl FilterOptions {
    pub fn of(events: &[Event]) -> Self {
        let events: Vec<&Event> = events
            .iter()
            .filter(|event| !event.kind.is_panel_only())
            .collect();
        let events = events.as_slice();
        let milestones: BTreeSet<&str> = events
            .iter()
            .filter_map(|e| e.milestone_id.as_ref().map(|m| m.0.as_str()))
            .collect();
        let features: BTreeSet<&str> = events
            .iter()
            .filter_map(|e| e.feature_id.as_ref().map(|f| f.0.as_str()))
            .collect();
        let models: BTreeSet<&str> = events
            .iter()
            .filter_map(|e| e.model.as_ref().map(|m| m.0.as_str()))
            .collect();
        let choices = |values: BTreeSet<&str>| values.into_iter().map(Choice::same).collect();
        Self {
            milestones: choices(milestones),
            features: choices(features),
            roles: Role::ALL
                .into_iter()
                .filter(|role| events.iter().any(|e| e.role == *role))
                .map(|role| Choice::same(role.as_str()))
                .collect(),
            models: choices(models),
            validation: ValidationBadge::ALL
                .into_iter()
                .map(|badge| Choice {
                    value: badge.as_str().to_owned(),
                    label: badge.label().to_owned(),
                })
                .collect(),
            gate: GateBadge::ALL
                .into_iter()
                .map(|badge| Choice::same(badge.as_str()))
                .collect(),
        }
    }

    pub fn of_field(&self, field: FilterField) -> &[Choice] {
        match field {
            FilterField::Milestone => &self.milestones,
            FilterField::Feature => &self.features,
            FilterField::Role => &self.roles,
            FilterField::Model => &self.models,
            FilterField::Validation => &self.validation,
            FilterField::Gate => &self.gate,
        }
    }
}

impl Filter {
    /// Sets one control from the value of its choice. An empty or unknown value clears it.
    pub fn set(&mut self, field: FilterField, value: &str) {
        let text = (!value.is_empty()).then(|| value.to_owned());
        match field {
            FilterField::Milestone => self.milestone = text.map(MilestoneId),
            FilterField::Feature => self.feature = text.map(FeatureId),
            FilterField::Role => self.role = Role::parse(value),
            FilterField::Model => self.model = text.map(ModelId),
            FilterField::Validation => self.validation = ValidationBadge::parse(value),
            FilterField::Gate => self.gate = GateBadge::parse(value),
        }
    }

    /// The value the control shows for this filter; empty when it is not set.
    pub fn value(&self, field: FilterField) -> String {
        match field {
            FilterField::Milestone => self.milestone.as_ref().map(|m| m.0.clone()),
            FilterField::Feature => self.feature.as_ref().map(|f| f.0.clone()),
            FilterField::Role => self.role.map(|r| r.as_str().to_owned()),
            FilterField::Model => self.model.as_ref().map(|m| m.0.clone()),
            FilterField::Validation => self.validation.map(|v| v.as_str().to_owned()),
            FilterField::Gate => self.gate.map(|g| g.as_str().to_owned()),
        }
        .unwrap_or_default()
    }
}

/// Everything the board is a function of: the filter, the Build lane grouping and the
/// scrubber position (`None`: live, every event).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ViewConfig {
    pub filter: Filter,
    pub grouping: WorkerGrouping,
    pub at: Option<Timestamp>,
}
