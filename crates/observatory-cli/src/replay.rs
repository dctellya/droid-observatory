//! `replay <file>`: attach that stops at the end of the file (D-008).

use std::error::Error;
use std::io::Write;
use std::path::Path;

use observatory_core::config::IngestConfig;
use observatory_core::ingest::{JsonlSource, RawRecord, SessionTree, Source, select_mapping_of};
use observatory_core::mapping::{Mapper, TreeMapper};
use observatory_core::model::{Event, EventKind};
use observatory_core::store::{CaptureInfo, Store};

/// Writes `<time>\t<lane>\t<kind>\t<session>` per event, then `unknown events: N`.
/// Child sessions found beside `file` or in `<slug>.children/` are replayed with it unless
/// `root_only` is set. Only reads session files; it writes to `out`, and to `warnings` once
/// when the capture's Droid version has no mapping of its own. With a `store`, the sessions
/// and the event pointers are written through to it as they are mapped.
pub fn replay(
    file: &Path,
    root_only: bool,
    cfg: &IngestConfig,
    mut store: Option<&mut Store>,
    out: &mut impl Write,
    warnings: &mut impl Write,
) -> Result<(), Box<dyn Error>> {
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
        let root = Mapper::new(tree.meta(tree.root()), version);
        (source, Mapping::Root(Box::new(root)))
    } else {
        let source = JsonlSource::open_tree(&tree, cfg)?;
        (source, Mapping::Tree(TreeMapper::new(&tree, version)))
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
        print_events(&events, &mut unknown, out)?;
        if at_eof {
            break;
        }
    }
    if let Some(store) = store {
        store.mark_tailed(tree.mission_id())?;
    }
    writeln!(out, "unknown events: {unknown}")?;
    Ok(())
}

fn print_events(
    events: &[Event],
    unknown: &mut u64,
    out: &mut impl Write,
) -> Result<(), Box<dyn Error>> {
    for event in events {
        *unknown += u64::from(matches!(event.kind, EventKind::Unknown { .. }));
        writeln!(out, "{}", event_line(event))?;
    }
    Ok(())
}

fn event_line(event: &Event) -> String {
    let kind = match &event.kind {
        EventKind::Unknown { raw_type } => format!("{}({raw_type})", event.kind.prd_name()),
        known => known.prd_name().into_owned(),
    };
    let session = event
        .session_id
        .as_ref()
        .map_or_else(String::new, ToString::to_string);
    format!(
        "{}\t{} {}\t{kind}\t{session}",
        event.at.to_rfc3339_millis(),
        event.lane.number(),
        event.lane.name()
    )
}

enum Mapping {
    Root(Box<Mapper>),
    Tree(TreeMapper),
}

impl Mapping {
    fn map(&mut self, record: &RawRecord) -> Vec<Event> {
        match self {
            Self::Root(mapper) => mapper.map(record),
            Self::Tree(mapper) => mapper.map(record),
        }
    }

    fn finish(&mut self) -> Vec<Event> {
        match self {
            Self::Root(mapper) => mapper.finish(),
            Self::Tree(mapper) => mapper.finish(),
        }
    }
}
