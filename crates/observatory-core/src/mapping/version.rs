use serde::{Deserialize, Serialize};

/// Version of the rules that turned a raw record into an `Event`. Stored on every event so
/// a change of rules is visible in old data.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MappingVersion {
    V1,
}

impl MappingVersion {
    pub const LATEST: MappingVersion = MappingVersion::V1;
}
