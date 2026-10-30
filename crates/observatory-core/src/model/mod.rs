//! Canonical event model (architecture §3). Compiled to wasm for the UI (architecture I9).

mod cycle;
mod event;
mod ids;
mod kind;
mod overlay;
mod role;
mod timestamp;
mod usage;

pub use cycle::{Clock, CycleError, CycleTemplate, LaneGrouping, LaneId, LaneSpec, SessionRules};
pub use event::{Event, SourcePtr};
pub use ids::{EventId, FeatureId, MilestoneId, MissionId, ModelId, SessionId};
pub use kind::EventKind;
pub use overlay::Overlay;
pub use role::Role;
pub use timestamp::Timestamp;
pub use usage::TokenUsage;
