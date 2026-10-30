//! The six time states of PRD §10.1: every instant of a feature is in exactly one.

use std::collections::{BTreeSet, HashSet, VecDeque};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::span::{Span, event_span, gap, timeline};
use crate::model::{Event, EventKind, Role, Timestamp};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WaitKind {
    Question,
    Permission,
    Merge,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeState {
    Active,
    WaitingHuman(WaitKind),
    WaitingValidation,
    RetryFailure,
    IdleQueued,
    IdleUnscheduled,
}

impl TimeState {
    /// When claims overlap, the highest wins: a failure inside an active span is the failure,
    /// and a wait with no active span under it is the wait (architecture §9).
    fn priority(self) -> u8 {
        match self {
            Self::RetryFailure => 5,
            Self::Active => 4,
            Self::WaitingHuman(_) => 3,
            Self::WaitingValidation => 2,
            Self::IdleQueued | Self::IdleUnscheduled => 1,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Interval {
    pub from: Timestamp,
    pub to: Timestamp,
    pub state: TimeState,
}

impl Interval {
    pub fn length(&self) -> Duration {
        gap(self.from, self.to)
    }
}

/// What lies outside the feature and bears on its idle time.
#[derive(Clone, Copy, Debug, Default)]
pub struct PartitionContext<'a> {
    /// Spans in which another feature is active: a gap in one of them is queued, not unscheduled.
    pub others_active: &'a [Span],
    /// Spans in which the platform stopped the work. They are left out of every state, Active
    /// included (a subagent's whole-run span stays one span across a stop), so the partition
    /// has a gap there (F-033: not idle, not waiting on a human).
    pub throttled: &'a [Span],
}

struct Claim {
    span: Span,
    state: TimeState,
}

/// The spans in which the events say the feature worked (the Active claims), for the others'
/// `PartitionContext::others_active`.
pub fn active_spans(events: &[Event]) -> Vec<Span> {
    let line = timeline(events);
    let turns = turn_sessions(&line);
    line.into_iter()
        .filter(|event| !is_failure(&event.kind) && measures_work(event, &turns))
        .filter_map(event_span)
        .collect()
}

/// The sessions whose turns the events measured (`task.done`, `task.skipped`).
fn turn_sessions<'a>(line: &[&'a Event]) -> HashSet<Option<&'a str>> {
    line.iter()
        .filter(|event| {
            matches!(event.kind, EventKind::TaskDone | EventKind::TaskSkipped)
                && event.duration.is_some()
        })
        .map(|event| event.session_id.as_ref().map(|id| id.0.as_str()))
        .collect()
}

/// A subagent's `duration` is its whole file span, across any pause: it measures work only for a
/// session none of whose turns was measured (a turn still open when the capture ends).
fn measures_work(event: &Event, turns: &HashSet<Option<&str>>) -> bool {
    event.kind != EventKind::SubagentFinished
        || !turns.contains(&event.session_id.as_ref().map(|id| id.0.as_str()))
}

fn is_failure(kind: &EventKind) -> bool {
    matches!(kind, EventKind::ToolError | EventKind::HookBlocked)
}

/// `events_of_feature` cut into contiguous, non-overlapping intervals from the first span to
/// `end`, each in one state (`time_states_partition_every_instant_exactly_once`).
pub fn partition(events_of_feature: &[Event], end: Timestamp) -> Vec<Interval> {
    partition_in(events_of_feature, end, &PartitionContext::default())
}

pub fn partition_in(
    events_of_feature: &[Event],
    end: Timestamp,
    context: &PartitionContext<'_>,
) -> Vec<Interval> {
    partition_with_human_waits(events_of_feature, end, context, &[])
}

/// `partition_in` with the spans in which a question no feature owns was open
/// (`mission_question_waits`): the mission waits on the human, so a feature that is not active in
/// one is waiting on the question (PRD §10.1). A span is clipped to the feature's window.
pub fn partition_with_human_waits(
    events_of_feature: &[Event],
    end: Timestamp,
    context: &PartitionContext<'_>,
    human_waits: &[Span],
) -> Vec<Interval> {
    let line = timeline(events_of_feature);
    let Some(first) = line.first() else {
        return Vec::new();
    };
    let mut claims = measured_claims(&line);
    claims.extend(waiting_claims(&line, end));
    let start = claims
        .iter()
        .map(|claim| claim.span.from)
        .chain([first.at])
        .min()
        .unwrap_or(first.at);
    if end <= start {
        return Vec::new();
    }
    claims.extend(human_waits.iter().filter_map(|wait| {
        let (from, to) = (wait.from.max(start), wait.to.min(end));
        (from < to).then_some(Claim {
            span: Span { from, to },
            state: TimeState::WaitingHuman(WaitKind::Question),
        })
    }));

    let mut cuts: BTreeSet<Timestamp> = BTreeSet::from([start, end]);
    let boundaries = claims
        .iter()
        .map(|claim| claim.span)
        .chain(context.others_active.iter().copied())
        .chain(context.throttled.iter().copied());
    for span in boundaries {
        cuts.extend(
            [span.from, span.to]
                .into_iter()
                .filter(|at| *at > start && *at < end),
        );
    }
    let cuts: Vec<Timestamp> = cuts.into_iter().collect();

    let mut intervals: Vec<Interval> = Vec::new();
    for pair in cuts.windows(2) {
        let (from, to) = (pair[0], pair[1]);
        let claimed = claims
            .iter()
            .filter(|claim| claim.span.covers(from, to))
            .map(|claim| claim.state)
            .max_by_key(|state| state.priority());
        let state = claimed.unwrap_or_else(|| {
            if context
                .others_active
                .iter()
                .any(|span| span.covers(from, to))
            {
                TimeState::IdleQueued
            } else {
                TimeState::IdleUnscheduled
            }
        });
        if context.throttled.iter().any(|span| span.covers(from, to)) {
            continue;
        }
        match intervals.last_mut() {
            Some(last) if last.state == state && last.to == from => last.to = to,
            _ => intervals.push(Interval { from, to, state }),
        }
    }
    intervals
}

/// The spans in which a question of `mission_level` events (those no feature owns) was open: to
/// its answer, or to the error result of the `AskUser` call the platform refused, which starts
/// at the question and carries a duration; an open one runs to `end`.
pub fn mission_question_waits(mission_level: &[Event], end: Timestamp) -> Vec<Span> {
    let mut open: VecDeque<Timestamp> = VecDeque::new();
    let mut waits = Vec::new();
    for event in timeline(mission_level) {
        match event.kind {
            EventKind::QuestionAsked => open.push_back(event.at),
            EventKind::ToolError => {
                let refused = event_span(event)
                    .and_then(|span| open.iter().position(|from| *from == span.from));
                if let Some(index) = refused
                    && let Some(from) = open.remove(index)
                {
                    waits.push(Span { from, to: event.at });
                }
            }
            EventKind::QuestionAnswered | EventKind::IntentAnswered => {
                if let Some(from) = open.pop_front() {
                    waits.push(Span { from, to: event.at });
                }
            }
            _ => {}
        }
    }
    waits.extend(open.into_iter().map(|from| Span { from, to: end }));
    waits
}

/// Active and failure spans: what the events measured.
fn measured_claims(line: &[&Event]) -> Vec<Claim> {
    let turns = turn_sessions(line);
    line.iter()
        .filter(|event| measures_work(event, &turns))
        .filter_map(|event| {
            let state = if is_failure(&event.kind) {
                TimeState::RetryFailure
            } else {
                TimeState::Active
            };
            event_span(event).map(|span| Claim { span, state })
        })
        .collect()
}

/// Waits that the events open and close: a question or a permission request until its answer,
/// a green gate until the hand-over, a finished task or a resolved finding until a validator
/// speaks. An open wait runs to `end`.
fn waiting_claims(line: &[&Event], end: Timestamp) -> Vec<Claim> {
    let mut claims = Vec::new();
    let mut question: VecDeque<Timestamp> = VecDeque::new();
    let mut permission: VecDeque<Timestamp> = VecDeque::new();
    let mut merge: Option<Timestamp> = None;
    let mut validation: Option<Timestamp> = None;
    let push = |claims: &mut Vec<Claim>, state, from, to| {
        claims.push(Claim {
            span: Span { from, to },
            state,
        })
    };

    for event in line {
        if event.role.is_validator()
            && let Some(from) = validation.take()
        {
            push(&mut claims, TimeState::WaitingValidation, from, event.at);
        }
        match event.kind {
            EventKind::QuestionAsked => question.push_back(event.at),
            EventKind::QuestionAnswered | EventKind::IntentAnswered => {
                if let Some(from) = question.pop_front() {
                    push(
                        &mut claims,
                        TimeState::WaitingHuman(WaitKind::Question),
                        from,
                        event.at,
                    );
                }
            }
            EventKind::PermissionRequested | EventKind::ApprovalRequested => {
                permission.push_back(event.at);
            }
            EventKind::PermissionGranted
            | EventKind::PermissionDenied
            | EventKind::ApprovalGiven => {
                if let Some(from) = permission.pop_front() {
                    push(
                        &mut claims,
                        TimeState::WaitingHuman(WaitKind::Permission),
                        from,
                        event.at,
                    );
                }
            }
            EventKind::CheckPassed => {
                merge.get_or_insert(event.at);
            }
            EventKind::CheckFailed | EventKind::HandedToCd { .. } => {
                if let Some(from) = merge.take() {
                    push(
                        &mut claims,
                        TimeState::WaitingHuman(WaitKind::Merge),
                        from,
                        event.at,
                    );
                }
            }
            EventKind::TaskDone | EventKind::ReworkResolved => {
                validation.get_or_insert(event.at);
            }
            // The runner cancels a worker after its `EndFeatureRun`, so a finished runner worker
            // ends in `task.skipped` (D-023); a validator's skipped turn finishes nothing.
            EventKind::TaskSkipped if event.role == Role::Worker => {
                validation.get_or_insert(event.at);
            }
            _ => {}
        }
    }
    for (state, from) in question
        .into_iter()
        .map(|from| (TimeState::WaitingHuman(WaitKind::Question), from))
        .chain(
            permission
                .into_iter()
                .map(|from| (TimeState::WaitingHuman(WaitKind::Permission), from)),
        )
        .chain(merge.map(|from| (TimeState::WaitingHuman(WaitKind::Merge), from)))
        .chain(validation.map(|from| (TimeState::WaitingValidation, from)))
    {
        push(&mut claims, state, from, end);
    }
    claims
}
