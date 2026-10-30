use std::path::{Path, PathBuf};

use crate::config::IngestConfig;
use crate::ingest::{IngestError, JsonlSource, RawRecord, Source, SourceKind};
use crate::model::{SourcePtr, Timestamp};

/// The tail of a spool: the same reader as a session file (incomplete last line held back,
/// pointers by byte offset), with pointers that say `rpc` so a stored event names the spool.
pub struct RpcSource {
    inner: JsonlSource,
    spool: PathBuf,
}

impl RpcSource {
    pub fn open(spool: &Path, cfg: &IngestConfig) -> Result<Self, IngestError> {
        Ok(Self {
            inner: JsonlSource::open(spool, cfg)?,
            spool: spool.to_owned(),
        })
    }

    pub fn spool(&self) -> &Path {
        &self.spool
    }

    /// The unfinished last line as a record: for a process that has exited.
    pub fn drain_partial(&mut self) -> Vec<RawRecord> {
        self.inner.drain_partial().into_iter().map(rpc).collect()
    }
}

fn rpc(mut record: RawRecord) -> RawRecord {
    record.arrived_at = Some(Timestamp(time::OffsetDateTime::now_utc()));
    if let SourcePtr::Jsonl {
        path,
        byte_offset,
        byte_len,
    } = record.ptr
    {
        record.ptr = SourcePtr::Rpc {
            path,
            byte_offset,
            byte_len,
        };
    }
    record
}

impl Source for RpcSource {
    fn kind(&self) -> SourceKind {
        SourceKind::Rpc
    }

    fn next_batch(&mut self) -> Result<Vec<RawRecord>, IngestError> {
        Ok(self.inner.next_batch()?.into_iter().map(rpc).collect())
    }

    fn at_eof(&self) -> bool {
        self.inner.at_eof()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingest::rpc::{Spool, spool_path};

    #[test]
    fn a_growing_spool_yields_rpc_pointers_and_holds_back_a_partial_line() {
        let dir = tempfile::tempdir().unwrap();
        let path = spool_path(dir.path(), "l1");
        let mut spool = Spool::create(&path).unwrap();
        let mut source = RpcSource::open(&path, &IngestConfig::default()).unwrap();
        assert_eq!(source.kind(), SourceKind::Rpc);
        assert!(source.next_batch().unwrap().is_empty());

        spool.append_line(b"{\"a\":1}").unwrap();
        let first = source.next_batch().unwrap();
        assert_eq!(first.len(), 1);
        assert_eq!(
            first[0].ptr,
            SourcePtr::Rpc {
                path: path.clone(),
                byte_offset: 0,
                byte_len: 7
            }
        );

        spool.copy_from(&b"{\"b\":2}"[..]).unwrap();
        assert!(source.next_batch().unwrap().is_empty());
        assert!(source.at_eof());
        let rest = source.drain_partial();
        assert_eq!(rest.len(), 1);
        assert_eq!(rest[0].bytes, b"{\"b\":2}");
        assert!(matches!(rest[0].ptr, SourcePtr::Rpc { byte_offset: 8, .. }));
        assert_eq!(source.spool(), path);
    }
}
