use serde::{Deserialize, Serialize};

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

/// Everything the board is a function of: the filter, the Build lane grouping and the
/// scrubber position (`None`: live, every event).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ViewConfig {
    pub filter: Filter,
    pub grouping: WorkerGrouping,
    pub at: Option<Timestamp>,
}
