//! Replay of session files held in memory, for the browser host (architecture §7
//! `BrowserBackend`): the same tree discovery, mapping and event order as the CLI replay, with
//! the file picker's files in place of a directory and no store.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::capture::CaptureHeader;
use super::capture::missing_usage_log_warning;
use super::sessions::{Files, Head, Transcript, feature_id_in, scan};
use super::sidecar::{Sidecar, sidecar_path};
use super::{IngestError, RawRecord, SessionTree, UsageLogSource, capture_path};
use crate::config::IngestConfig;
use crate::mapping::{TreeMapper, select_version};
use crate::model::{Event, EventKind, ModelId, Role, SessionId, SourcePtr};
use crate::view::MissionSummary;

const JSONL_EXTENSION: &str = "jsonl";
const CAPTURE_SUFFIX: &str = ".capture.json";

/// A session of the opened tree, for naming rows.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: SessionId,
    pub parent: Option<SessionId>,
    pub role: Role,
    pub model: Option<ModelId>,
    /// The session name: its feature id, else its first user tag, else its id.
    pub tag: Option<String>,
    /// Sidecar `reasoningEffort`.
    pub effort: Option<String>,
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

    fn feature_id(&self, path: &Path) -> Result<Option<String>, IngestError> {
        Ok(feature_id_in(self.bytes(path)))
    }
}

/// Opens a selection of files: the root session `.jsonl`, the child `.jsonl` files, their
/// `.settings.json` sidecars and the `.capture.json` header, keyed by the path they were picked
/// with (a file name alone is enough). The root is the session with no `callingSessionId`; with
/// several, the one with the largest tree.
pub fn replay_files(files: BTreeMap<PathBuf, Vec<u8>>) -> Result<MemoryReplay, IngestError> {
    replay_files_with(files, &IngestConfig::default().runner_tags)
}

/// `replay_files` naming sessions with the `runner_tags` of the loaded configuration.
pub fn replay_files_with(
    files: BTreeMap<PathBuf, Vec<u8>>,
    runner_tags: &[String],
) -> Result<MemoryReplay, IngestError> {
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
    // The extract is a `.jsonl` too, but holds log lines, not a session.
    let usage_name = header
        .as_ref()
        .and_then(|h| h.usage_log.as_ref())
        .map(|info| info.file.clone());
    let usage_key = usage_name.as_deref().and_then(|name| {
        let name = Path::new(name).file_name()?;
        files
            .keys()
            .find(|path| path.file_name() == Some(name))
            .cloned()
    });
    let sessions_files: Vec<PathBuf> = sessions_files
        .into_iter()
        .filter(|path| Some(path) != usage_key.as_ref())
        .collect();
    let mut best: Option<SessionTree> = None;
    for root in &roots {
        let candidates = sessions_files.clone();
        let tree = SessionTree::assemble(&source, root, candidates, version, runner_tags)?;
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
    let mut warnings: Vec<String> = version_warning.iter().map(ToString::to_string).collect();
    match (&usage_name, &usage_key) {
        (Some(_), Some(key)) => {
            let mut log = UsageLogSource::in_memory(&IngestConfig::default());
            let turns = log.read_bytes(key, source.bytes(key));
            events.extend(turns.iter().filter_map(|turn| mapper.map_usage(turn)));
        }
        (Some(name), None) => warnings.push(missing_usage_log_warning(name)),
        _ => {}
    }
    events.sort_by_key(order_key);

    let unknown = events
        .iter()
        .filter(|event| matches!(event.kind, EventKind::Unknown { .. }))
        .count() as u64;
    let root = tree.root();
    let name = header
        .as_ref()
        .and_then(|h| h.mission_name.clone())
        .or_else(|| root.user_tag.clone())
        .unwrap_or_else(|| tree.mission_id().0.clone());
    let started_at = header
        .as_ref()
        .and_then(|h| h.captured_at.clone())
        .or_else(|| events.first().map(|event| event.at.to_rfc3339_millis()))
        .unwrap_or_default();
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
        event_count: events
            .iter()
            .filter(|event| !event.kind.is_panel_only())
            .count() as u64,
        unknown_count: unknown,
        record_count: records,
        session_count: tree.sessions().len() as u64,
        mapping_version: version.to_string(),
        warning: (!warnings.is_empty()).then(|| warnings.join("; ")),
    };
    let sessions = session_infos(&tree);
    Ok(MemoryReplay {
        summary,
        events,
        sessions,
        files,
    })
}

/// The sessions of a tree, for naming rows.
pub fn session_infos(tree: &SessionTree) -> Vec<SessionInfo> {
    tree.sessions()
        .iter()
        .map(|session| SessionInfo {
            id: session.id.clone(),
            parent: session.parent.clone(),
            role: session.role,
            model: session.model().map(|model| ModelId(model.to_owned())),
            tag: Some(session.display_name()),
            effort: session
                .sidecar
                .as_ref()
                .and_then(|sidecar| sidecar.reasoning_effort.clone()),
        })
        .collect()
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
                arrived_at: None,
            });
        }
        offset += line.len() + 1;
    }
    records
}

/// Where an event stands in a replay: time, then session, then position in the file. Attach
/// sorts what it receives by this key, so a live board and a replay list the same order.
pub fn order_key(event: &Event) -> (crate::model::Timestamp, String, u64, u32) {
    let offset = match &event.source {
        SourcePtr::Jsonl { byte_offset, .. }
        | SourcePtr::Rpc { byte_offset, .. }
        | SourcePtr::UsageLog { byte_offset, .. } => *byte_offset,
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
