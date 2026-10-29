//! Synchronous core of droid-observatory: no async runtime, no UI (architecture I1, I2).

pub mod config;
pub mod control;
pub mod ingest;
pub mod mapping;
pub mod metrics;
pub mod model;
#[cfg(feature = "secrets")]
pub mod secrets;
#[cfg(feature = "store")]
pub mod store;
pub mod view;
