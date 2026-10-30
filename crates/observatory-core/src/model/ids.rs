use std::fmt;

use serde::{Deserialize, Serialize};

macro_rules! string_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub String);

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

string_id! {
    /// `<session_id>:<byte_offset>:<sub>`, derived from the source pointer so ids are
    /// deterministic without a hash. `<sub>` is the 0-based index when one raw line
    /// yields several events.
    EventId
}
string_id! {
    /// Droid session uuid (the file stem).
    SessionId
}
string_id! {
    /// The `SessionId` of the root session (the one without `callingSessionId`).
    MissionId
}
string_id! {
    /// Opaque, assigned by the mapping (for example `m0`).
    MilestoneId
}
string_id! {
    /// Opaque, assigned by the mapping.
    FeatureId
}
string_id! {
    /// For example `claude-sonnet-5-5`.
    ModelId
}
