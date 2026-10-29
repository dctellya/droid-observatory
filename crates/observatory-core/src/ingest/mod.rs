//! Readers that turn files into raw records (architecture §4). Blocking IO only.

mod capture;
pub mod forge;
pub mod git;
#[cfg(feature = "github")]
pub mod github;
mod jsonl;
mod live;
mod memory;
#[cfg(feature = "rpc")]
pub mod rpc;
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
pub use live::{
    AttachStatus, EventBatch, EventBatcher, OpenedMission, Retraction, SessionRow, drop_events,
    merge_ordered, retract_stale,
};
pub use memory::{
    MemoryReplay, SessionInfo, order_key, replay_files, replay_files_with, session_infos,
};
pub use sessions::{Session, SessionTree, display_name, feature_id_in, feature_id_of, user_tag};
pub use sidecar::{Sidecar, SidecarTag, SidecarUsage, read_sidecar, sidecar_path};
pub use source::{IngestError, RawRecord, Source, SourceKind};
pub use usage_log::{UsageLogSource, UsageTurn};
