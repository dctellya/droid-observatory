//! The two line shapes of Droid's CLI log the usage source reads (v1 heuristics, D-029).
//! Everything else in the log is `Other` and is not kept.

use serde_json::Value;

use crate::model::{ModelId, SessionId, Timestamp};

const CONTEXT: &str = " | Context: ";
const STREAMING_RESULT: &str = "[Agent] Streaming result";

/// What one log line is, as far as the usage source cares.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LogLine {
    /// `[<ts>] INFO: [Agent] Streaming result | Context: {...}`: one model turn.
    Turn(StreamingResult),
    /// A line whose Context JSON has a top-level `sessionId` (for example
    /// `[metrics_log_droid_chat_client_success_count]`): the turns right after it, in the same
    /// second, belong to that session.
    Session(SessionLine),
    Other,
}

/// The token counts of a Streaming result line. A count the line lacks, or a Context that is
/// not JSON, reads as 0: the turn is still counted (rule 18).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamingResult {
    pub log_ts: Timestamp,
    pub input: u64,
    pub cache_read: u64,
    pub total_input: u64,
    pub reasoning: u64,
    pub output: u64,
    /// `tags.sessionId` of the line itself (Droid 0.247 writes it): only a cross-check of
    /// the join, never the join.
    pub own_session: Option<SessionId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionLine {
    pub log_ts: Timestamp,
    pub session_id: SessionId,
    pub model: Option<ModelId>,
}

pub fn parse_line(bytes: &[u8]) -> LogLine {
    let Ok(line) = std::str::from_utf8(bytes) else {
        return LogLine::Other;
    };
    let Some((stamp, after)) = line.strip_prefix('[').and_then(|rest| rest.split_once(']')) else {
        return LogLine::Other;
    };
    let Ok(log_ts) = Timestamp::parse(stamp) else {
        return LogLine::Other;
    };
    let Some((head, json)) = after.split_once(CONTEXT) else {
        return LogLine::Other;
    };
    if head.ends_with(STREAMING_RESULT) {
        let context: Value = serde_json::from_str(json).unwrap_or(Value::Null);
        let count = |key: &str| context.get(key).and_then(Value::as_u64).unwrap_or(0);
        return LogLine::Turn(StreamingResult {
            log_ts,
            input: count("inputTokens"),
            cache_read: count("cacheReadInputTokens"),
            total_input: count("totalInputTokens"),
            reasoning: count("reasoningTokens"),
            output: count("outputTokens"),
            own_session: context
                .pointer("/tags/sessionId")
                .and_then(Value::as_str)
                .map(|id| SessionId(id.to_owned())),
        });
    }
    // Most lines have no top-level sessionId; skip the parse unless the key is there at all.
    if !json.contains("\"sessionId\"") {
        return LogLine::Other;
    }
    let Ok(context) = serde_json::from_str::<Value>(json) else {
        return LogLine::Other;
    };
    match context.get("sessionId").and_then(Value::as_str) {
        Some(id) if !id.is_empty() => LogLine::Session(SessionLine {
            log_ts,
            session_id: SessionId(id.to_owned()),
            model: context
                .get("modelId")
                .and_then(Value::as_str)
                .map(|model| ModelId(model.to_owned())),
        }),
        _ => LogLine::Other,
    }
}
