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

    /// Picks the rules for the session format of a `droid --version` string. An absent,
    /// empty or unrecognised version gets `LATEST`: showing events with the newest rules
    /// beats showing none.
    pub fn for_droid_version(droid_version: &str) -> MappingVersion {
        let seen = droid_version.trim();
        if seen == V1_DROID_SERIES
            || seen
                .strip_prefix(V1_DROID_SERIES)
                .is_some_and(|rest| rest.starts_with('.'))
        {
            MappingVersion::V1
        } else {
            MappingVersion::LATEST
        }
    }
}

/// The Droid release series whose session files the v1 rules were written against.
const V1_DROID_SERIES: &str = "0.247";

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
