use leptos::prelude::*;
use observatory_core::ingest::SessionInfo;
use observatory_core::model::MilestoneId;
use observatory_core::view::{GateGroupView, MilestoneWorktree};

/// `[metrics].budget` as the window read it; zero (the default, and always in the browser host)
/// leaves the Panel's budget meter off.
#[derive(Clone, Copy)]
pub struct MetricsBudget(pub RwSignal<f64>);

/// The text of the price table the Panel and the Details parse: the bundled one until the window
/// has read `[metrics].price_table` through the shell (`price_table`); the browser host keeps the
/// bundled one.
#[derive(Clone, Copy)]
pub struct PriceTableText(pub RwSignal<String>);

/// What the board hands down to its cards and rows: how to open a milestone's Details and the
/// sessions of the opened tree, which name an agent.
#[derive(Clone)]
pub struct BoardCtx {
    pub open_details: Callback<MilestoneId>,
    pub sessions: ArcRwSignal<Vec<SessionInfo>>,
    /// The Gate group as the lane and the Details draw it.
    pub gate: Signal<GateGroupView>,
    /// The worktrees git listed for the milestones at the last poll.
    pub worktrees: ArcRwSignal<Vec<MilestoneWorktree>>,
}
