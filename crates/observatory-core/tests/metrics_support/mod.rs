//! Events for the metrics tests: a base instant, a builder, and the M0 golden events.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

use observatory_core::mapping::MappingVersion;
use observatory_core::metrics::PriceTable;
use observatory_core::model::{
    CycleTemplate, Event, EventId, EventKind, FeatureId, LaneId, MilestoneId, MissionId, ModelId,
    Role, SessionId, SourcePtr, Timestamp, TokenUsage,
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

/// The model a usage report or turn was made with, and the lane an event sits in.
pub trait UsageExt: Sized {
    fn model(self, model: &str) -> Self;
    fn in_lane(self, lane: &str) -> Self;
    fn reporting(self, usage: TokenUsage) -> Self;
}

impl UsageExt for Event {
    fn model(mut self, model: &str) -> Self {
        self.model = Some(ModelId(model.to_owned()));
        self
    }

    fn in_lane(mut self, lane: &str) -> Self {
        self.lane = LaneId::from(lane);
        self
    }

    fn reporting(mut self, usage: TokenUsage) -> Self {
        self.usage = Some(usage);
        self
    }
}

/// One `turn.usage` event of `role` at `seconds`, in lane `lane`, with `[input, output, cache_read]`.
pub fn turn(
    id: &str,
    seconds: i64,
    role: Role,
    model: &str,
    lane: &str,
    counts: [u64; 3],
) -> Event {
    let kind = EventKind::TurnUsage {
        input: counts[0],
        output: counts[1],
        cache_read: counts[2],
        reasoning: 0,
        total_input: counts[0] + counts[2],
        model: Some(ModelId(model.to_owned())),
        log_ts: t(seconds),
    };
    ev(id, kind, seconds).by(role).model(model).in_lane(lane)
}

pub fn prices() -> PriceTable {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../config/prices/2026-10-22.toml");
    let text = std::fs::read_to_string(path).expect("the price table exists");
    PriceTable::parse(&text).expect("the price table parses")
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
