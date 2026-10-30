//! Attach: watch `[ingest].factory_sessions_dir`, tail growing session files by byte offset
//! and map their lines as they arrive (architecture §4.3, FR-L1, FR-L2).

mod attach;
#[cfg(feature = "store")]
mod live;
#[cfg(feature = "store")]
mod names;
mod scan;
mod watcher;

pub use attach::{Attach, AttachBatch, UsageCounters, WatchedSession};
#[cfg(feature = "store")]
pub use live::{LiveIngest, LiveStep};
#[cfg(feature = "store")]
pub use names::{NameChange, RenameRefused, SessionNames, SessionRename};
pub use watcher::{DiscoveredSession, SessionWatcher, session_name};
