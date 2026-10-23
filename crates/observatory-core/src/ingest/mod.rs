//! Readers that turn files into raw records (architecture §4). Blocking IO only.

mod capture;
mod jsonl;
mod source;

pub use capture::{CaptureHeader, capture_path, mapping_version_of, read_capture};
pub use jsonl::JsonlSource;
pub use source::{IngestError, RawRecord, Source, SourceKind};
