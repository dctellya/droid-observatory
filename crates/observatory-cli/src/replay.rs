//! `replay <file>`: attach that stops at the end of the file (D-008).

use std::error::Error;
use std::io::Write;
use std::path::Path;

use observatory_core::config::IngestConfig;
use observatory_core::ingest::{JsonlSource, Source, mapping_version_of};
use observatory_core::mapping::{Mapper, SessionMeta};
use observatory_core::model::{Event, EventKind};

/// Writes `<time>\t<lane>\t<kind>\t<session>` per event, then `unknown events: N`.
/// Only reads `file`; it writes to `out` and nowhere else.
pub fn replay(file: &Path, cfg: &IngestConfig, out: &mut impl Write) -> Result<(), Box<dyn Error>> {
    let mut source = JsonlSource::open(file, cfg)?;
    let version = mapping_version_of(file)?;
    let mut mapper = Mapper::new(SessionMeta::root(source.root_session_id().clone()), version);

    let mut unknown = 0_u64;
    loop {
        let batch = source.next_batch()?;
        let at_eof = source.at_eof();
        let records = if at_eof {
            batch.into_iter().chain(source.drain_partial()).collect()
        } else {
            batch
        };
        for record in &records {
            for event in mapper.map(record) {
                unknown += u64::from(matches!(event.kind, EventKind::Unknown { .. }));
                writeln!(out, "{}", event_line(&event))?;
            }
        }
        if at_eof {
            break;
        }
    }
    writeln!(out, "unknown events: {unknown}")?;
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
