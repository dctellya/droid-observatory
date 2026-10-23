//! Readers that turn files into raw records (architecture §4). Blocking IO only.

mod capture;
mod jsonl;
mod memory;
mod sessions;
mod sidecar;
mod source;
#[cfg(feature = "watch")]
pub mod watch;

pub use capture::{
    CaptureHeader, capture_path, mapping_version_of, read_capture, select_mapping_of,
};
pub use jsonl::JsonlSource;
pub use memory::{MemoryReplay, SessionInfo, replay_files};
pub use sessions::{Session, SessionTree};
pub use sidecar::{Sidecar, SidecarTag, SidecarUsage, read_sidecar, sidecar_path};
pub use source::{IngestError, RawRecord, Source, SourceKind};
