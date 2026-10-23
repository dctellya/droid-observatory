use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::config::MissionConfig;
use crate::model::{Event, Lane, MilestoneId, ModelId, Overlay, Role, SourcePtr, Timestamp};

use super::agents::AgentRow;
use super::card::Card;
use super::details::TimelineRow;
use super::filter::{ViewConfig, WorkerGrouping};
use super::gate::{GateGroup, GatePlaceholder, mission_branch};
use super::scrubber::OverlayMarker;
use super::state::{BoardState, TrayEntry};

/// A set of rows of one lane under one heading: a worker or a feature in Build, a milestone in
/// Integrate.
#[derive(Clone, Debug, PartialEq)]
pub struct RowGroup {
    /// Session, feature or milestone id; empty for rows that have none.
    pub key: String,
    pub label: String,
    /// The role of the session, when grouped per worker.
    pub role: Option<Role>,
    pub rows: Vec<TimelineRow>,
}

/// One lane column as the UI draws it.
#[derive(Clone, Debug, PartialEq)]
pub struct LaneView {
    pub lane: Lane,
    pub name: &'static str,
    /// Stable accessible name, `lane-<n>`, which validators address the column by.
    pub aria_label: String,
    pub card_count: usize,
    /// Events in the lane under the filter and the scrubber.
    pub event_count: u64,
    /// The milestone cards whose most recent open activity is in this lane.
    pub cards: Vec<Card>,
    /// True when `groups` are sub-lanes (Build) or per-milestone sections (Integrate).
    pub grouped: bool,
    pub groups: Vec<RowGroup>,
    /// Gate only, while no PR exists.
    pub placeholder: Option<GatePlaceholder>,
}

/// The models the mission command declares per role (`[mission]` in the config).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeclaredModels {
    pub orchestrator: ModelId,
    pub worker: ModelId,
    pub validator: ModelId,
}

impl DeclaredModels {
    pub fn from_config(config: &MissionConfig) -> Self {
        Self {
            orchestrator: ModelId(config.orchestrator_model.clone()),
            worker: ModelId(config.worker_model.clone()),
            validator: ModelId(config.validator_model.clone()),
        }
    }

    /// The declared model of a role; none for a person or an outside actor.
    pub fn of(&self, role: Role) -> Option<&ModelId> {
        match role {
            Role::Orchestrator => Some(&self.orchestrator),
            Role::Worker => Some(&self.worker),
            Role::ScrutinyValidator | Role::UserTestingValidator => Some(&self.validator),
            Role::Human | Role::External | Role::Unresolved => None,
        }
    }
}

/// The roles the Panel reports on, in order.
const PANEL_ROLES: [Role; 4] = [
    Role::Orchestrator,
    Role::Worker,
    Role::ScrutinyValidator,
    Role::UserTestingValidator,
];

/// Tokens of the sessions of one role (the latest `usage.reported` of each).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleTokens {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_write: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservedModel {
    pub model: ModelId,
    /// Events attributed to the model.
    pub events: u64,
}

/// One role of the Panel: tokens and the models seen against the declared one.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PanelRole {
    pub role: Role,
    pub sessions: u64,
    pub tokens: RoleTokens,
    pub declared: ModelId,
    /// Sorted by model id.
    pub observed: Vec<ObservedModel>,
    /// A model other than the declared one was observed.
    pub drifted: bool,
}

/// The Panel (time, waste, cost, model drift): drawn above the lanes, never one of them.
/// Cost and waste arrive with M5.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PanelView {
    /// Stable accessible name, `panel`.
    pub aria_label: String,
    /// First to last event of the board.
    pub elapsed_ms: Option<u64>,
    pub roles: Vec<PanelRole>,
}

impl PanelView {
    fn of(state: &BoardState, kept: &[&Event], declared: &DeclaredModels) -> Self {
        let roles = PANEL_ROLES
            .iter()
            .filter_map(|&role| {
                let declared = declared.of(role)?.clone();
                let agents = || state.agents.iter().filter(|agent| agent.role == role);
                let mut tokens = RoleTokens::default();
                for usage in agents().filter_map(|agent| agent.tokens) {
                    tokens.input += usage.input;
                    tokens.output += usage.output;
                    tokens.cache_read += usage.cache_read;
                    tokens.cache_write += usage.cache_write;
                }
                let mut seen: BTreeMap<&ModelId, u64> = BTreeMap::new();
                for model in kept
                    .iter()
                    .filter(|event| event.role == role)
                    .filter_map(|event| event.model.as_ref())
                {
                    *seen.entry(model).or_default() += 1;
                }
                let drifted = seen.keys().any(|model| **model != declared);
                Some(PanelRole {
                    role,
                    sessions: agents().count() as u64,
                    tokens,
                    declared,
                    observed: seen
                        .into_iter()
                        .map(|(model, events)| ObservedModel {
                            model: model.clone(),
                            events,
                        })
                        .collect(),
                    drifted,
                })
            })
            .collect();
        let elapsed_ms = match (kept.first(), kept.last()) {
            (Some(first), Some(last)) => {
                u64::try_from((last.at.0 - first.at.0).whole_milliseconds()).ok()
            }
            _ => None,
        };
        Self {
            aria_label: PANEL_LABEL.to_owned(),
            elapsed_ms,
            roles,
        }
    }
}

const PANEL_LABEL: &str = "panel";
const OVERLAY_HUMAN_LABEL: &str = "overlay-human";

/// The overlay filter list (`overlay-human`): the events a person caused, which also carry a
/// marker on their own phase lane.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OverlayView {
    pub aria_label: String,
    pub markers: Vec<OverlayMarker>,
}

/// The board: the seven phase lanes in order, the Panel and the unmapped tray.
#[derive(Clone, Debug, PartialEq)]
pub struct BoardView {
    pub lanes: Vec<LaneView>,
    pub panel: PanelView,
    pub gate: GateGroup,
    pub overlay_human: OverlayView,
    /// The sessions at the scrubbed instant; `agent_entries` names them for the agent panel.
    pub agents: Vec<AgentRow>,
    pub event_count: u64,
    pub tray_unknown: u64,
    pub tray: Vec<TrayEntry>,
    pub grouping: WorkerGrouping,
    pub now: Option<Timestamp>,
}

impl BoardView {
    pub fn empty() -> Self {
        let lanes = Lane::ALL
            .iter()
            .map(|&lane| LaneView {
                lane,
                name: lane.name(),
                aria_label: format!("lane-{}", lane.number()),
                card_count: 0,
                event_count: 0,
                cards: Vec::new(),
                grouped: is_grouped(lane),
                groups: Vec::new(),
                placeholder: None,
            })
            .collect();
        Self {
            lanes,
            panel: PanelView {
                aria_label: PANEL_LABEL.to_owned(),
                elapsed_ms: None,
                roles: Vec::new(),
            },
            gate: GateGroup::default(),
            overlay_human: OverlayView {
                aria_label: OVERLAY_HUMAN_LABEL.to_owned(),
                markers: Vec::new(),
            },
            agents: Vec::new(),
            event_count: 0,
            tray_unknown: 0,
            tray: Vec::new(),
            grouping: WorkerGrouping::default(),
            now: None,
        }
    }

    /// The board of `events` (in time order) under `cfg`: what the UI draws, with nothing left
    /// to compute. `branch` is the mission branch the Gate placeholder names (`branch_of`).
    pub fn build(
        events: &[Event],
        cfg: &ViewConfig,
        declared: &DeclaredModels,
        branch: &str,
    ) -> Self {
        let (state, kept) = BoardState::fold_kept(events, cfg);
        let mut board = Self::empty();
        for view in &mut board.lanes {
            let in_lane = kept.iter().copied().filter(|event| event.lane == view.lane);
            view.cards = state
                .cards
                .iter()
                .filter(|card| card.lane == view.lane)
                .cloned()
                .collect();
            view.card_count = view.cards.len();
            view.groups = lane_groups(view.lane, cfg.grouping, in_lane);
            view.event_count = view.groups.iter().map(|g| g.rows.len() as u64).sum();
            if view.lane == Lane::Gate {
                view.placeholder = state.gate.placeholder(branch);
            }
        }
        board.overlay_human.markers = kept
            .iter()
            .filter(|event| event.overlay == Some(Overlay::Human))
            .map(|event| OverlayMarker::of(event))
            .collect();
        board.panel = PanelView::of(&state, &kept, declared);
        board.gate = state.gate.clone();
        board.agents = state.agents;
        board.event_count = state.event_count;
        board.tray_unknown = state.tray_unknown;
        board.tray = state.tray;
        board.grouping = cfg.grouping;
        board.now = state.now;
        board
    }

    pub fn card_count(&self) -> usize {
        self.lanes.iter().map(|lane| lane.card_count).sum()
    }

    /// The figures a replay of the same events must reproduce: what the `board` command sends.
    pub fn digest(&self) -> BoardDigest {
        let mut digest = BoardDigest {
            event_count: self.event_count,
            tray_unknown: self.tray_unknown,
            lane_events: [0; Lane::ALL.len()],
            lane_cards: [0; Lane::ALL.len()],
            cards: Vec::new(),
        };
        for (index, view) in self.lanes.iter().enumerate().take(Lane::ALL.len()) {
            digest.lane_events[index] = view.event_count;
            digest.lane_cards[index] = view.card_count;
            digest.cards.extend(view.cards.iter().map(|card| CardLane {
                milestone: card.milestone.clone(),
                lane: card.lane,
            }));
        }
        digest
    }
}

/// The milestone cards and the lane each sits in.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardLane {
    pub milestone: MilestoneId,
    pub lane: Lane,
}

/// The counts of a board, in lane order (Plan first), for the `board` command and for comparing
/// a live capture with the replay of the same file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoardDigest {
    pub event_count: u64,
    pub tray_unknown: u64,
    pub lane_events: [u64; Lane::ALL.len()],
    pub lane_cards: [usize; Lane::ALL.len()],
    pub cards: Vec<CardLane>,
}

/// The branch the Gate placeholder names for a mission called `name`.
pub fn branch_of(config: &MissionConfig, name: &str) -> String {
    mission_branch(&config.branch_pattern, name)
}

fn is_grouped(lane: Lane) -> bool {
    matches!(lane, Lane::Build | Lane::Integrate)
}

/// The rows of one lane, split under their headings where the lane has any.
fn lane_groups<'a>(
    lane: Lane,
    grouping: WorkerGrouping,
    events: impl Iterator<Item = &'a Event>,
) -> Vec<RowGroup> {
    let mut groups: Vec<RowGroup> = Vec::new();
    for event in events {
        let (key, none_label, role) = match (lane, grouping) {
            (Lane::Build, WorkerGrouping::PerWorker) => (
                event.session_id.as_ref().map(|s| s.0.clone()),
                "(no session)",
                Some(event.role),
            ),
            (Lane::Build, WorkerGrouping::PerFeature) => (
                event.feature_id.as_ref().map(|f| f.0.clone()),
                "(no feature)",
                None,
            ),
            (Lane::Integrate, _) => (
                event.milestone_id.as_ref().map(|m| m.0.clone()),
                "(no milestone)",
                None,
            ),
            _ => (Some(String::new()), "", None),
        };
        let key = key.unwrap_or_default();
        let index = match groups.iter().position(|group| group.key == key) {
            Some(index) => index,
            None => {
                groups.push(RowGroup {
                    label: if key.is_empty() {
                        none_label.to_owned()
                    } else {
                        key.clone()
                    },
                    key,
                    role,
                    rows: Vec::new(),
                });
                groups.len() - 1
            }
        };
        groups[index].rows.push(TimelineRow::of(event));
    }
    groups
}

/// The Integrate rows of one milestone (none: the events with no milestone).
#[derive(Clone, Debug, PartialEq)]
pub struct IntegrateGroup {
    pub milestone: Option<MilestoneId>,
    pub rows: Vec<TimelineRow>,
    /// Rows that are `gate_command.*` events.
    pub gate_runs: u64,
}

/// The Integrate lane per milestone, in the order of the milestones' first Integrate event,
/// rows oldest first.
pub fn integrate_timeline(events: &[Event]) -> Vec<IntegrateGroup> {
    let mut groups: Vec<IntegrateGroup> = Vec::new();
    for event in events.iter().filter(|event| event.lane == Lane::Integrate) {
        let index = match groups
            .iter()
            .position(|group| group.milestone == event.milestone_id)
        {
            Some(index) => index,
            None => {
                groups.push(IntegrateGroup {
                    milestone: event.milestone_id.clone(),
                    rows: Vec::new(),
                    gate_runs: 0,
                });
                groups.len() - 1
            }
        };
        let group = &mut groups[index];
        group.gate_runs += u64::from(event.kind.prd_name().starts_with("gate_command."));
        group.rows.push(TimelineRow::of(event));
    }
    for group in &mut groups {
        group.rows.sort_by_key(|row| row.at);
    }
    groups
}

/// A pointer is only shown as text: where the raw line lives.
pub fn pointer_text(pointer: &SourcePtr) -> String {
    match pointer {
        SourcePtr::Jsonl {
            path,
            byte_offset,
            byte_len,
        }
        | SourcePtr::Rpc {
            path,
            byte_offset,
            byte_len,
        } => format!("{}:{byte_offset}+{byte_len}", path.display()),
        SourcePtr::Git { repo, sha } => format!("{}@{sha}", repo.display()),
        SourcePtr::GitHub { url } => url.clone(),
        SourcePtr::App { action_id } => format!("app action {action_id}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_board_has_seven_lanes_a_panel_and_no_cards() {
        let board = BoardView::empty();

        assert_eq!(board.lanes.len(), 7);
        assert_eq!(board.card_count(), 0);
        let names: Vec<&str> = board.lanes.iter().map(|l| l.name).collect();
        assert_eq!(
            names,
            [
                "Plan",
                "Contract",
                "Build",
                "Scrutiny",
                "User testing",
                "Integrate",
                "Gate",
            ]
        );
        let labels: Vec<&str> = board.lanes.iter().map(|l| l.aria_label.as_str()).collect();
        assert_eq!(
            labels,
            [
                "lane-1", "lane-2", "lane-3", "lane-4", "lane-5", "lane-6", "lane-7"
            ]
        );
        assert_eq!(board.panel.aria_label, "panel");
    }
}
