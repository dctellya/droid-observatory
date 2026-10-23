//! Replay of session files held in memory, for the browser host (architecture §7
//! `BrowserBackend`): the same tree discovery, mapping and event order as the CLI replay, with
//! the file picker's files in place of a directory and no store.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::capture::CaptureHeader;
use super::sessions::{Files, Head, Transcript, scan};
use super::sidecar::{Sidecar, sidecar_path};
use super::{IngestError, RawRecord, SessionTree, capture_path};
use crate::mapping::{TreeMapper, select_version};
use crate::model::{Event, EventKind, ModelId, Role, SessionId, SourcePtr};
use crate::view::MissionSummary;

const JSONL_EXTENSION: &str = "jsonl";
const CAPTURE_SUFFIX: &str = ".capture.json";

/// A session of the opened tree, for naming rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionInfo {
    pub id: SessionId,
    pub parent: Option<SessionId>,
    pub role: Role,
    pub model: Option<ModelId>,
    /// Name of the first sidecar tag.
    pub tag: Option<String>,
}

/// The events of an opened selection, in `(at, session, byte offset, sub)` order, and the bytes
/// their pointers read.
#[derive(Debug)]
pub struct MemoryReplay {
    pub summary: MissionSummary,
    pub events: Vec<Event>,
    pub sessions: Vec<SessionInfo>,
    files: BTreeMap<PathBuf, Vec<u8>>,
}

impl MemoryReplay {
    /// The line a pointer names, as it is in the selected file; `None` when the file is not in
    /// the selection or is shorter than the pointer says.
    pub fn raw_line(&self, pointer: &SourcePtr) -> Option<&[u8]> {
        let SourcePtr::Jsonl {
            path,
            byte_offset,
            byte_len,
        } = pointer
        else {
            return None;
        };
        let start = usize::try_from(*byte_offset).ok()?;
        let end = start.checked_add(usize::try_from(*byte_len).ok()?)?;
        self.files.get(path)?.get(start..end)
    }
}

struct MemoryFiles<'a>(&'a BTreeMap<PathBuf, Vec<u8>>);

impl MemoryFiles<'_> {
    fn bytes(&self, path: &Path) -> &[u8] {
        self.0.get(path).map_or(&[], Vec::as_slice)
    }
}

impl Files for MemoryFiles<'_> {
    fn head(&self, path: &Path) -> Result<Option<Head>, IngestError> {
        let first = self.bytes(path).split(|byte| *byte == b'\n').next();
        Ok(first.and_then(Head::of_line))
    }

    fn sidecar(&self, path: &Path) -> Result<Option<Sidecar>, IngestError> {
        let sidecar = sidecar_path(path);
        let Some(bytes) = self.0.get(&sidecar) else {
            return Ok(None);
        };
        serde_json::from_slice(bytes)
            .map(Some)
            .map_err(|err| IngestError::BadSidecar {
                path: sidecar,
                message: err.to_string(),
            })
    }

    fn transcript(&self, path: &Path) -> Result<Transcript, IngestError> {
        scan(self.bytes(path)).map_err(|source| IngestError::Io {
            path: path.to_owned(),
            source,
        })
    }
}

/// Opens a selection of files: the root session `.jsonl`, the child `.jsonl` files, their
/// `.settings.json` sidecars and the `.capture.json` header, keyed by the path they were picked
/// with (a file name alone is enough). The root is the session with no `callingSessionId`; with
/// several, the one with the largest tree.
pub fn replay_files(files: BTreeMap<PathBuf, Vec<u8>>) -> Result<MemoryReplay, IngestError> {
    let source = MemoryFiles(&files);
    let sessions_files: Vec<PathBuf> = files
        .keys()
        .filter(|path| path.extension().is_some_and(|ext| ext == JSONL_EXTENSION))
        .cloned()
        .collect();
    let mut roots = Vec::new();
    for path in &sessions_files {
        if source
            .head(path)?
            .is_some_and(|head| head.calling_session_id.is_none())
        {
            roots.push(path.clone());
        }
    }
    if roots.is_empty() {
        return Err(IngestError::NoSession { files: files.len() });
    }

    let header = capture_header(&files, &roots[0])?;
    let (version, version_warning) =
        select_version(header.as_ref().and_then(|h| h.droid_version.as_deref()));
    let mut best: Option<SessionTree> = None;
    for root in &roots {
        let candidates = sessions_files.clone();
        let tree = SessionTree::assemble(&source, root, candidates, version)?;
        if best
            .as_ref()
            .is_none_or(|kept| tree.sessions().len() > kept.sessions().len())
        {
            best = Some(tree);
        }
    }
    let tree = best.expect("at least one root was found");

    let mut mapper = TreeMapper::new(&tree, version);
    let mut records = 0_u64;
    let mut events = Vec::new();
    for session in tree.sessions() {
        for record in records_of(&session.id, &session.path, source.bytes(&session.path)) {
            records += 1;
            events.extend(mapper.map(&record));
        }
    }
    events.extend(mapper.finish());
    events.sort_by_key(sort_key);

    let unknown = events
        .iter()
        .filter(|event| matches!(event.kind, EventKind::Unknown { .. }))
        .count() as u64;
    let root = tree.root();
    let name = header
        .as_ref()
        .and_then(|h| h.mission_name.clone())
        .or_else(|| root.first_tag().map(str::to_owned))
        .unwrap_or_else(|| tree.mission_id().0.clone());
    let started_at = header
        .as_ref()
        .and_then(|h| h.captured_at.clone())
        .or_else(|| events.first().map(|event| event.at.to_rfc3339_millis()))
        .unwrap_or_default();
    let mut warnings: Vec<String> = version_warning.iter().map(ToString::to_string).collect();
    if roots.len() > 1 {
        warnings.push(format!(
            "{} root sessions in the selection: opened the one with the most sessions",
            roots.len()
        ));
    }
    let summary = MissionSummary {
        mission_id: tree.mission_id().clone(),
        name,
        started_at,
        event_count: events.len() as u64,
        unknown_count: unknown,
        record_count: records,
        session_count: tree.sessions().len() as u64,
        mapping_version: version.to_string(),
        warning: (!warnings.is_empty()).then(|| warnings.join("; ")),
    };
    let sessions = tree
        .sessions()
        .iter()
        .map(|session| SessionInfo {
            id: session.id.clone(),
            parent: session.parent.clone(),
            role: session.role,
            model: session.model().map(|model| ModelId(model.to_owned())),
            tag: session.first_tag().map(str::to_owned),
        })
        .collect();
    Ok(MemoryReplay {
        summary,
        events,
        sessions,
        files,
    })
}

/// `<root stem>.capture.json` when the selection has it, else the only other capture header.
fn capture_header(
    files: &BTreeMap<PathBuf, Vec<u8>>,
    root: &Path,
) -> Result<Option<CaptureHeader>, IngestError> {
    let own = capture_path(root);
    let path = if files.contains_key(&own) {
        Some(own)
    } else {
        files
            .keys()
            .find(|path| path.to_string_lossy().ends_with(CAPTURE_SUFFIX))
            .cloned()
    };
    let Some(path) = path else {
        return Ok(None);
    };
    serde_json::from_slice(&files[&path])
        .map(Some)
        .map_err(|err| IngestError::BadCapture {
            path,
            message: err.to_string(),
        })
}

/// The lines of a file as the tail reader yields them: split on newlines, an unfinished last
/// line included (a replay's file will not grow).
fn records_of(session: &SessionId, path: &Path, bytes: &[u8]) -> Vec<RawRecord> {
    let mut records = Vec::new();
    let mut offset = 0_usize;
    let mut lines = bytes.split(|byte| *byte == b'\n').peekable();
    while let Some(line) = lines.next() {
        let last = lines.peek().is_none();
        if !(last && line.is_empty()) {
            records.push(RawRecord {
                ptr: SourcePtr::Jsonl {
                    path: path.to_owned(),
                    byte_offset: offset as u64,
                    byte_len: u32::try_from(line.len()).unwrap_or(u32::MAX),
                },
                session_id: session.clone(),
                bytes: line.to_vec(),
            });
        }
        offset += line.len() + 1;
    }
    records
}

fn sort_key(event: &Event) -> (crate::model::Timestamp, String, u64, u32) {
    let offset = match &event.source {
        SourcePtr::Jsonl { byte_offset, .. } | SourcePtr::Rpc { byte_offset, .. } => *byte_offset,
        _ => 0,
    };
    let sub = event
        .id
        .0
        .rsplit(':')
        .next()
        .and_then(|sub| sub.parse().ok())
        .unwrap_or(0);
    (
        event.at,
        event
            .session_id
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_default(),
        offset,
        sub,
    )
}
