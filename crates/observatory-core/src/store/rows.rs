//! Event columns: the flat form of an `Event` the `events` table holds. No column takes
//! raw text, so none is built here.

use std::path::{Path, PathBuf};

use serde::Serialize;

use super::StoreError;
use crate::model::{Event, EventKind, SourcePtr};

/// The pointer part of a row, in column order of the `events` table.
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct PointerColumns {
    pub source: String,
    pub path: Option<String>,
    pub byte_offset: Option<i64>,
    pub byte_len: Option<i64>,
    pub repo: Option<String>,
    pub sha: Option<String>,
    pub url: Option<String>,
    pub action_id: Option<i64>,
}

pub(super) fn utf8(path: &Path) -> Result<String, StoreError> {
    path.to_str()
        .map(str::to_owned)
        .ok_or_else(|| StoreError::NonUtf8Path {
            path: path.to_owned(),
        })
}

fn integer(value: u64, id: &str) -> Result<i64, StoreError> {
    i64::try_from(value).map_err(|_| StoreError::BadRow {
        id: id.to_owned(),
        message: format!("{value} does not fit a SQLite integer"),
    })
}

impl PointerColumns {
    pub fn of(ptr: &SourcePtr, id: &str) -> Result<Self, StoreError> {
        let file = |source: &str, path: &PathBuf, byte_offset, byte_len| {
            Ok(Self {
                source: source.to_owned(),
                path: Some(utf8(path)?),
                byte_offset: Some(integer(byte_offset, id)?),
                byte_len: Some(i64::from(byte_len)),
                ..Self::default()
            })
        };
        match ptr {
            SourcePtr::Jsonl {
                path,
                byte_offset,
                byte_len,
            } => file("jsonl", path, *byte_offset, *byte_len),
            SourcePtr::Rpc {
                path,
                byte_offset,
                byte_len,
            } => file("rpc", path, *byte_offset, *byte_len),
            SourcePtr::Git { repo, sha } => Ok(Self {
                source: "git".into(),
                repo: Some(utf8(repo)?),
                sha: Some(sha.clone()),
                ..Self::default()
            }),
            SourcePtr::GitHub { url } => Ok(Self {
                source: "github".into(),
                url: Some(url.clone()),
                ..Self::default()
            }),
            SourcePtr::App { action_id } => Ok(Self {
                source: "app".into(),
                action_id: Some(integer(*action_id, id)?),
                ..Self::default()
            }),
        }
    }

    /// Rebuilds the pointer from stored columns.
    pub fn into_ptr(self, id: &str) -> Result<SourcePtr, StoreError> {
        let bad = |message: &str| StoreError::BadRow {
            id: id.to_owned(),
            message: message.to_owned(),
        };
        let file = |this: &Self| -> Result<(PathBuf, u64, u32), StoreError> {
            let path = this.path.clone().ok_or_else(|| bad("no path"))?;
            let offset = this.byte_offset.ok_or_else(|| bad("no byte_offset"))?;
            let len = this.byte_len.ok_or_else(|| bad("no byte_len"))?;
            Ok((
                PathBuf::from(path),
                u64::try_from(offset).map_err(|_| bad("negative byte_offset"))?,
                u32::try_from(len).map_err(|_| bad("byte_len out of range"))?,
            ))
        };
        match self.source.as_str() {
            "jsonl" => {
                let (path, byte_offset, byte_len) = file(&self)?;
                Ok(SourcePtr::Jsonl {
                    path,
                    byte_offset,
                    byte_len,
                })
            }
            "rpc" => {
                let (path, byte_offset, byte_len) = file(&self)?;
                Ok(SourcePtr::Rpc {
                    path,
                    byte_offset,
                    byte_len,
                })
            }
            "git" => Ok(SourcePtr::Git {
                repo: PathBuf::from(self.repo.ok_or_else(|| bad("no repo"))?),
                sha: self.sha.ok_or_else(|| bad("no sha"))?,
            }),
            "github" => Ok(SourcePtr::GitHub {
                url: self.url.ok_or_else(|| bad("no url"))?,
            }),
            "app" => Ok(SourcePtr::App {
                action_id: u64::try_from(self.action_id.ok_or_else(|| bad("no action_id"))?)
                    .map_err(|_| bad("negative action_id"))?,
            }),
            other => Err(bad(&format!("unknown source {other:?}"))),
        }
    }
}

/// The text a unit-variant enum has on the wire (`"scrutiny_validator"`).
pub(super) fn wire_text<T: Serialize>(value: &T) -> String {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::String(text)) => text,
        _ => String::new(),
    }
}

pub(super) fn raw_type(kind: &EventKind) -> Option<&str> {
    match kind {
        EventKind::Unknown { raw_type } => Some(raw_type),
        _ => None,
    }
}

/// `<sub>` of `<session_id>:<byte_offset>:<sub>`; 0 for an id of another shape.
pub(super) fn sub_of(event: &Event) -> i64 {
    event
        .id
        .0
        .rsplit(':')
        .next()
        .and_then(|sub| sub.parse().ok())
        .unwrap_or(0)
}
