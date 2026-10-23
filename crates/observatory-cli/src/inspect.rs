//! `inspect <event-id>`: the raw line an event was mapped from, read back by its pointer.

use std::error::Error;
use std::io::Write;
use std::path::Path;

use observatory_core::ingest::JsonlSource;
use observatory_core::model::{EventId, SourcePtr};
use observatory_core::store::Store;

/// Writes the line the stored pointer of `id` names, byte for byte and without a newline, so
/// the output compares equal to the file slice. The store is opened read-only; the session
/// file is only read.
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
        other => return Err(format!("event {id} has no raw line: its source is {other:?}").into()),
    };
    out.write_all(&line)?;
    Ok(())
}
