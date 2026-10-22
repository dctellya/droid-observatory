use serde::{Deserialize, Serialize};

/// Token counts of one usage report. `credits` is the settings `factoryCredits` value
/// when present.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_write: u64,
    pub credits: Option<f64>,
}
