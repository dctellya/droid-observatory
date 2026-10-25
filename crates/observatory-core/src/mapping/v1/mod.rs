//! Mapping rules for Droid 0.247 session files. The rule table is in `kinds.rs`.
//!
//! # `turn.usage` and the usage-log heuristics (F-034, D-029)
//!
//! `turn.usage` is the one kind not read from a session file. It comes from Droid's CLI log
//! (`droid-log-single.log` and its dated rotations), whose lines the session files cannot
//! replace; `ingest::usage_log` parses them and this module only places a parsed turn.
//!
//! - Line format: `[<RFC 3339>] INFO: [Agent] Streaming result | Context: {json}`; the Context
//!   holds `inputTokens`, `cacheReadInputTokens`, `totalInputTokens`, `reasoningTokens` and
//!   `outputTokens`. The payload is `input`, `output`, `cache_read`, `reasoning`, `total_input`,
//!   `model` and `log_ts`.
//! - Join: the session and model of a turn come from the nearest preceding line whose Context
//!   has a top-level `sessionId` and that has the same second as the turn. The turn line's own
//!   `tags.sessionId` is a cross-check; a difference is counted as a join mismatch and shown.
//!   A turn with no such line is an unknown turn: counted and shown, never dropped (rule 18).
//! - Lane and role: those of the session the turn joined to, in the phase the mapper holds for
//!   that session when the turn is mapped (the rule of the other role-neutral events,
//!   architecture §5). The event is timed by `log_ts`; its id is
//!   `<session>:<byte offset in the log>:<2^31>` so it cannot meet the id of a session line.
//! - The log is a heuristic source, not a documented format: a Droid version that changes the
//!   line text yields unknown turns, not a wrong join. `usage_reported`, derived from the
//!   session sidecar, stays beside it.

mod findings;
mod ids;
mod kinds;
mod roles;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;

pub(crate) use ids::proposal_milestones;
pub use roles::RoleEvidence;
pub(in crate::mapping) use roles::is_contract_task;
pub(in crate::mapping) use roles::resolve_role;

use super::{MappingVersion, SessionMeta};
use crate::ingest::{RawRecord, UsageTurn};
use crate::model::{
    CycleTemplate, Event, EventId, EventKind, FeatureId, LaneId, MilestoneId, ModelId, Role,
    SourcePtr, Timestamp,
};
use kinds::{Outcome, Tool};

/// `raw_type` of a line that is not a JSON object with a string `type`.
const UNPARSEABLE: &str = "<unparseable>";
/// `userMessageSource` of text the person typed in the root session.
const TYPED_BY_HUMAN: &str = "cli_tui";

/// What the rules remember of one session between its lines.
pub(super) struct State {
    template: Arc<CycleTemplate>,
    first_at: Option<Timestamp>,
    last_at: Option<Timestamp>,
    /// Time of the first timestamped line since the last `agent_turn_outcome`.
    turn_at: Option<Timestamp>,
    /// `modelId` of the last assistant message.
    model: Option<ModelId>,
    todos: Option<Value>,
    calls: HashMap<String, Call>,
    /// Events of lines without a time (`session_start`), waiting for the next timestamped line.
    pending: Vec<Event>,
    feature: Option<FeatureId>,
    milestones: Vec<MilestoneId>,
    defined_features: HashSet<String>,
    task_started: bool,
    spawned: bool,
    last_ptr: Option<SourcePtr>,
    /// Events the last line produced, so the end-of-file events continue its `sub` numbering.
    last_subs: u32,
}

#[derive(Clone, Copy)]
struct Call {
    tool: CallKind,
    at: Timestamp,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CallKind {
    AskUser,
    Gate,
}

impl State {
    pub(super) fn new(session: &SessionMeta, template: Arc<CycleTemplate>) -> Self {
        Self {
            template,
            first_at: None,
            last_at: None,
            turn_at: None,
            model: None,
            todos: None,
            calls: HashMap::new(),
            pending: Vec::new(),
            feature: None,
            milestones: session.milestones.clone(),
            defined_features: HashSet::new(),
            task_started: false,
            spawned: false,
            last_ptr: None,
            last_subs: 0,
        }
    }

    /// The time a line without one gets when it is the first line of its session.
    pub(super) fn set_last_at(&mut self, at: Timestamp) {
        self.last_at = Some(at);
    }

    fn milestone(&self) -> Option<MilestoneId> {
        let feature = self.feature.as_ref()?;
        ids::milestone_of(&feature.0, &self.milestones)
    }

    fn is_child(&self, session: &SessionMeta) -> bool {
        self.spawned || session.role != Role::Orchestrator
    }
}

/// The events of one line under construction.
struct Rec<'a> {
    template: &'a CycleTemplate,
    session: &'a SessionMeta,
    version: MappingVersion,
    ptr: &'a SourcePtr,
    offset: u64,
    sub_base: u32,
    at: Timestamp,
    model: Option<ModelId>,
    feature: Option<FeatureId>,
    milestone: Option<MilestoneId>,
    events: Vec<Event>,
}

impl Rec<'_> {
    fn push(&mut self, kind: EventKind) -> &mut Event {
        let sub = self.sub_base + u32::try_from(self.events.len()).unwrap_or(u32::MAX);
        let lane = lane_of(self.template, &kind, self.session, self.feature.as_ref());
        let overlay = kind.overlay();
        self.events.push(Event {
            id: EventId(format!("{}:{}:{sub}", self.session.id, self.offset)),
            mission_id: self.session.mission_id.clone(),
            milestone_id: self.milestone.clone(),
            feature_id: self.feature.clone(),
            lane,
            overlay,
            kind,
            role: self.session.role,
            session_id: Some(self.session.id.clone()),
            model: self.model.clone(),
            at: self.at,
            duration: None,
            usage: None,
            source: self.ptr.clone(),
            mapping_version: self.version,
        });
        self.events.last_mut().expect("an event was just pushed")
    }

    fn unknown(&mut self, raw_type: impl Into<String>) {
        self.push(EventKind::Unknown {
            raw_type: raw_type.into(),
        });
    }

    /// An event that names its own feature (a `Task` call) rather than the session's.
    fn push_for(&mut self, kind: EventKind, feature: Option<&str>, milestone: Option<MilestoneId>) {
        let event = self.push(kind);
        event.feature_id = feature.map(|id| FeatureId(id.to_owned()));
        event.milestone_id = milestone;
    }
}

/// The lane of an event (D-024). The kind's own lane wins; role-neutral kinds of an
/// integration or contract session take that session's lane; the rest take the role's.
/// `feature` is the session's feature, not the one a `Task` call names.
///
/// A kind the template lists in `kind_follows_emitter` (`feature.defined`) follows its emitter
/// when that is a validator (D-032): a validator that names the features it reviews is working
/// in its own lane, not planning.
fn lane_of(
    template: &CycleTemplate,
    kind: &EventKind,
    session: &SessionMeta,
    feature: Option<&FeatureId>,
) -> LaneId {
    if template.follows_emitter(kind) && session.role.is_validator() {
        return template.home(session.role).clone();
    }
    if template.fixed_lane(kind).is_some() || matches!(kind, EventKind::Unknown { .. }) {
        return template.lane_of(kind, session.role);
    }
    if feature.is_some_and(|feature| roles::is_integration_feature(&feature.0)) {
        return template.rules.integration_session.clone();
    }
    if session.contract {
        return template.rules.contract_session.clone();
    }
    template.lane_of(kind, session.role)
}

pub(super) fn map_record(
    record: &RawRecord,
    session: &SessionMeta,
    version: MappingVersion,
    state: &mut State,
) -> Vec<Event> {
    let line = serde_json::from_slice::<Value>(&record.bytes).ok();
    let stamp = line
        .as_ref()
        .and_then(|line| line.get("timestamp"))
        .and_then(Value::as_str)
        .and_then(|text| Timestamp::parse(text).ok());
    if let Some(stamp) = stamp {
        state.first_at.get_or_insert(stamp);
        state.turn_at.get_or_insert(stamp);
        state.last_at = Some(stamp);
    }
    let raw_type = line
        .as_ref()
        .and_then(|line| line.get("type"))
        .and_then(Value::as_str);
    // A line without a time of its own takes the time of the line before it. `session_start`,
    // the first line, has none before it: it waits for the next timestamped line instead.
    let waits = stamp.is_none() && raw_type == Some("session_start");
    let at = state
        .last_at
        .unwrap_or(Timestamp(time::OffsetDateTime::UNIX_EPOCH));
    let template = state.template.clone();
    let mut rec = Rec {
        template: &template,
        session,
        version,
        ptr: &record.ptr,
        offset: byte_offset(&record.ptr),
        sub_base: 0,
        at,
        model: state.model.clone().or_else(|| session.model.clone()),
        feature: state.feature.clone(),
        milestone: state.milestone(),
        events: Vec::new(),
    };

    match (line.as_ref(), raw_type) {
        (Some(line), Some("session_start")) => session_start(&mut rec, state, line),
        (Some(line), Some("message")) => message(&mut rec, state, line),
        (Some(line), Some("todo_state")) => {
            if state.todos.as_ref() != line.get("todos") {
                state.todos = line.get("todos").cloned();
                rec.push(EventKind::PlanRevised);
            }
        }
        (Some(_), Some("compaction_state")) => {
            rec.push(EventKind::ContextCompacted);
        }
        (Some(line), Some("agent_turn_outcome")) => outcome(&mut rec, state, line),
        (_, other) => rec.unknown(other.unwrap_or(UNPARSEABLE)),
    }

    state.last_ptr = Some(record.ptr.clone());
    state.last_subs = u32::try_from(rec.events.len()).unwrap_or(u32::MAX);
    let mut events = rec.events;
    if waits {
        state.pending.append(&mut events);
        return events;
    }
    match stamp {
        Some(stamp) if !state.pending.is_empty() => {
            let mut released = release_pending(state, session, stamp);
            released.append(&mut events);
            released
        }
        _ => events,
    }
}

/// The `sub` of a `turn.usage` event. Its pointer is into the usage log, whose offsets can
/// equal those of the session's own lines; a `sub` no line yields keeps the ids apart.
const TURN_USAGE_SUB: u32 = 1 << 31;

/// The `turn.usage` event of a turn of `session`: in the phase the session is in now (the same
/// rule as its other role-neutral events), timed by the log line.
pub(super) fn turn_usage(
    turn: &UsageTurn,
    session: &SessionMeta,
    version: MappingVersion,
    state: &State,
) -> Event {
    let kind = EventKind::TurnUsage {
        input: turn.input,
        output: turn.output,
        cache_read: turn.cache_read,
        reasoning: turn.reasoning,
        total_input: turn.total_input,
        model: turn.model.clone(),
        log_ts: turn.log_ts,
    };
    Event {
        id: EventId(format!(
            "{}:{}:{TURN_USAGE_SUB}",
            session.id,
            byte_offset(&turn.ptr)
        )),
        mission_id: session.mission_id.clone(),
        milestone_id: state.milestone(),
        feature_id: state.feature.clone(),
        lane: lane_of(&state.template, &kind, session, state.feature.as_ref()),
        overlay: None,
        kind,
        role: session.role,
        session_id: Some(session.id.clone()),
        model: turn
            .model
            .clone()
            .or_else(|| state.model.clone())
            .or_else(|| session.model.clone()),
        at: turn.log_ts,
        duration: None,
        usage: None,
        source: turn.ptr.clone(),
        mapping_version: version,
    }
}

/// The events of the lines that had no time, now timed by the line that followed them.
fn release_pending(state: &mut State, session: &SessionMeta, at: Timestamp) -> Vec<Event> {
    let (feature, milestone) = (state.feature.clone(), state.milestone());
    let template = state.template.clone();
    let mut events = std::mem::take(&mut state.pending);
    for event in &mut events {
        // The feature is only known once the session's first prompt has been read.
        event.lane = lane_of(&template, &event.kind, session, feature.as_ref());
        event.at = at;
        event.feature_id = feature.clone();
        event.milestone_id = milestone.clone();
    }
    events
}

/// Events still waiting for a time, timed by the last time seen (the epoch when none).
pub(super) fn flush(state: &mut State, session: &SessionMeta) -> Vec<Event> {
    let at = state
        .last_at
        .unwrap_or(Timestamp(time::OffsetDateTime::UNIX_EPOCH));
    release_pending(state, session, at)
}

/// The events that belong to the end of a session file: lines still waiting for a time, the
/// child's `SubagentFinished` and the sidecar usage. They point at the last line.
pub(super) fn finish(
    session: &SessionMeta,
    version: MappingVersion,
    state: &mut State,
) -> Vec<Event> {
    let mut events = flush(state, session);
    let at = state
        .last_at
        .unwrap_or(Timestamp(time::OffsetDateTime::UNIX_EPOCH));
    let Some(ptr) = state.last_ptr.clone() else {
        return events;
    };
    let template = state.template.clone();
    let mut rec = Rec {
        template: &template,
        session,
        version,
        ptr: &ptr,
        offset: byte_offset(&ptr),
        sub_base: state.last_subs,
        at,
        model: session.model.clone().or_else(|| state.model.clone()),
        feature: state.feature.clone(),
        milestone: state.milestone(),
        events: Vec::new(),
    };
    if let Some(usage) = session.usage {
        rec.push(EventKind::UsageReported).usage = Some(usage);
    }
    if state.is_child(session) {
        let finished = rec.push(EventKind::SubagentFinished);
        finished.duration = state.first_at.and_then(|first| between(first, at));
    }
    state.last_subs += u32::try_from(rec.events.len()).unwrap_or(u32::MAX);
    events.append(&mut rec.events);
    events
}

fn session_start(rec: &mut Rec<'_>, state: &mut State, line: &Value) {
    if line.get("callingSessionId").is_some_and(Value::is_string) {
        state.spawned = true;
        rec.push(EventKind::SubagentSpawned);
    } else {
        rec.push(EventKind::IntentCreated);
    }
}

fn message(rec: &mut Rec<'_>, state: &mut State, line: &Value) {
    let Some(message) = line.get("message").filter(|message| message.is_object()) else {
        return rec.unknown("message");
    };
    let Some(parts) = parts(&message["content"]) else {
        return rec.unknown("message");
    };
    let role = message["role"].as_str();
    if !matches!(role, Some("user" | "assistant" | "system")) {
        return rec.unknown("message");
    }
    // Droid's own annotations ("Request interrupted by user" after a pause): the pause is
    // already an event of its own, the note carries no lane meaning.
    if role == Some("system") {
        return;
    }
    if role == Some("assistant") {
        model_of(rec, state, message);
    }
    for part in &parts {
        match part {
            Part::Text(text) if role == Some("assistant") => validator_text(rec, text),
            Part::Text(text) => user_text(rec, state, message, text),
            Part::Thinking => {}
            Part::ToolUse { id, name, input } => tool_use(rec, state, id, name, input),
            Part::ToolResult {
                id,
                content,
                is_error,
            } => tool_result(rec, state, id, content, *is_error),
            Part::Other(kind) => rec.unknown(format!("message/{kind}")),
        }
    }
}

fn model_of(rec: &mut Rec<'_>, state: &mut State, message: &Value) {
    let Some(id) = message["modelId"].as_str() else {
        return;
    };
    let current = ModelId(id.to_owned());
    let switched = state
        .model
        .as_ref()
        .is_some_and(|before| *before != current);
    state.model = Some(current.clone());
    rec.model = Some(current);
    if switched {
        rec.push(EventKind::ModelChanged);
    }
}

fn validator_text(rec: &mut Rec<'_>, text: &str) {
    let kinds = match rec.session.role {
        Role::ScrutinyValidator => findings::scrutiny(text),
        Role::UserTestingValidator => findings::user_testing(text),
        _ => Vec::new(),
    };
    for kind in kinds {
        rec.push(kind);
    }
}

fn user_text(rec: &mut Rec<'_>, state: &mut State, message: &Value, text: &str) {
    if !state.is_child(rec.session) {
        if message["userMessageSource"] == TYPED_BY_HUMAN {
            rec.push(EventKind::IntentAmended);
        }
        return;
    }
    if state.task_started {
        return;
    }
    if let Some(feature) = ids::assignment(text, &state.milestones) {
        state.task_started = true;
        state.feature = feature.map(FeatureId);
        rec.feature = state.feature.clone();
        rec.milestone = state.milestone();
        rec.push(EventKind::TaskStarted);
    }
}

fn tool_use(rec: &mut Rec<'_>, state: &mut State, id: &str, name: &str, input: &Value) {
    let Some(tool) = kinds::tool(name) else {
        return rec.unknown(format!("message/tool_use/{name}"));
    };
    match tool {
        Tool::Quiet => {}
        Tool::Task => {
            let description = input["description"].as_str().unwrap_or_default();
            let feature = ids::feature_in(description, &state.milestones);
            let milestone = feature
                .as_deref()
                .and_then(|feature| ids::milestone_of(feature, &state.milestones));
            rec.push_for(
                EventKind::TaskAssigned,
                feature.as_deref(),
                milestone.clone(),
            );
            if let Some(feature) = feature.filter(|f| state.defined_features.insert(f.clone())) {
                rec.push_for(EventKind::FeatureDefined, Some(&feature), milestone);
            }
        }
        Tool::ProposeMission => {
            rec.push_for(EventKind::PlanCreated, None, None);
            let proposal = input["proposal"].as_str().unwrap_or_default();
            for milestone in ids::proposal_milestones(proposal) {
                let milestone = MilestoneId(milestone);
                if !state.milestones.contains(&milestone) {
                    state.milestones.push(milestone.clone());
                }
                rec.push_for(EventKind::MilestoneDefined, None, Some(milestone));
            }
        }
        Tool::AskUser => {
            state.calls.insert(
                id.to_owned(),
                Call {
                    tool: CallKind::AskUser,
                    at: rec.at,
                },
            );
            rec.push(EventKind::QuestionAsked);
        }
        Tool::Command => {
            let kinds = kinds::classify_command(input["command"].as_str().unwrap_or_default());
            if kinds.commit {
                rec.push(EventKind::CommitCreated);
            }
            if kinds.tests {
                rec.push(EventKind::TestsRun);
            }
            if kinds.gate {
                state.calls.insert(
                    id.to_owned(),
                    Call {
                        tool: CallKind::Gate,
                        at: rec.at,
                    },
                );
                rec.push(EventKind::GateCommandRun);
            }
        }
        Tool::Edit => {
            rec.push(EventKind::FileTouched);
        }
    }
}

fn tool_result(rec: &mut Rec<'_>, state: &mut State, id: &str, content: &Value, is_error: bool) {
    let call = state.calls.remove(id);
    let elapsed = call.and_then(|call| between(call.at, rec.at));
    let text = flatten(content);
    let gate_failed = call.map(|call| call.tool) == Some(CallKind::Gate)
        && (is_error || kinds::exit_code(&text).is_some_and(|code| code != 0));
    match (is_error, call.map(|call| call.tool)) {
        (true, tool) => {
            let kind = if kinds::names_a_hook(&text) {
                EventKind::HookBlocked
            } else if tool == Some(CallKind::Gate) {
                EventKind::GateCommandFailed
            } else {
                EventKind::ToolError
            };
            rec.push(kind).duration = elapsed;
        }
        (false, Some(CallKind::AskUser)) => {
            rec.push(EventKind::IntentAnswered);
        }
        (false, Some(CallKind::Gate)) if gate_failed => {
            rec.push(EventKind::GateCommandFailed).duration = elapsed;
        }
        (false, Some(CallKind::Gate)) => {
            rec.push(EventKind::GateCommandPassed).duration = elapsed;
        }
        (false, None) => {}
    }
}

fn outcome(rec: &mut Rec<'_>, state: &mut State, line: &Value) {
    let Some(reason) = line["reason"].as_str() else {
        return rec.unknown("agent_turn_outcome");
    };
    let Some(outcome) = kinds::outcome(reason) else {
        return rec.unknown(format!("agent_turn_outcome/{reason}"));
    };
    let elapsed = state.turn_at.and_then(|start| between(start, rec.at));
    state.turn_at = None;
    let kind = match (state.is_child(rec.session), outcome) {
        (true, Outcome::Completed) => Some(EventKind::TaskDone),
        (true, Outcome::Cancelled) => Some(EventKind::TaskSkipped),
        (true, Outcome::Failed) => Some(EventKind::ToolError),
        (false, Outcome::Completed) => None,
        (false, Outcome::Cancelled) => Some(EventKind::MissionPaused),
        (false, Outcome::Failed) => Some(EventKind::EscalationRaised),
    };
    if let Some(kind) = kind {
        rec.push(kind).duration = elapsed;
    }
}

enum Part<'a> {
    Text(&'a str),
    Thinking,
    ToolUse {
        id: &'a str,
        name: &'a str,
        input: &'a Value,
    },
    ToolResult {
        id: &'a str,
        content: &'a Value,
        is_error: bool,
    },
    /// A content part type the rules do not know.
    Other(&'a str),
}

/// `None` when `content` is neither a string nor a list.
fn parts(content: &Value) -> Option<Vec<Part<'_>>> {
    if let Some(text) = content.as_str() {
        return Some(vec![Part::Text(text)]);
    }
    let list = content.as_array()?;
    Some(
        list.iter()
            .map(|part| match part["type"].as_str() {
                Some("text") => Part::Text(part["text"].as_str().unwrap_or_default()),
                Some("thinking") => Part::Thinking,
                Some("tool_use") => Part::ToolUse {
                    id: part["id"].as_str().unwrap_or_default(),
                    name: part["name"].as_str().unwrap_or_default(),
                    input: &part["input"],
                },
                Some("tool_result") => Part::ToolResult {
                    id: part["tool_use_id"].as_str().unwrap_or_default(),
                    content: &part["content"],
                    is_error: part["is_error"].as_bool().unwrap_or(false),
                },
                Some(other) => Part::Other(other),
                None => Part::Other("?"),
            })
            .collect(),
    )
}

/// The text of a tool result, which is a string or a list of `{type: text, text}` parts.
fn flatten(content: &Value) -> String {
    match content {
        Value::String(text) => text.clone(),
        Value::Array(list) => list
            .iter()
            .filter_map(|part| part["text"].as_str())
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}

fn between(from: Timestamp, to: Timestamp) -> Option<Duration> {
    Duration::try_from(to.0 - from.0).ok()
}

fn byte_offset(ptr: &SourcePtr) -> u64 {
    match ptr {
        SourcePtr::Jsonl { byte_offset, .. }
        | SourcePtr::Rpc { byte_offset, .. }
        | SourcePtr::UsageLog { byte_offset, .. } => *byte_offset,
        SourcePtr::Git { .. } | SourcePtr::GitHub { .. } | SourcePtr::App { .. } => 0,
    }
}
