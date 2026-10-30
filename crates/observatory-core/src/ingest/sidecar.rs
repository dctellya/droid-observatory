//! The `<session-id>.settings.json` that sits beside each session file (architecture §4.2).
//! Token usage, model and tags live here, not in the JSONL lines.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::Value;

use super::IngestError;
use crate::model::TokenUsage;

/// Name of the tag the mission runner puts on every session it starts, and on the orchestrator.
pub(crate) const MISSION_SESSION_TAG: &str = "mission-session";

/// One entry of the sidecar `tags` array: an object, not a string.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct SidecarTag {
    pub name: String,
    #[serde(default)]
    pub metadata: Option<Value>,
}

/// A usage block. A key Droid leaves out (`lastCallTokenUsage` omits zero counts) reads as 0.
#[derive(Clone, Copy, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SidecarUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_creation_tokens: u64,
    pub thinking_tokens: u64,
    pub factory_credits: Option<f64>,
}

impl SidecarUsage {
    pub fn to_token_usage(&self) -> TokenUsage {
        TokenUsage {
            input: self.input_tokens,
            output: self.output_tokens,
            cache_read: self.cache_read_tokens,
            cache_write: self.cache_creation_tokens,
            credits: self.factory_credits,
        }
    }
}

/// The fields the observatory reads. Other keys (`systemPrompt`, `enabledToolIds`, ...) are
/// ignored at parse time, so the system prompt is never held.
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Sidecar {
    pub model: Option<String>,
    pub reasoning_effort: Option<String>,
    pub interaction_mode: Option<String>,
    pub autonomy_level: Option<String>,
    pub tags: Option<Vec<SidecarTag>>,
    pub assistant_active_time_ms: Option<u64>,
    pub token_usage: Option<SidecarUsage>,
    pub inclusive_token_usage: Option<SidecarUsage>,
    pub last_call_token_usage: Option<SidecarUsage>,
    pub child_inclusive_token_usage_by_session_id: BTreeMap<String, SidecarUsage>,
}

impl Sidecar {
    /// The `mission-session` tag, present on the orchestrator and on runner-started sessions.
    fn mission_tag(&self) -> Option<&SidecarTag> {
        self.tags
            .iter()
            .flatten()
            .find(|tag| tag.name == MISSION_SESSION_TAG)
    }

    fn mission_metadata(&self, key: &str) -> Option<&str> {
        self.mission_tag()?.metadata.as_ref()?.get(key)?.as_str()
    }

    /// `metadata.role` of the `mission-session` tag.
    pub fn mission_role(&self) -> Option<&str> {
        self.mission_metadata("role")
    }

    /// `metadata.missionId` of the `mission-session` tag.
    pub fn mission_id(&self) -> Option<&str> {
        self.mission_metadata("missionId")
    }
}

/// `<dir>/<id>.jsonl` has its sidecar at `<dir>/<id>.settings.json`.
pub fn sidecar_path(jsonl_path: &Path) -> PathBuf {
    jsonl_path.with_extension("settings.json")
}

/// `None` when the sibling file is absent.
pub fn read_sidecar(jsonl_path: &Path) -> Result<Option<Sidecar>, IngestError> {
    let path = sidecar_path(jsonl_path);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => return Err(IngestError::Io { path, source }),
    };
    serde_json::from_str(&text)
        .map(Some)
        .map_err(|err| IngestError::BadSidecar {
            path,
            message: err.to_string(),
        })
}
