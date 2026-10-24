//! Readers that turn files into raw records (architecture §4). Blocking IO only.

mod capture;
mod jsonl;
mod live;
mod memory;
mod sessions;
mod sidecar;
mod source;
pub mod usage_log;
#[cfg(feature = "watch")]
pub mod watch;

pub use capture::{
    CaptureHeader, UsageLogInfo, capture_path, mapping_version_of, read_capture, select_mapping_of,
    usage_log_path,
};
pub use jsonl::JsonlSource;
pub use live::{AttachStatus, EventBatch, EventBatcher, OpenedMission, SessionRow, merge_ordered};
pub use memory::{
    MemoryReplay, SessionInfo, order_key, replay_files, replay_files_with, session_infos,
};
pub use sessions::{Session, SessionTree, display_name, user_tag};
pub use sidecar::{Sidecar, SidecarTag, SidecarUsage, read_sidecar, sidecar_path};
pub use source::{IngestError, RawRecord, Source, SourceKind};
pub use usage_log::{UsageLogSource, UsageTurn};
