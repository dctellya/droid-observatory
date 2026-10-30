use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::mapping::MappingVersion;

use super::cycle::LaneId;
use super::ids::{EventId, FeatureId, MilestoneId, MissionId, ModelId, SessionId};
use super::kind::EventKind;
use super::overlay::Overlay;
use super::role::Role;
use super::timestamp::Timestamp;
use super::usage::TokenUsage;

/// Where the raw material of an event lives. Raw text is never copied into the store:
/// the pointer is re-read on demand.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "snake_case")]
pub enum SourcePtr {
    /// A line of a session file. `byte_offset` is the offset of the line's first byte,
    /// `byte_len` excludes the newline.
    Jsonl {
        path: PathBuf,
        byte_offset: u64,
        byte_len: u32,
    },
    /// A line of the host spool file.
    Rpc {
        path: PathBuf,
        byte_offset: u64,
        byte_len: u32,
    },
    /// A `[Agent] Streaming result` line of the Droid CLI log (the usage-log source, §4.7).
    /// Offsets are those of the file the line was read from.
    UsageLog {
        path: PathBuf,
        byte_offset: u64,
        byte_len: u32,
    },
    Git {
        repo: PathBuf,
        sha: String,
    },
    /// API URL of the object observed.
    GitHub {
        url: String,
    },
    /// Row id of an app action (`human_actions.id`).
    App {
        action_id: u64,
    },
}

/// One thing that happened in a mission (PRD appendix A).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Event {
    pub id: EventId,
    pub mission_id: MissionId,
    pub milestone_id: Option<MilestoneId>,
    pub feature_id: Option<FeatureId>,
    pub lane: LaneId,
    /// Set for events a person caused; serialised only when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overlay: Option<Overlay>,
    pub kind: EventKind,
    pub role: Role,
    pub session_id: Option<SessionId>,
    pub model: Option<ModelId>,
    pub at: Timestamp,
    #[serde(rename = "duration_ms", default, with = "duration_ms")]
    pub duration: Option<Duration>,
    pub usage: Option<TokenUsage>,
    pub source: SourcePtr,
    pub mapping_version: MappingVersion,
}

/// `Duration` on the wire is an integer number of milliseconds.
mod duration_ms {
    use std::time::Duration;

    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(
        duration: &Option<Duration>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match duration {
            Some(duration) => {
                let millis = u64::try_from(duration.as_millis()).unwrap_or(u64::MAX);
                serializer.serialize_some(&millis)
            }
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<Duration>, D::Error> {
        Ok(Option::<u64>::deserialize(deserializer)?.map(Duration::from_millis))
    }
}
