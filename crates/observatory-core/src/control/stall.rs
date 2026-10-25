//! Stall detection (FR-H5, FR-H6): a feature with no active time for the configured threshold
//! raises a flag that names the runbook prompt matching what it last did.

use std::time::Duration;

use crate::config::InterventionConfig;
use crate::model::{Event, EventKind, FeatureId, SourcePtr, Timestamp};

const SECONDS_PER_MINUTE: u64 = 60;

/// The threshold, from `[intervention].stall_threshold_min`.
pub fn threshold(config: &InterventionConfig) -> Duration {
    Duration::from_secs(
        config
            .stall_threshold_min
            .saturating_mul(SECONDS_PER_MINUTE),
    )
}

/// What a feature last did, as far as stall detection needs.
#[derive(Clone, Debug, PartialEq)]
pub struct FeatureActivity {
    pub feature_id: FeatureId,
    pub last_active_at: Timestamp,
    pub last_kind: EventKind,
    /// The kind before the last one was the same `hook.blocked`.
    pub repeated: bool,
    /// A spawned worker's session file still grows.
    pub child_growing: bool,
}

/// A prompt of `docs/runbooks/stalled-mission.md` and the anchor of its section.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RunbookPrompt {
    pub prompt: &'static str,
    pub anchor: &'static str,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StallFlag {
    pub feature_id: FeatureId,
    pub idle_for: Duration,
    pub runbook: RunbookPrompt,
}

const WAITING_ON_HUMAN: RunbookPrompt = RunbookPrompt {
    prompt: "Waiting on human: answer it",
    anchor: "#waiting-on-human",
};
const WORKER_SILENT: RunbookPrompt = RunbookPrompt {
    prompt: "Worker silent: check the child file, retry or skip",
    anchor: "#worker-silent",
};
const HOOK_BLOCKED: RunbookPrompt = RunbookPrompt {
    prompt: "Agent hits the write guard: forward-only instruction",
    anchor: "#hook-blocked",
};
const INTEGRATION_CONFLICT: RunbookPrompt = RunbookPrompt {
    prompt: "Two worktrees touched one file",
    anchor: "#integration-conflict",
};
const LONG_TOOL_CALL: RunbookPrompt = RunbookPrompt {
    prompt: "Long tool call: wait one more threshold, then pause and inspect",
    anchor: "#long-tool-call",
};

impl FeatureActivity {
    fn runbook(&self) -> RunbookPrompt {
        match self.last_kind {
            EventKind::QuestionAsked | EventKind::PermissionRequested => WAITING_ON_HUMAN,
            EventKind::SubagentSpawned if !self.child_growing => WORKER_SILENT,
            EventKind::HookBlocked if self.repeated => HOOK_BLOCKED,
            EventKind::IntegrationConflict => INTEGRATION_CONFLICT,
            _ => LONG_TOOL_CALL,
        }
    }
}

/// The features idle for at least `threshold` at `now`; nothing under it is reported.
pub fn evaluate(
    features: &[FeatureActivity],
    now: Timestamp,
    threshold: Duration,
) -> Vec<StallFlag> {
    features
        .iter()
        .filter_map(|feature| {
            let idle_for = Duration::try_from(now.0 - feature.last_active_at.0).unwrap_or_default();
            (idle_for >= threshold).then(|| StallFlag {
                feature_id: feature.feature_id.clone(),
                idle_for,
                runbook: feature.runbook(),
            })
        })
        .collect()
}

/// A feature is finished when its last event closes it: the task done or skipped, or a person
/// marking it complete or skipped (an app event that names the feature). A finished feature
/// is not idle, it is done.
fn closes_feature(event: &Event) -> bool {
    matches!(event.kind, EventKind::TaskDone | EventKind::TaskSkipped)
        || matches!(event.source, SourcePtr::App { .. })
}

/// The name a wait of the mission itself is flagged under: the orchestrator asks a question
/// that names no feature.
pub const MISSION_LEVEL: &str = "mission";

/// The activity of each unfinished feature of `events` (in time order): its last event. The
/// child of a `subagent.spawned` that grows has events of its own, so the last event of a
/// feature whose worker is alive is never the spawn. A question or permission request that
/// names no feature and has no later event of the mission is a wait of the mission itself.
pub fn activity_of(events: &[Event]) -> Vec<FeatureActivity> {
    let mut activity = feature_activity(events);
    let waiting = events
        .iter()
        .filter(|event| !event.kind.is_panel_only())
        .max_by_key(|event| event.at)
        .filter(|last| {
            last.feature_id.is_none()
                && matches!(
                    last.kind,
                    EventKind::QuestionAsked | EventKind::PermissionRequested
                )
        });
    if let Some(last) = waiting {
        activity.push(FeatureActivity {
            feature_id: FeatureId(MISSION_LEVEL.to_owned()),
            last_active_at: last.at,
            last_kind: last.kind.clone(),
            repeated: false,
            child_growing: false,
        });
    }
    activity
}

fn feature_activity(events: &[Event]) -> Vec<FeatureActivity> {
    let mut features: Vec<(&FeatureId, Vec<&Event>)> = Vec::new();
    for event in events.iter().filter(|event| !event.kind.is_panel_only()) {
        let Some(feature) = &event.feature_id else {
            continue;
        };
        match features.iter_mut().find(|(known, _)| *known == feature) {
            Some((_, seen)) => seen.push(event),
            None => features.push((feature, vec![event])),
        }
    }
    features
        .into_iter()
        .filter_map(|(feature, seen)| {
            let last = seen.iter().copied().max_by_key(|event| event.at)?;
            if closes_feature(last) {
                return None;
            }
            let repeated = last.kind == EventKind::HookBlocked
                && seen
                    .iter()
                    .rev()
                    .skip(1)
                    .take(1)
                    .any(|before| before.kind == EventKind::HookBlocked);
            Some(FeatureActivity {
                feature_id: feature.clone(),
                last_active_at: last.at,
                last_kind: last.kind.clone(),
                repeated,
                child_growing: false,
            })
        })
        .collect()
}
