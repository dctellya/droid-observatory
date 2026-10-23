//! The `<slug>.capture.json` header that sits beside a fixture (architecture §4.2).

use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::IngestError;
use crate::mapping::{MappingVersion, Warning, select_version};

/// The part of the header the readers use. Other keys are ignored.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct CaptureHeader {
    pub droid_version: Option<String>,
}

/// `<dir>/<slug>.jsonl` has its header at `<dir>/<slug>.capture.json`.
pub fn capture_path(jsonl_path: &Path) -> PathBuf {
    jsonl_path.with_extension("capture.json")
}

/// `None` when the file has no header, as for a live session directory.
pub fn read_capture(jsonl_path: &Path) -> Result<Option<CaptureHeader>, IngestError> {
    let path = capture_path(jsonl_path);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => return Err(IngestError::Io { path, source }),
    };
    serde_json::from_str(&text)
        .map(Some)
        .map_err(|err| IngestError::BadCapture {
            path,
            message: err.to_string(),
        })
}

/// Mapping version for a session file: chosen by the header's Droid version, `LATEST`
/// when there is no header or no version in it.
pub fn mapping_version_of(jsonl_path: &Path) -> Result<MappingVersion, IngestError> {
    let version = read_capture(jsonl_path)?
        .and_then(|header| header.droid_version)
        .map_or(MappingVersion::LATEST, |seen| {
            MappingVersion::for_droid_version(&seen)
        });
    Ok(version)
}

/// Like `mapping_version_of`, with the warning to show once when the header's Droid version has
/// no mapping of its own (or the header names none).
pub fn select_mapping_of(
    jsonl_path: &Path,
) -> Result<(MappingVersion, Option<Warning>), IngestError> {
    let header = read_capture(jsonl_path)?;
    Ok(select_version(
        header
            .as_ref()
            .and_then(|header| header.droid_version.as_deref()),
    ))
}
