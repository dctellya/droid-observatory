use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::model::{
    Event, EventId, EventKind, Lane, MilestoneId, Overlay, Role, SourcePtr, Timestamp,
};

use super::agents::AgentRow;
use super::card::Card;
use super::filter::{ViewConfig, WorkerGrouping};
use super::gate::GateGroup;
use super::scrubber::lane_index;

/// An event the mapping did not know (FR-B7). The pointer opens the raw line.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrayEntry {
    pub event_id: EventId,
    pub raw_type: String,
    pub pointer: SourcePtr,
    pub at: Timestamp,
}

/// One row of the Build lane (FR-B2): a worker session, or a feature.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubLane {
    /// Session id or feature id; empty for events without a feature.
    pub key: String,
    /// Set when grouped per worker.
    pub role: Option<Role>,
    pub event_count: u64,
    pub last_at: Timestamp,
}

/// The board for one filter and scrubber position (architecture §7).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BoardState {
    /// In order of the milestone's first event.
    pub cards: Vec<Card>,
    /// Header of the Gate lane; a placeholder before a PR.
    pub gate: GateGroup,
    pub tray_unknown: u64,
    pub tray: Vec<TrayEntry>,
    pub agents: Vec<AgentRow>,
    pub grouping: WorkerGrouping,
    /// Rows of the Build lane under `grouping`.
    pub sublanes: Vec<SubLane>,
    pub event_count: u64,
    /// Events per phase, in lane order.
    pub lane_counts: [u64; Lane::ALL.len()],
    pub human_overlays: u64,
    pub commander_overlays: u64,
    /// The scrubber position, or the time of the last event when live.
    pub now: Option<Timestamp>,
}

impl BoardState {
    pub fn new(grouping: WorkerGrouping) -> Self {
        Self {
            grouping,
            ..Self::default()
        }
    }

    /// The board of `events` (in time order) under `cfg`. The scrubber keeps the events up to
    /// `cfg.at`; the filter narrows what is left.
    pub fn fold(events: &[Event], cfg: &ViewConfig) -> Self {
        let kept =
            |event: &&Event| cfg.at.is_none_or(|at| event.at <= at) && cfg.filter.keeps(event);
        let build = |events: &mut dyn Iterator<Item = &Event>| {
            let mut state = Self::new(cfg.grouping);
            for event in events {
                state.apply(event);
            }
            state
        };

        let mut state = build(&mut events.iter().filter(kept));
        if let Some(gate) = cfg.filter.gate
            && state.gate.badge != gate
        {
            state = Self::new(cfg.grouping);
        }
        if let Some(validation) = cfg.filter.validation {
            let milestones: HashSet<&MilestoneId> = state
                .cards
                .iter()
                .filter(|card| card.validation == validation)
                .map(|card| &card.milestone)
                .collect();
            let narrowed = build(&mut events.iter().filter(kept).filter(|event| {
                event
                    .milestone_id
                    .as_ref()
                    .is_some_and(|m| milestones.contains(m))
            }));
            state = narrowed;
        }
        state.now = cfg.at.or(state.now);
        state
    }

    /// Folds one more event, in time order, into the board.
    pub fn apply(&mut self, event: &Event) {
        self.event_count += 1;
        self.lane_counts[lane_index(event.lane)] += 1;
        match event.overlay {
            Some(Overlay::Human) => self.human_overlays += 1,
            Some(Overlay::Commander) => self.commander_overlays += 1,
            None => {}
        }
        self.now = Some(self.now.map_or(event.at, |now| now.max(event.at)));
        self.gate.apply(event);

        if let EventKind::Unknown { raw_type } = &event.kind {
            self.tray_unknown += 1;
            self.tray.push(TrayEntry {
                event_id: event.id.clone(),
                raw_type: raw_type.clone(),
                pointer: event.source.clone(),
                at: event.at,
            });
        }
        if let Some(milestone) = &event.milestone_id {
            match self.cards.iter_mut().find(|c| &c.milestone == milestone) {
                Some(card) => card.apply(event),
                None => self.cards.push(Card::start(milestone.clone(), event)),
            }
        }
        if let Some(session) = &event.session_id {
            match self.agents.iter_mut().find(|a| &a.session_id == session) {
                Some(agent) => agent.apply(event),
                None => {
                    let mut agent = AgentRow::start(session.clone(), event);
                    agent.apply(event);
                    self.agents.push(agent);
                }
            }
        }
        if event.lane == Lane::Build {
            self.apply_to_sublane(event);
        }
    }

    pub fn with_now(mut self, now: Option<Timestamp>) -> Self {
        self.now = now;
        self
    }

    fn apply_to_sublane(&mut self, event: &Event) {
        let (key, role) = match self.grouping {
            WorkerGrouping::PerWorker => (
                event.session_id.as_ref().map(|s| s.0.clone()),
                Some(event.role),
            ),
            WorkerGrouping::PerFeature => (event.feature_id.as_ref().map(|f| f.0.clone()), None),
        };
        let key = key.unwrap_or_default();
        match self.sublanes.iter_mut().find(|lane| lane.key == key) {
            Some(lane) => {
                lane.event_count += 1;
                lane.last_at = lane.last_at.max(event.at);
            }
            None => self.sublanes.push(SubLane {
                key,
                role,
                event_count: 1,
                last_at: event.at,
            }),
        }
    }
}
