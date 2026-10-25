//! `replay <file>`: attach that stops at the end of the file (D-008).

use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use observatory_core::config::IngestConfig;
use observatory_core::ingest::rpc::{RpcMapper, RpcSource};
use observatory_core::ingest::{
    JsonlSource, RawRecord, SessionTree, Source, UsageLogSource, UsageTurn, select_mapping_of,
};
use observatory_core::mapping::{Mapper, MappingVersion, TreeMapper};
use observatory_core::model::{CycleTemplate, Event, EventKind, Role, SessionId};
use observatory_core::store::{CaptureInfo, HostedRun, Store};

/// A spool line is a JSON-RPC envelope; a session line never carries this key first.
const SPOOL_FIRST_KEY: &str = r#"{"jsonrpc":"#;

/// What a replay reads besides the file: the ingest limits and the cycle whose lanes name the events.
pub struct Setup<'a> {
    pub ingest: &'a IngestConfig,
    pub template: &'a CycleTemplate,
}

/// Writes `<time>\t<lane>\t<kind>\t<session>` per event, then `unknown events: N`.
/// Child sessions found beside `file` or in `<slug>.children/` are replayed with it unless
/// `root_only` is set. Only reads session files; it writes to `out`, and to `warnings` once
/// when the capture's Droid version has no mapping of its own. With a `store`, the sessions
/// and the event pointers are written through to it as they are mapped.
///
/// With `usage_logs`, the turns of those Droid log files are joined to the capture's sessions
/// after its records: one `turn.usage` line per turn, one `totals` row per role, then
/// `unknown turns: N` before the closing `unknown events: N`. The log files are only read.
pub fn replay(
    file: &Path,
    root_only: bool,
    usage_logs: &[PathBuf],
    setup: &Setup<'_>,
    mut store: Option<&mut Store>,
    out: &mut impl Write,
    warnings: &mut impl Write,
) -> Result<(), Box<dyn Error>> {
    let (cfg, template) = (setup.ingest, setup.template);
    if is_spool(file)? {
        if !usage_logs.is_empty() {
            return Err("--usage-log joins turns to session files, not to a hosted spool".into());
        }
        return replay_spool(file, cfg, template, store, out, warnings);
    }
    let (version, warning) = select_mapping_of(file)?;
    let tree = SessionTree::discover(file, cfg)?;
    if let Some(warning) = warning {
        writeln!(warnings, "observatory-cli: warning: {warning}")?;
    }
    if let Some(store) = store.as_deref_mut() {
        store.record_tree(&tree, &CaptureInfo::from_session_file(file)?, root_only)?;
    }
    let (mut source, mut mapper) = if root_only {
        let source = JsonlSource::open(file, cfg)?;
        let root = Mapper::with_template(tree.meta(tree.root()), version, template.clone());
        (
            source,
            Mapping::Root(Box::new(root), tree.root().id.clone()),
        )
    } else {
        let source = JsonlSource::open_tree(&tree, cfg)?;
        (
            source,
            Mapping::Tree(TreeMapper::with_template(&tree, version, template.clone())),
        )
    };

    let mut unknown = 0_u64;
    loop {
        let batch = source.next_batch()?;
        let at_eof = source.at_eof();
        let records = if at_eof {
            batch.into_iter().chain(source.drain_partial()).collect()
        } else {
            batch
        };
        let mut events: Vec<Event> = records
            .iter()
            .flat_map(|record| mapper.map(record))
            .collect();
        if at_eof {
            events.extend(mapper.finish());
        }
        if let Some(store) = store.as_deref_mut() {
            store.insert_events(&events)?;
        }
        print_events(&events, template, &mut unknown, out)?;
        if at_eof {
            break;
        }
    }
    if !usage_logs.is_empty() {
        let mut log = UsageLogSource::open_files(usage_logs, cfg)?;
        let turns = log.read_all()?;
        let mut elsewhere = 0_u64;
        let events: Vec<Event> = turns
            .iter()
            .filter_map(|turn| {
                let event = mapper.map_usage(turn);
                elsewhere += u64::from(event.is_none() && turn.session_id.is_some());
                event
            })
            .collect();
        if let Some(store) = store.as_deref_mut() {
            store.insert_events(&events)?;
        }
        print_events(&events, template, &mut unknown, out)?;
        print_totals(&events, out)?;
        writeln!(out, "join mismatches: {}", log.join_mismatches())?;
        writeln!(out, "turns of other sessions: {elsewhere}")?;
        writeln!(out, "unknown turns: {}", log.unknown_turns())?;
    }
    if let Some(store) = store {
        store.mark_tailed(tree.mission_id())?;
    }
    writeln!(out, "unknown events: {unknown}")?;
    Ok(())
}

/// Whether `file` is the spool of a hosted mission (its first line is a JSON-RPC envelope).
fn is_spool(file: &Path) -> Result<bool, Box<dyn Error>> {
    let mut head = Vec::new();
    let mut reader =
        BufReader::new(File::open(file).map_err(|err| format!("{}: {err}", file.display()))?);
    reader.read_until(b'\n', &mut head)?;
    Ok(head.starts_with(SPOOL_FIRST_KEY.as_bytes()))
}

/// The hosted run's events through the RPC source, the way the app maps them live. The launch
/// id is the spool's file stem. Protocol chatter is counted on `warnings`, not dropped.
fn replay_spool(
    file: &Path,
    cfg: &IngestConfig,
    template: &CycleTemplate,
    store: Option<&mut Store>,
    out: &mut impl Write,
    warnings: &mut impl Write,
) -> Result<(), Box<dyn Error>> {
    let launch_id = file
        .file_stem()
        .and_then(|stem| stem.to_str())
        .ok_or("the spool has no file name")?;
    let mut source = RpcSource::open(file, cfg)?;
    let mut mapper = RpcMapper::with_template(launch_id, MappingVersion::LATEST, template.clone())
        .with_session_files(cfg);
    let mut unknown = 0_u64;
    let mut all = Vec::new();
    loop {
        let batch = source.next_batch()?;
        let at_eof = source.at_eof();
        let records: Vec<RawRecord> = if at_eof {
            batch.into_iter().chain(source.drain_partial()).collect()
        } else {
            batch
        };
        let mut events: Vec<Event> = records
            .iter()
            .flat_map(|record| mapper.map(record))
            .collect();
        if at_eof {
            events.extend(mapper.finish());
        }
        print_events(&events, template, &mut unknown, out)?;
        all.extend(events);
        if at_eof {
            break;
        }
    }
    if let (Some(store), Some(session), Some(first)) = (store, mapper.session_id(), all.first()) {
        store.record_hosted(&HostedRun {
            mission_name: launch_id.to_owned(),
            session: session.clone(),
            spool: file.to_owned(),
            droid_version: None,
            started_at: first.at.to_rfc3339_millis(),
        })?;
        store.insert_events(&all)?;
    }
    let chatter: Vec<String> = mapper
        .protocol_counts()
        .iter()
        .map(|(kind, count)| format!("{kind}={count}"))
        .collect();
    if !chatter.is_empty() {
        writeln!(
            warnings,
            "observatory-cli: protocol chatter: {}",
            chatter.join(" ")
        )?;
    }
    writeln!(out, "unknown events: {unknown}")?;
    Ok(())
}

/// `totals\t<role>\t<turns>\t<input>\t<output>\t<cache_read>` per role that has a turn.
fn print_totals(events: &[Event], out: &mut impl Write) -> Result<(), Box<dyn Error>> {
    for role in Role::ALL {
        let (mut turns, mut input, mut output, mut cache_read) = (0_u64, 0_u64, 0_u64, 0_u64);
        for event in events.iter().filter(|event| event.role == role) {
            if let EventKind::TurnUsage {
                input: i,
                output: o,
                cache_read: c,
                ..
            } = &event.kind
            {
                turns += 1;
                input += i;
                output += o;
                cache_read += c;
            }
        }
        if turns > 0 {
            writeln!(
                out,
                "totals\t{}\t{turns}\t{input}\t{output}\t{cache_read}",
                role.as_str()
            )?;
        }
    }
    Ok(())
}

fn print_events(
    events: &[Event],
    template: &CycleTemplate,
    unknown: &mut u64,
    out: &mut impl Write,
) -> Result<(), Box<dyn Error>> {
    for event in events {
        *unknown += u64::from(matches!(event.kind, EventKind::Unknown { .. }));
        writeln!(out, "{}", event_line(event, template))?;
    }
    Ok(())
}

fn event_line(event: &Event, template: &CycleTemplate) -> String {
    let kind = match &event.kind {
        EventKind::Unknown { raw_type } => format!("{}({raw_type})", event.kind.prd_name()),
        known => known.prd_name().into_owned(),
    };
    let session = event
        .session_id
        .as_ref()
        .map_or_else(String::new, ToString::to_string);
    format!(
        "{}\t{}\t{kind}\t{session}",
        event.at.to_rfc3339_millis(),
        template.lane_text(&event.lane)
    )
}

enum Mapping {
    Root(Box<Mapper>, SessionId),
    Tree(TreeMapper),
}

impl Mapping {
    fn map(&mut self, record: &RawRecord) -> Vec<Event> {
        match self {
            Self::Root(mapper, _) => mapper.map(record),
            Self::Tree(mapper) => mapper.map(record),
        }
    }

    fn finish(&mut self) -> Vec<Event> {
        match self {
            Self::Root(mapper, _) => mapper.finish(),
            Self::Tree(mapper) => mapper.finish(),
        }
    }

    /// `None` for a turn with no session or of a session this replay does not hold.
    fn map_usage(&mut self, turn: &UsageTurn) -> Option<Event> {
        match self {
            Self::Root(mapper, root) => {
                (turn.session_id.as_ref() == Some(root)).then(|| mapper.map_usage(turn))
            }
            Self::Tree(mapper) => mapper.map_usage(turn),
        }
    }
}
