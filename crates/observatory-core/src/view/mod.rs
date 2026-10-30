//! View-models the UI renders as they are; no component computes anything itself (architecture I2).

mod agent_panel;
mod agents;
mod board;
mod card;
mod controls;
mod details;
mod filter;
mod gate;
mod hosted;
mod launch;
mod library;
mod live_output;
mod panel_metrics;
mod post_mortem;
mod scrub;
mod scrubber;
mod state;
mod summary;
mod tabs;
mod worktree;

pub use agent_panel::{AgentEntry, AgentStream, agent_entries, agent_stream, session_name};
pub use agents::{AgentRow, AgentStatus, Marker, MarkerKind, agent_markers};
pub use board::{
    BoardDigest, BoardView, CardLane, DeclaredModels, IntegrateGroup, LaneView, ObservedModel,
    OverlayView, PanelRole, PanelView, RoleTokens, RowGroup, branch_of, integrate_timeline,
    pointer_text,
};
pub use card::{Card, CardNow, IdleReason, ReturnArc, ValidationBadge, card_lane};
pub use controls::{
    ActionRecorded, ControlBar, ControlMode, RUNBOOK_PATH, RunState, StallBanner, StallBatch,
    StallFlagView, StallRow, TERMINAL_INSTRUCTION, terminal_text,
};
pub use details::{Details, RawLineView, TimelineRow, details, raw_line_view};
pub use filter::{Choice, Filter, FilterField, FilterOptions, ViewConfig, WorkerGrouping};
pub use gate::{
    ClockLabel, GateBadge, GateChanged, GateCheckRow, GateClockLine, GateGroup, GateGroupView,
    GatePlaceholder, GateVerdict, MergeConfirmation, MergeOffer, MergePressed, mission_branch,
};
pub use hosted::{
    AckRow, ControlSent, HostedState, HostedStatus, Launched, PermissionRow, QuestionRow,
};
pub use launch::{
    AUTONOMY_LEVELS, LaunchConfig, LaunchError, LaunchField, LaunchForm, PerRole, session_uuid,
};
pub use library::{Headline, LibraryEntry, MissionRow, library};
pub use live_output::{LiveOutput, OutputLines};
pub use panel_metrics::{
    FeatureTimeline, GroupTotal, PanelMetrics, RUN_LABEL, ReworkBand, SparkPoint, Sparkline,
    StateBand, TimelineChart, WasteBar, WasteChart, WasteSegment, panel_metrics,
};
pub use post_mortem::{ClockReading, LaneSummary, PostMortemInput, post_mortem_input};
pub use scrub::{ScrubbedLine, scrub};
pub use scrubber::{
    OverlayMarker, Scrubber, lane_counts, overlay_marker_rows, overlay_markers, scrub_to,
    tray_count,
};
pub use state::{BoardState, RoleTurns, SubLane, TrayEntry};
pub use summary::MissionSummary;
pub use tabs::{
    CommitRow, CommitsTab, CostTab, CriteriaTab, CriterionRow, EventRow, FeatureCriteria,
    FindingRow, GateClock, GateTab, HistoryRow, PrLink, ReviewTab, ReworkRow, ScenarioRow,
    UsageRow, WasteRow, commits_tab, cost_tab, criteria_tab, gate_tab, review_tab,
};
pub use worktree::{MilestoneWorktree, milestone_worktrees};
