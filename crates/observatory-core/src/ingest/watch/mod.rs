//! Attach: watch `[ingest].factory_sessions_dir`, tail growing session files by byte offset
//! and map their lines as they arrive (architecture §4.3, FR-L1, FR-L2).

mod attach;
#[cfg(feature = "store")]
mod live;
mod scan;
mod watcher;

pub use attach::{Attach, AttachBatch, UsageCounters, WatchedSession};
#[cfg(feature = "store")]
pub use live::{LiveIngest, LiveStep};
pub use watcher::{DiscoveredSession, SessionWatcher, session_name};
