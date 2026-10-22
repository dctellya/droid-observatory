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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn droid_version_selects_the_mapping_version() {
        assert_eq!(
            MappingVersion::for_droid_version("0.247.0"),
            MappingVersion::V1
        );
        assert_eq!(
            MappingVersion::for_droid_version("0.247.3"),
            MappingVersion::V1
        );
    }

    #[test]
    fn unknown_or_empty_droid_version_falls_back_to_latest() {
        for seen in ["", "  ", "9.9.9", "not a version", "0.246.0"] {
            assert_eq!(
                MappingVersion::for_droid_version(seen),
                MappingVersion::LATEST,
                "{seen:?}"
            );
        }
    }
}
