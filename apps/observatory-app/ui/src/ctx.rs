use leptos::prelude::*;
use observatory_core::ingest::SessionInfo;
use observatory_core::model::MilestoneId;

/// What the board hands down to its cards and rows: how to open a milestone's Details and the
/// sessions of the opened tree, which name an agent.
#[derive(Clone)]
pub struct BoardCtx {
    pub open_details: Callback<MilestoneId>,
    pub sessions: ArcRwSignal<Vec<SessionInfo>>,
}
