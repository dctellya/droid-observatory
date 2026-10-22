//! Mapping rules for Droid 0.247 session files. The skeleton knows none of them yet: every
//! record becomes `Unknown`, so the unknown count equals the line count.

use serde_json::Value;

use super::MapContext;
use crate::ingest::RawRecord;
use crate::model::{Event, EventId, EventKind, Lane, SourcePtr, Timestamp};

/// `raw_type` of a line that is not a JSON object with a string `type`.
const UNPARSEABLE: &str = "<unparseable>";

pub(super) fn map_record(record: &RawRecord, ctx: &MapContext<'_>) -> Vec<Event> {
    let line = serde_json::from_slice::<Value>(&record.bytes).ok();
    let raw_type = line
        .as_ref()
        .and_then(|line| line.get("type"))
        .and_then(Value::as_str)
        .unwrap_or(UNPARSEABLE);
    let at = line
        .as_ref()
        .and_then(|line| line.get("timestamp"))
        .and_then(Value::as_str)
        .and_then(|text| Timestamp::parse(text).ok())
        .unwrap_or(ctx.fallback_at);
    let session = ctx.session;

    vec![Event {
        id: EventId(format!("{}:{}:0", session.id, byte_offset(&record.ptr))),
        mission_id: session.mission_id.clone(),
        milestone_id: None,
        feature_id: None,
        lane: Lane::for_role(session.role),
        kind: EventKind::Unknown {
            raw_type: raw_type.to_owned(),
        },
        role: session.role,
        session_id: Some(session.id.clone()),
        model: session.model.clone(),
        at,
        duration: None,
        usage: None,
        source: record.ptr.clone(),
        mapping_version: ctx.version,
    }]
}

fn byte_offset(ptr: &SourcePtr) -> u64 {
    match ptr {
        SourcePtr::Jsonl { byte_offset, .. } | SourcePtr::Rpc { byte_offset, .. } => *byte_offset,
        SourcePtr::Git { .. } | SourcePtr::GitHub { .. } | SourcePtr::App { .. } => 0,
    }
}
