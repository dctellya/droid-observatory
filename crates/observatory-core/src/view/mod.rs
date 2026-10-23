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
mod summary;

pub use agents::{AgentRow, AgentStatus, Marker, MarkerKind, agent_markers};
pub use board::{
    BoardView, DeclaredModels, IntegrateGroup, LaneView, ObservedModel, OverlayView, PanelRole,
    PanelView, RoleTokens, RowGroup, branch_of, integrate_timeline, pointer_text,
};
pub use card::{Card, CardNow, ValidationBadge, card_lane};
pub use details::{Details, RawLineView, TimelineRow, details, raw_line_view};
pub use filter::{Choice, Filter, FilterField, FilterOptions, ViewConfig, WorkerGrouping};
pub use gate::{ClockLabel, GateBadge, GateGroup, GatePlaceholder, GateVerdict, mission_branch};
pub use library::{LibraryEntry, MissionRow, library};
pub use scrub::{ScrubbedLine, scrub};
pub use scrubber::{
    OverlayMarker, Scrubber, lane_counts, overlay_marker_rows, overlay_markers, scrub_to,
    tray_count,
};
pub use state::{BoardState, SubLane, TrayEntry};
pub use summary::MissionSummary;
