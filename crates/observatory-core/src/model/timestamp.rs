use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use time::format_description::FormatItem;
use time::format_description::well_known::Rfc3339;
use time::macros::format_description;
use time::{OffsetDateTime, UtcOffset};

/// Wire format: RFC 3339, UTC, exactly three fractional digits (`2026-10-22T14:54:47.240Z`).
const WIRE: &[FormatItem<'static>] =
    format_description!("[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:3]Z");

/// Instant of an event. Serialized with millisecond precision; finer precision is dropped
/// on the wire.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Timestamp(pub OffsetDateTime);

impl Timestamp {
    /// Parses any RFC 3339 instant and normalises it to UTC.
    pub fn parse(text: &str) -> Result<Self, time::error::Parse> {
        OffsetDateTime::parse(text, &Rfc3339).map(|at| Self(at.to_offset(UtcOffset::UTC)))
    }

    pub fn now() -> Self {
        Self(OffsetDateTime::now_utc())
    }

    pub fn to_rfc3339_millis(&self) -> String {
        self.0
            .to_offset(UtcOffset::UTC)
            .format(WIRE)
            .expect("the wire format has no component that can fail for a valid instant")
    }
}

impl Serialize for Timestamp {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_rfc3339_millis())
    }
}

impl<'de> Deserialize<'de> for Timestamp {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Self::parse(&text).map_err(D::Error::custom)
    }
}
