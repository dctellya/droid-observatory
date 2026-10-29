use serde::{Deserialize, Serialize};

use crate::model::{
    CycleTemplate, Event, EventKind, FeatureId, LaneId, MilestoneId, ModelId, Role, SessionId,
    Timestamp,
};

/// PRD §8.1 validation state of a milestone card.
///
/// Red: a blocking finding is open or a scenario failed. Green: scrutiny passed and a scenario
/// passed with nothing red. Grey: anything else, including not validated yet.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationBadge {
    Red,
    Grey,
    Green,
}

impl ValidationBadge {
    pub const ALL: [ValidationBadge; 3] = [
        ValidationBadge::Red,
        ValidationBadge::Grey,
        ValidationBadge::Green,
    ];

    /// The wire text (`red`).
    pub fn as_str(self) -> &'static str {
        match self {
            ValidationBadge::Red => "red",
            ValidationBadge::Grey => "grey",
            ValidationBadge::Green => "green",
        }
    }

    /// The badge as the board prints it (`RED`).
    pub fn label(self) -> &'static str {
        match self {
            ValidationBadge::Red => "RED",
            ValidationBadge::Grey => "GREY",
            ValidationBadge::Green => "GREEN",
        }
    }

    pub fn parse(text: &str) -> Option<ValidationBadge> {
        ValidationBadge::ALL
            .into_iter()
            .find(|badge| badge.as_str() == text)
    }
}

/// Who holds a milestone right now: the session whose events for it came last and that has
/// not finished.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardNow {
    pub session_id: SessionId,
    pub role: Role,
    pub feature: Option<FeatureId>,
    pub model: Option<ModelId>,
    pub at: Timestamp,
}

/// Why nobody holds a milestone that is not finished (FR-A1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdleReason {
    /// A question, permission or approval of the milestone is open.
    WaitingOnHuman,
    /// A task is done and the validators have not passed it.
    WaitingOnValidation,
    /// No work has been done on it yet.
    Queued,
}

impl IdleReason {
    pub fn label(self) -> &'static str {
        match self {
            IdleReason::WaitingOnHuman => "waiting on human",
            IdleReason::WaitingOnValidation => "waiting on validation",
            IdleReason::Queued => "queued",
        }
    }
}

/// One milestone on the board (FR-B1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Card {
    pub milestone: MilestoneId,
    /// Lane of the most recent open activity.
    pub lane: LaneId,
    pub event_count: u64,
    pub last_at: Timestamp,
    /// In order of first appearance.
    pub features: Vec<FeatureId>,
    pub validation: ValidationBadge,
    /// Fix cycles the card has been through: each red check that found it done sends it back to
    /// the lane its workers belong to (FR-B4, FR-G7).
    #[serde(default)]
    pub cycle: u32,
    pub now: Option<CardNow>,
    /// Set while `now` is empty and the milestone is not finished.
    pub idle: Option<IdleReason>,
    #[serde(skip)]
    track: Track,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Track {
    lanes: LaneTracker,
    blocking_open: bool,
    scenario_failed: bool,
    scrutiny_passed: bool,
    scenario_passed: bool,
    human_open: bool,
    task_done: bool,
    /// A red check sent the card back and no green one has come since.
    fixing: bool,
    /// Least recent first.
    holders: Vec<CardNow>,
}

/// Lane of the last open event, else of the last event.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct LaneTracker {
    open: Option<LaneId>,
    last: Option<LaneId>,
}

impl LaneTracker {
    fn apply(&mut self, event: &Event, template: &CycleTemplate) {
        if is_open(event, template) {
            self.open = Some(event.lane.clone());
        }
        self.last = Some(event.lane.clone());
    }

    fn move_to(&mut self, lane: &LaneId) {
        self.open = Some(lane.clone());
        self.last = Some(lane.clone());
    }

    fn lane(&self, template: &CycleTemplate) -> LaneId {
        self.open
            .clone()
            .or_else(|| self.last.clone())
            .unwrap_or_else(|| template.first().clone())
    }
}

/// Every lane but the final one is open unless the kind ends `.done`, `.passed`, `.merged` or
/// `.finished`; in the final lane only the kinds that wait for someone are.
fn is_open(event: &Event, template: &CycleTemplate) -> bool {
    if event.lane == *template.final_lane() {
        return matches!(
            event.kind,
            EventKind::PrReviewRequested
                | EventKind::CheckStarted
                | EventKind::CheckFailed
                | EventKind::CheckRequiredMissing
                | EventKind::RulesetBlocked
        );
    }
    let name = event.kind.prd_name();
    ![".done", ".passed", ".merged", ".finished"]
        .iter()
        .any(|suffix| name.ends_with(suffix))
}

/// The lane a milestone's card sits in, from the events of that milestone in time order. A
/// milestone with no events waits in the template's first lane.
pub fn card_lane(events_of_milestone: &[Event], template: &CycleTemplate) -> LaneId {
    let mut tracker = LaneTracker::default();
    for event in events_of_milestone
        .iter()
        .filter(|event| !event.kind.is_panel_only())
    {
        tracker.apply(event, template);
    }
    tracker.lane(template)
}

impl Card {
    pub(super) fn start(milestone: MilestoneId, first: &Event, template: &CycleTemplate) -> Self {
        let mut card = Self {
            milestone,
            lane: template.first().clone(),
            event_count: 0,
            last_at: first.at,
            features: Vec::new(),
            validation: ValidationBadge::Grey,
            cycle: 0,
            now: None,
            idle: None,
            track: Track::default(),
        };
        card.apply(first, template);
        card
    }

    pub(super) fn apply(&mut self, event: &Event, template: &CycleTemplate) {
        self.event_count += 1;
        self.last_at = self.last_at.max(event.at);
        if let Some(feature) = &event.feature_id
            && !self.features.contains(feature)
        {
            self.features.push(feature.clone());
        }
        if !self.track_fix_cycle(&event.kind, template) {
            self.track.lanes.apply(event, template);
        }
        self.lane = self.track.lanes.lane(template);
        self.track_validation(&event.kind);
        self.validation = self.track.badge();
        self.track_holder(event);
        self.now = self.track.holders.last().cloned();
        self.idle = self.now.is_none().then(|| self.track.idle()).flatten();
    }

    /// A red check returns the card to the build lane, once per cycle, and the green one that
    /// follows puts it back at the gate. True when the event placed the card itself.
    fn track_fix_cycle(&mut self, kind: &EventKind, template: &CycleTemplate) -> bool {
        let track = &mut self.track;
        match kind {
            EventKind::CheckFailed => {
                if !track.fixing {
                    track.fixing = true;
                    self.cycle += 1;
                    track.lanes.move_to(template.home(Role::Worker));
                }
                true
            }
            EventKind::CheckPassed if track.fixing => {
                track.fixing = false;
                track.lanes.move_to(template.final_lane());
                true
            }
            _ => false,
        }
    }

    fn track_validation(&mut self, kind: &EventKind) {
        let track = &mut self.track;
        match kind {
            EventKind::FindingBlocking => {
                track.blocking_open = true;
                track.scrutiny_passed = false;
            }
            EventKind::ReworkResolved => track.blocking_open = false,
            EventKind::ScrutinyPassed => {
                track.blocking_open = false;
                track.scrutiny_passed = true;
            }
            EventKind::QaScenarioFailed => {
                track.scenario_failed = true;
                track.scenario_passed = false;
            }
            EventKind::QaScenarioPassed => {
                track.scenario_failed = false;
                track.scenario_passed = true;
            }
            EventKind::QuestionAsked
            | EventKind::PermissionRequested
            | EventKind::ApprovalRequested => track.human_open = true,
            EventKind::QuestionAnswered
            | EventKind::IntentAnswered
            | EventKind::PermissionGranted
            | EventKind::PermissionDenied
            | EventKind::ApprovalGiven => track.human_open = false,
            EventKind::TaskDone => track.task_done = true,
            _ => {}
        }
    }

    fn track_holder(&mut self, event: &Event) {
        let Some(session) = &event.session_id else {
            return;
        };
        if !matches!(
            event.role,
            Role::Worker | Role::ScrutinyValidator | Role::UserTestingValidator
        ) {
            return;
        }
        let holders = &mut self.track.holders;
        let held = holders.iter().any(|holder| &holder.session_id == session);
        // Usage, model and compaction reports are bookkeeping: they never take hold of a card.
        if !held
            && matches!(
                event.kind,
                EventKind::UsageReported | EventKind::ModelChanged | EventKind::ContextCompacted
            )
        {
            return;
        }
        holders.retain(|holder| &holder.session_id != session);
        if !matches!(
            event.kind,
            EventKind::SubagentFinished | EventKind::TaskDone
        ) {
            holders.push(CardNow {
                session_id: session.clone(),
                role: event.role,
                feature: event.feature_id.clone(),
                model: event.model.clone(),
                at: event.at,
            });
        }
    }
}

impl Track {
    fn idle(&self) -> Option<IdleReason> {
        if self.badge() == ValidationBadge::Green {
            None
        } else if self.human_open {
            Some(IdleReason::WaitingOnHuman)
        } else if self.task_done {
            Some(IdleReason::WaitingOnValidation)
        } else {
            Some(IdleReason::Queued)
        }
    }

    fn badge(&self) -> ValidationBadge {
        if self.blocking_open || self.scenario_failed {
            ValidationBadge::Red
        } else if self.scrutiny_passed && self.scenario_passed {
            ValidationBadge::Green
        } else {
            ValidationBadge::Grey
        }
    }
}
