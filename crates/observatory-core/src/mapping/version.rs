use std::fmt;

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
        if is_known_series(droid_version) {
            MappingVersion::V1
        } else {
            MappingVersion::LATEST
        }
    }
}

/// The Droid release series whose session files the v1 rules were written against.
const V1_DROID_SERIES: &str = "0.247";

impl fmt::Display for MappingVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MappingVersion::V1 => f.write_str("v1"),
        }
    }
}

/// Something to tell the user once about how a capture is being read. The mapping goes on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Warning {
    /// The capture's Droid version has no mapping of its own. `seen` is `None` when the capture
    /// names no version at all.
    UnknownDroidVersion {
        seen: Option<String>,
        used: MappingVersion,
    },
}

impl fmt::Display for Warning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Warning::UnknownDroidVersion {
                seen: Some(seen),
                used,
            } => write!(f, "unknown Droid version {seen}: mapping with {used}"),
            Warning::UnknownDroidVersion { seen: None, used } => {
                write!(
                    f,
                    "no Droid version in the capture header: mapping with {used}"
                )
            }
        }
    }
}

/// The mapping version for the Droid version a capture header names, and the warning to show
/// once when that version has no mapping of its own (`LATEST` is used then).
pub fn select_version(droid_version: Option<&str>) -> (MappingVersion, Option<Warning>) {
    match droid_version {
        Some(seen) if is_known_series(seen) => (MappingVersion::for_droid_version(seen), None),
        _ => (
            MappingVersion::LATEST,
            Some(Warning::UnknownDroidVersion {
                seen: droid_version.map(str::to_owned),
                used: MappingVersion::LATEST,
            }),
        ),
    }
}

fn is_known_series(droid_version: &str) -> bool {
    let seen = droid_version.trim();
    seen == V1_DROID_SERIES
        || seen
            .strip_prefix(V1_DROID_SERIES)
            .is_some_and(|rest| rest.starts_with('.'))
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
