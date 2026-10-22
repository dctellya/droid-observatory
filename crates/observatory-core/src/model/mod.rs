//! Canonical event model (architecture §3). Compiled to wasm for the UI (architecture I9).

mod event;
mod ids;
mod kind;
mod lane;
mod role;
mod timestamp;
mod usage;

pub use event::{Event, SourcePtr};
pub use ids::{EventId, FeatureId, MilestoneId, MissionId, ModelId, SessionId};
pub use kind::EventKind;
pub use lane::Lane;
pub use role::Role;
pub use timestamp::Timestamp;
pub use usage::TokenUsage;
