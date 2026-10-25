//! View-models the UI renders as they are; no component computes anything itself (architecture I2).

mod agent_panel;
mod agents;
mod board;
mod card;
mod details;
mod filter;
mod gate;
mod hosted;
mod launch;
mod library;
mod live_output;
mod post_mortem;
mod scrub;
mod scrubber;
mod state;
mod summary;

pub use agent_panel::{AgentEntry, AgentStream, agent_entries, agent_stream, session_name};
pub use agents::{AgentRow, AgentStatus, Marker, MarkerKind, agent_markers};
pub use board::{
    BoardDigest, BoardView, CardLane, DeclaredModels, IntegrateGroup, LaneView, ObservedModel,
    OverlayView, PanelRole, PanelView, RoleTokens, RowGroup, branch_of, integrate_timeline,
    pointer_text,
};
pub use card::{Card, CardNow, IdleReason, ValidationBadge, card_lane};
pub use details::{Details, RawLineView, TimelineRow, details, raw_line_view};
pub use filter::{Choice, Filter, FilterField, FilterOptions, ViewConfig, WorkerGrouping};
pub use gate::{ClockLabel, GateBadge, GateGroup, GatePlaceholder, GateVerdict, mission_branch};
pub use hosted::{
    AckRow, ControlSent, HostedState, HostedStatus, Launched, PermissionRow, QuestionRow,
};
pub use launch::{
    AUTONOMY_LEVELS, LaunchConfig, LaunchError, LaunchField, LaunchForm, PerRole, session_uuid,
};
pub use library::{LibraryEntry, MissionRow, library};
pub use live_output::{LiveOutput, OutputLines};
pub use post_mortem::{ClockReading, LaneSummary, PostMortemInput, post_mortem_input};
pub use scrub::{ScrubbedLine, scrub};
pub use scrubber::{
    OverlayMarker, Scrubber, lane_counts, overlay_marker_rows, overlay_markers, scrub_to,
    tray_count,
};
pub use state::{BoardState, RoleTurns, SubLane, TrayEntry};
pub use summary::MissionSummary;
