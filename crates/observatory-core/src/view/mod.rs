//! View-models the UI renders as they are; no component computes anything itself (architecture I2).

mod agents;
mod board;
mod card;
mod details;
mod filter;
mod gate;
mod library;
mod scrub;
mod scrubber;
mod state;

pub use agents::{AgentRow, AgentStatus, Marker, MarkerKind, agent_markers};
pub use board::{BoardView, LaneView, PanelView};
pub use card::{Card, CardNow, ValidationBadge, card_lane};
pub use details::{Details, RawLineView, TimelineRow, details, raw_line_view};
pub use filter::{Filter, ViewConfig, WorkerGrouping};
pub use gate::{GateBadge, GateGroup, GateVerdict};
pub use library::{LibraryEntry, MissionRow, library};
pub use scrub::{ScrubbedLine, scrub};
pub use scrubber::{lane_counts, overlay_markers, scrub_to, tray_count};
pub use state::{BoardState, SubLane, TrayEntry};
