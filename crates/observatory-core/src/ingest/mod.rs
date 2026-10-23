//! Readers that turn files into raw records (architecture §4). Blocking IO only.

mod capture;
mod jsonl;
mod sessions;
mod sidecar;
mod source;

pub use capture::{CaptureHeader, capture_path, mapping_version_of, read_capture};
pub use jsonl::JsonlSource;
pub use sessions::{Session, SessionTree};
pub use sidecar::{Sidecar, SidecarTag, SidecarUsage, read_sidecar, sidecar_path};
pub use source::{IngestError, RawRecord, Source, SourceKind};
