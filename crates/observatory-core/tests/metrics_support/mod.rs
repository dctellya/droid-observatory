//! Events for the metrics tests: a base instant, a builder, and the M0 golden events.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

use observatory_core::mapping::MappingVersion;
use observatory_core::model::{
    CycleTemplate, Event, EventId, EventKind, FeatureId, MilestoneId, MissionId, Role, SessionId,
    SourcePtr, Timestamp,
};
use time::{Duration as Offset, OffsetDateTime};

pub const MISSION: &str = "mission-1";

/// `seconds` after 2026-10-30T10:00:00Z.
pub fn t(seconds: i64) -> Timestamp {
    let base = Timestamp::parse("2026-10-30T10:00:00.000Z").expect("valid time");
    Timestamp(base.0 + Offset::seconds(seconds))
}

pub fn secs(from: Timestamp, to: Timestamp) -> i64 {
    (to.0 - from.0).whole_seconds()
}

/// A worker event of the default template's lane for its kind, no feature, no duration.
pub fn ev(id: &str, kind: EventKind, seconds: i64) -> Event {
    let template = CycleTemplate::engineering();
    let role = Role::Worker;
    Event {
        id: EventId(id.to_owned()),
        mission_id: MissionId(MISSION.to_owned()),
        milestone_id: None,
        feature_id: None,
        lane: template.lane_of(&kind, role),
        overlay: kind.overlay(),
        kind,
        role,
        session_id: Some(SessionId(MISSION.to_owned())),
        model: None,
        at: t(seconds),
        duration: None,
        usage: None,
        source: SourcePtr::Rpc {
            path: PathBuf::from("spool.jsonl"),
            byte_offset: 0,
            byte_len: 1,
        },
        mapping_version: MappingVersion::LATEST,
    }
}

pub trait EventExt: Sized {
    fn feature(self, feature: &str) -> Self;
    fn milestone(self, milestone: &str) -> Self;
    fn by(self, role: Role) -> Self;
    /// The span the mapper measured, ending at the event's `at`.
    fn lasting(self, seconds: u64) -> Self;
}

impl EventExt for Event {
    fn feature(mut self, feature: &str) -> Self {
        self.feature_id = Some(FeatureId(feature.to_owned()));
        self
    }

    fn milestone(mut self, milestone: &str) -> Self {
        self.milestone_id = Some(MilestoneId(milestone.to_owned()));
        self
    }

    fn by(mut self, role: Role) -> Self {
        let template = CycleTemplate::engineering();
        self.lane = template.lane_of(&self.kind, role);
        self.role = role;
        self
    }

    fn lasting(mut self, seconds: u64) -> Self {
        self.duration = Some(std::time::Duration::from_secs(seconds));
        self
    }
}

pub fn m0_events() -> Vec<Event> {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/streams/2026-10-22-observatory-v1-m0.events.json");
    let text = std::fs::read_to_string(path).expect("the M0 golden events exist");
    serde_json::from_str(&text).expect("the M0 golden events parse")
}

pub fn instant(epoch_seconds: i64) -> Timestamp {
    Timestamp(OffsetDateTime::from_unix_timestamp(epoch_seconds).expect("valid epoch"))
}
