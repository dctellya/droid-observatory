//! `inspect <event-id>`: the raw line an event was mapped from, read back by its pointer. The
//! pointer of a `turn.usage` event names a line of Droid's own log, which is not printed:
//! only the fields parsed from it are.

use std::error::Error;
use std::io::Write;
use std::path::Path;

use observatory_core::ingest::JsonlSource;
use observatory_core::ingest::usage_log::{LogLine, parse_line};
use observatory_core::model::{EventId, SourcePtr};
use observatory_core::store::Store;

/// Writes the line the stored pointer of `id` names, byte for byte and without a newline, so
/// the output compares equal to the file slice. For a `turn.usage` event it writes the parsed
/// row instead (`session`, `role`, `lane`, `log_ts`, `model` and the five token counts, one
/// `name: value` per line). The store is opened read-only; the files are only read.
pub fn inspect(store: &Path, id: &str, out: &mut impl Write) -> Result<(), Box<dyn Error>> {
    let store = Store::open_existing(store)?;
    let line = match store.pointer(&EventId(id.to_owned()))? {
        SourcePtr::Jsonl {
            path,
            byte_offset,
            byte_len,
        }
        | SourcePtr::Rpc {
            path,
            byte_offset,
            byte_len,
        } => JsonlSource::read_at(&path, byte_offset, byte_len)?,
        SourcePtr::UsageLog {
            path,
            byte_offset,
            byte_len,
        } => {
            let line = JsonlSource::read_at(&path, byte_offset, byte_len)?;
            let LogLine::Turn(turn) = parse_line(&line) else {
                return Err(format!(
                    "event {id}: {} at byte_offset={byte_offset} byte_len={byte_len} is no longer a turn line",
                    path.display()
                )
                .into());
            };
            let event = store.event(&EventId(id.to_owned()))?;
            let session = event
                .session_id
                .map_or_else(String::new, |id| id.to_string());
            let model = event.model.map_or_else(String::new, |id| id.to_string());
            writeln!(out, "session: {session}")?;
            writeln!(out, "role: {}", event.role.as_str())?;
            writeln!(out, "lane: {} {}", event.lane.number(), event.lane.name())?;
            writeln!(out, "log_ts: {}", turn.log_ts.to_rfc3339_millis())?;
            writeln!(out, "model: {model}")?;
            writeln!(out, "input: {}", turn.input)?;
            writeln!(out, "output: {}", turn.output)?;
            writeln!(out, "cache_read: {}", turn.cache_read)?;
            writeln!(out, "reasoning: {}", turn.reasoning)?;
            writeln!(out, "total_input: {}", turn.total_input)?;
            return Ok(());
        }
        other => return Err(format!("event {id} has no raw line: its source is {other:?}").into()),
    };
    out.write_all(&line)?;
    Ok(())
}
