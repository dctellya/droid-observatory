//! Child discovery and the session tree (architecture §4.2). A session is a child when the
//! `callingSessionId` on its first line names a session already in the tree.

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};

use serde_json::Value;

use super::IngestError;
use super::capture::mapping_version_of;
use super::sidecar::{Sidecar, SidecarTag, read_sidecar};
use crate::config::IngestConfig;
use crate::mapping::{
    MappingVersion, RoleEvidence, SessionMeta, is_contract_task, proposal_milestones, resolve_role,
};
use crate::model::{MilestoneId, MissionId, ModelId, Role, SessionId, TokenUsage};

const KB: u64 = 1024;
const JSONL_EXTENSION: &str = "jsonl";
/// A fixture keeps the children of `<slug>.jsonl` in `<slug>.children/`; a live directory keeps
/// them beside it.
const CHILDREN_DIR_SUFFIX: &str = "children";

/// Metadata keys only the runner sets on the tags it creates.
const RUNNER_METADATA_KEYS: [&str; 3] = ["role", "missionId", "callingSessionId"];

/// The first tag a user wrote: not named in `runner_tags` and not carrying runner metadata.
pub fn user_tag<'a>(tags: &'a [SidecarTag], runner_tags: &[String]) -> Option<&'a str> {
    tags.iter()
        .find(|tag| {
            !runner_tags.contains(&tag.name)
                && !RUNNER_METADATA_KEYS.iter().any(|key| {
                    tag.metadata
                        .as_ref()
                        .is_some_and(|meta| meta.get(key).is_some())
                })
        })
        .map(|tag| tag.name.as_str())
}

/// What the UI calls a session (FR-L1, F-038): its feature id, else its first user tag, else its
/// id. The watcher, the store and the in-memory replay all name sessions through this one rule.
pub fn display_name(
    feature_id: Option<&str>,
    tags: &[SidecarTag],
    id: &SessionId,
    runner_tags: &[String],
) -> String {
    feature_id
        .or_else(|| user_tag(tags, runner_tags))
        .map_or_else(|| id.0.clone(), str::to_owned)
}

/// The runner's sentence in the first user message of a worker session (Droid 0.247, D-036).
const ASSIGNMENT: &str = "assigned to execute feature \"";

/// The feature id a runner-started session was assigned (D-036): the text parts of the user
/// messages before the first assistant message. Reads `reader` line by line and stops at the
/// first assistant message, so a transcript that quotes the sentence later is not renamed.
pub fn feature_id_in(mut reader: impl BufRead) -> Option<String> {
    let mut line = Vec::new();
    loop {
        line.clear();
        if reader.read_until(b'\n', &mut line).ok()? == 0 {
            return None;
        }
        let Ok(record) = serde_json::from_slice::<Value>(&line) else {
            continue;
        };
        match record["message"]["role"].as_str() {
            Some("assistant") => return None,
            Some("user") => {}
            _ => continue,
        }
        let content = &record["message"]["content"];
        let parts: Vec<&str> = match content {
            Value::String(text) => vec![text.as_str()],
            Value::Array(parts) => parts
                .iter()
                .filter_map(|part| part["text"].as_str())
                .collect(),
            _ => Vec::new(),
        };
        if let Some(id) = parts.into_iter().find_map(assigned_feature) {
            return Some(id);
        }
    }
}

/// `<id>` of `assigned to execute feature "<id>"`, when it is a plausible identifier.
fn assigned_feature(text: &str) -> Option<String> {
    let plain = |c: char| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | ':' | '-');
    text.match_indices(ASSIGNMENT).find_map(|(at, _)| {
        let rest = &text[at + ASSIGNMENT.len()..];
        let id = &rest[..rest.find('"')?];
        (!id.is_empty() && id.chars().all(plain)).then(|| id.to_owned())
    })
}

/// `feature_id_in` over the first `head_bytes` of the file at `path`; `None` when the file
/// cannot be read.
pub fn feature_id_of(path: &Path, head_bytes: usize) -> Option<String> {
    let file = File::open(path).ok()?;
    feature_id_in(BufReader::new(file.take(head_bytes as u64)))
}

/// One session of a mission, with what its sidecar says.
#[derive(Clone, Debug, PartialEq)]
pub struct Session {
    pub id: SessionId,
    pub path: PathBuf,
    pub parent: Option<SessionId>,
    pub calling_tool_use_id: Option<String>,
    pub role: Role,
    /// Spawned by a `Task` whose description names the validation contract.
    pub contract: bool,
    /// `None` when the sibling `settings.json` is absent.
    pub sidecar: Option<Sidecar>,
    /// The first tag a user wrote, by the `runner_tags` the tree was assembled with.
    pub user_tag: Option<String>,
    /// The feature the runner assigned the session, read from its first user message (D-036).
    pub feature_id: Option<String>,
    name: String,
}

impl Session {
    pub fn model(&self) -> Option<&str> {
        self.sidecar.as_ref()?.model.as_deref()
    }

    /// The session's own `tokenUsage`.
    pub fn usage(&self) -> Option<TokenUsage> {
        self.sidecar
            .as_ref()?
            .token_usage
            .as_ref()
            .map(|usage| usage.to_token_usage())
    }

    pub fn tags(&self) -> &[SidecarTag] {
        self.sidecar
            .as_ref()
            .and_then(|sidecar| sidecar.tags.as_deref())
            .unwrap_or_default()
    }

    /// What the UI shows as the session name: see [`display_name`].
    pub fn display_name(&self) -> String {
        self.name.clone()
    }

    /// Name of the first tag, runner-generated or not.
    pub fn first_tag(&self) -> Option<&str> {
        self.tags().first().map(|tag| tag.name.as_str())
    }
}

/// The root session and every session that descends from it, parents before children.
#[derive(Clone, Debug, PartialEq)]
pub struct SessionTree {
    mission_id: MissionId,
    /// Milestone ids of the plan the root proposed, in the plan's order.
    milestones: Vec<MilestoneId>,
    sessions: Vec<Session>,
}

impl SessionTree {
    /// `root` is the orchestrator's `.jsonl`. Siblings of it and the files in `<stem>.children/`
    /// are candidates; each one whose first line names a session already attached joins the tree.
    pub fn discover(root: &Path, cfg: &IngestConfig) -> Result<Self, IngestError> {
        let head_bytes =
            usize::try_from(cfg.tail_chunk_kb.saturating_mul(KB)).unwrap_or(usize::MAX);
        let version = mapping_version_of(root)?;
        Self::assemble(
            &DiskFiles { head_bytes },
            root,
            candidate_paths(root)?,
            version,
            &cfg.runner_tags,
        )
    }

    /// The tree of `root` among `candidates`, read through `files`.
    pub(super) fn assemble(
        files: &dyn Files,
        root: &Path,
        candidates: Vec<PathBuf>,
        version: MappingVersion,
        runner_tags: &[String],
    ) -> Result<Self, IngestError> {
        let root_head = files.head(root)?.unwrap_or_default();
        let root_id = SessionId(root_head.id.clone().unwrap_or_else(|| file_stem(root)));

        let mut known: HashSet<SessionId> = HashSet::from([root_id.clone()]);
        let mut pending = Vec::new();
        for path in candidates {
            let Some(head) = files.head(&path)? else {
                continue;
            };
            if let (Some(id), Some(caller)) = (head.id.clone(), head.calling_session_id.clone()) {
                pending.push(Attached {
                    id: SessionId(id),
                    parent: SessionId(caller),
                    path,
                    calling_tool_use_id: head.calling_tool_use_id,
                });
            }
        }
        let mut attached = vec![Attached {
            id: root_id.clone(),
            parent: root_id.clone(),
            path: root.to_owned(),
            calling_tool_use_id: None,
        }];
        // A round attaches every candidate whose caller joined in an earlier round, so a parent
        // always precedes its children whatever order the files are listed in.
        loop {
            let (ready, rest): (Vec<_>, Vec<_>) = pending
                .into_iter()
                .partition(|candidate| known.contains(&candidate.parent));
            pending = rest;
            if ready.is_empty() {
                break;
            }
            for candidate in ready {
                if known.insert(candidate.id.clone()) {
                    attached.push(candidate);
                }
            }
        }

        let sidecars = attached
            .iter()
            .map(|entry| files.sidecar(&entry.path))
            .collect::<Result<Vec<_>, _>>()?;
        let mission_id = sidecars[0]
            .as_ref()
            .and_then(Sidecar::mission_id)
            .map_or_else(
                || MissionId(root_id.0.clone()),
                |id| MissionId(id.to_owned()),
            );

        let mut transcripts = Transcripts::new(files);
        let milestones = transcripts
            .get(&attached[0].path)?
            .proposals
            .iter()
            .flat_map(|proposal| proposal_milestones(version, proposal))
            .fold(Vec::new(), |mut all, milestone| {
                if !all.contains(&milestone) {
                    all.push(milestone);
                }
                all
            });
        let mut sessions = Vec::with_capacity(attached.len());
        for (index, (entry, sidecar)) in attached.iter().zip(sidecars).enumerate() {
            let is_root = index == 0;
            let parent = (!is_root).then(|| entry.parent.clone());
            let role = resolve(
                version,
                entry,
                is_root,
                sidecar.as_ref(),
                &attached[..],
                &mut transcripts,
            )?;
            let tags = sidecar
                .as_ref()
                .and_then(|sidecar| sidecar.tags.as_deref())
                .unwrap_or_default();
            let user_tag = user_tag(tags, runner_tags).map(str::to_owned);
            let feature_id = files.feature_id(&entry.path)?;
            let name = display_name(feature_id.as_deref(), tags, &entry.id, runner_tags);
            sessions.push(Session {
                id: entry.id.clone(),
                path: entry.path.clone(),
                parent,
                calling_tool_use_id: entry.calling_tool_use_id.clone(),
                role,
                contract: is_contract(version, entry, &attached[..], &mut transcripts)?,
                sidecar,
                user_tag,
                feature_id,
                name,
            });
        }
        Ok(Self {
            mission_id,
            milestones,
            sessions,
        })
    }

    /// The Factory mission id from the root's `mission-session` tag, else the root session id.
    pub fn mission_id(&self) -> &MissionId {
        &self.mission_id
    }

    pub fn root(&self) -> &Session {
        &self.sessions[0]
    }

    /// Root first, then every session after the one that spawned it.
    pub fn sessions(&self) -> &[Session] {
        &self.sessions
    }

    pub fn get(&self, id: &SessionId) -> Option<&Session> {
        self.sessions.iter().find(|session| session.id == *id)
    }

    pub fn children_of<'a>(&'a self, id: &'a SessionId) -> impl Iterator<Item = &'a Session> {
        self.sessions
            .iter()
            .filter(move |session| session.parent.as_ref() == Some(id))
    }

    /// What the mapping needs to know about a session of this tree.
    pub fn meta(&self, session: &Session) -> SessionMeta {
        SessionMeta {
            id: session.id.clone(),
            mission_id: self.mission_id.clone(),
            role: session.role,
            model: session.model().map(|model| ModelId(model.to_owned())),
            usage: session.usage(),
            milestones: self.milestones.clone(),
            contract: session.contract,
        }
    }
}

struct Attached {
    id: SessionId,
    parent: SessionId,
    path: PathBuf,
    calling_tool_use_id: Option<String>,
}

fn resolve(
    version: MappingVersion,
    entry: &Attached,
    is_root: bool,
    sidecar: Option<&Sidecar>,
    all: &[Attached],
    transcripts: &mut Transcripts<'_>,
) -> Result<Role, IngestError> {
    let parent_path = all
        .iter()
        .find(|other| other.id == entry.parent)
        .map(|other| other.path.as_path());
    let subagent_type = match (&entry.calling_tool_use_id, parent_path) {
        (Some(tool_use_id), Some(path)) => transcripts
            .get(path)?
            .subagent_types
            .get(tool_use_id)
            .cloned(),
        _ => None,
    };
    let skills = if entry.calling_tool_use_id.is_none() && !is_root {
        transcripts.get(&entry.path)?.skills.clone()
    } else {
        Vec::new()
    };
    let calling_session_id = (!is_root).then_some(entry.parent.0.as_str());
    Ok(resolve_role(
        version,
        &RoleEvidence {
            calling_session_id,
            calling_tool_use_id: entry.calling_tool_use_id.as_deref(),
            subagent_type: subagent_type.as_deref(),
            tag_role: sidecar.and_then(Sidecar::mission_role),
            skills: &skills,
        },
    ))
}

fn is_contract(
    version: MappingVersion,
    entry: &Attached,
    all: &[Attached],
    transcripts: &mut Transcripts<'_>,
) -> Result<bool, IngestError> {
    let parent_path = all
        .iter()
        .find(|other| other.id == entry.parent && other.id != entry.id)
        .map(|other| other.path.as_path());
    let (Some(tool_use_id), Some(path)) = (&entry.calling_tool_use_id, parent_path) else {
        return Ok(false);
    };
    Ok(transcripts
        .get(path)?
        .task_descriptions
        .get(tool_use_id)
        .is_some_and(|description| is_contract_task(version, description)))
}

/// First line of a session file.
#[derive(Default)]
pub(super) struct Head {
    pub(super) id: Option<String>,
    pub(super) calling_session_id: Option<String>,
    pub(super) calling_tool_use_id: Option<String>,
}

impl Head {
    /// `None` when the line is not a `session_start`, so a stray `.jsonl` is not a session.
    pub(super) fn of_line(line: &[u8]) -> Option<Self> {
        let line = serde_json::from_slice::<Value>(line).ok()?;
        if line["type"] != "session_start" {
            return None;
        }
        let text = |key: &str| line[key].as_str().map(str::to_owned);
        Some(Self {
            id: text("id"),
            calling_session_id: text("callingSessionId"),
            calling_tool_use_id: text("callingToolUseId"),
        })
    }
}

/// Where the files of a tree are read from: the disk, or memory for the browser host.
pub(super) trait Files {
    fn head(&self, path: &Path) -> Result<Option<Head>, IngestError>;
    fn sidecar(&self, path: &Path) -> Result<Option<Sidecar>, IngestError>;
    fn transcript(&self, path: &Path) -> Result<Transcript, IngestError>;
    fn feature_id(&self, path: &Path) -> Result<Option<String>, IngestError>;
}

struct DiskFiles {
    head_bytes: usize,
}

impl Files for DiskFiles {
    fn head(&self, path: &Path) -> Result<Option<Head>, IngestError> {
        let mut bytes = Vec::new();
        let file = File::open(path).map_err(|source| io_error(path, source))?;
        BufReader::new(file.take(self.head_bytes as u64))
            .read_until(b'\n', &mut bytes)
            .map_err(|source| io_error(path, source))?;
        Ok(Head::of_line(&bytes))
    }

    fn sidecar(&self, path: &Path) -> Result<Option<Sidecar>, IngestError> {
        read_sidecar(path)
    }

    fn transcript(&self, path: &Path) -> Result<Transcript, IngestError> {
        let file = File::open(path).map_err(|source| io_error(path, source))?;
        scan(BufReader::new(file)).map_err(|source| io_error(path, source))
    }

    fn feature_id(&self, path: &Path) -> Result<Option<String>, IngestError> {
        let file = File::open(path).map_err(|source| io_error(path, source))?;
        Ok(feature_id_in(BufReader::new(
            file.take(self.head_bytes as u64),
        )))
    }
}

fn candidate_paths(root: &Path) -> Result<Vec<PathBuf>, IngestError> {
    let dir = match root.parent() {
        Some(dir) if !dir.as_os_str().is_empty() => dir,
        _ => Path::new("."),
    };
    let mut paths = jsonl_files_in(dir)?;
    paths.extend(jsonl_files_in(
        &dir.join(format!("{}.{CHILDREN_DIR_SUFFIX}", file_stem(root))),
    )?);
    Ok(paths)
}

/// Sorted, so discovery is deterministic. A directory that does not exist has no files.
fn jsonl_files_in(dir: &Path) -> Result<Vec<PathBuf>, IngestError> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => return Err(io_error(dir, source)),
    };
    let mut paths = Vec::new();
    for entry in entries {
        let path = entry.map_err(|source| io_error(dir, source))?.path();
        if path.extension().is_some_and(|ext| ext == JSONL_EXTENSION) && path.is_file() {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

fn file_stem(path: &Path) -> String {
    path.file_stem()
        .map_or_else(String::new, |stem| stem.to_string_lossy().into_owned())
}

fn io_error(path: &Path, source: std::io::Error) -> IngestError {
    IngestError::Io {
        path: path.to_owned(),
        source,
    }
}

/// What role resolution reads from a transcript.
#[derive(Default)]
pub(super) struct Transcript {
    /// `Task` tool_use id to its `input.subagent_type`.
    subagent_types: HashMap<String, String>,
    /// `Task` tool_use id to its `input.description`.
    task_descriptions: HashMap<String, String>,
    /// `input.skill` of every `Skill` tool_use, in order.
    skills: Vec<String>,
    /// `input.proposal` of every `ProposeMission` tool_use, in order.
    proposals: Vec<String>,
}

/// Transcripts are large and only a few are needed, so each is read at most once and only on demand.
struct Transcripts<'a> {
    files: &'a dyn Files,
    scanned: HashMap<PathBuf, Transcript>,
}

impl<'a> Transcripts<'a> {
    fn new(files: &'a dyn Files) -> Self {
        Self {
            files,
            scanned: HashMap::new(),
        }
    }

    fn get(&mut self, path: &Path) -> Result<&Transcript, IngestError> {
        if !self.scanned.contains_key(path) {
            self.scanned
                .insert(path.to_owned(), self.files.transcript(path)?);
        }
        Ok(&self.scanned[path])
    }
}

const WATCHED_TOOLS: [&[u8]; 3] = [
    br#""name":"Task""#,
    br#""name":"Skill""#,
    br#""name":"ProposeMission""#,
];

/// Reads the transcript lines of `reader`, which hold the tool_uses role resolution needs.
pub(super) fn scan(mut reader: impl BufRead) -> std::io::Result<Transcript> {
    let mut transcript = Transcript::default();
    let mut line = Vec::new();
    loop {
        line.clear();
        if reader.read_until(b'\n', &mut line)? == 0 {
            return Ok(transcript);
        }
        // Compact JSON writes the part as `"name":"Task"`; the same text inside a string value
        // is escaped, so this filter skips parsing lines that cannot hold any of the tool_uses.
        if !WATCHED_TOOLS.iter().any(|name| contains(&line, name)) {
            continue;
        }
        let Ok(value) = serde_json::from_slice::<Value>(&line) else {
            continue;
        };
        let parts = value["message"]["content"].as_array().into_iter().flatten();
        for part in parts.filter(|part| part["type"] == "tool_use") {
            match part["name"].as_str() {
                Some("Task") => {
                    if let (Some(id), Some(description)) =
                        (part["id"].as_str(), part["input"]["description"].as_str())
                    {
                        transcript
                            .task_descriptions
                            .insert(id.to_owned(), description.to_owned());
                    }
                    if let (Some(id), Some(kind)) =
                        (part["id"].as_str(), part["input"]["subagent_type"].as_str())
                    {
                        transcript
                            .subagent_types
                            .insert(id.to_owned(), kind.to_owned());
                    }
                }
                Some("ProposeMission") => {
                    if let Some(proposal) = part["input"]["proposal"].as_str() {
                        transcript.proposals.push(proposal.to_owned());
                    }
                }
                Some("Skill") => {
                    if let Some(skill) = part["input"]["skill"].as_str() {
                        transcript.skills.push(skill.to_owned());
                    }
                }
                _ => {}
            }
        }
    }
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}
