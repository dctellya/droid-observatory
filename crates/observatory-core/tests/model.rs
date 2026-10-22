//! Canonical event model: lanes, roles, events and their wire form (architecture §3).

use std::path::PathBuf;
use std::time::Duration;

use observatory_core::mapping::MappingVersion;
use observatory_core::model::{
    Event, EventId, EventKind, FeatureId, Lane, MilestoneId, MissionId, ModelId, Role, SessionId,
    SourcePtr, Timestamp, TokenUsage,
};

#[test]
fn lanes_are_nine_in_prd_order() {
    let names: Vec<&str> = Lane::ALL.iter().map(|lane| lane.name()).collect();
    assert_eq!(
        names,
        [
            "Intent",
            "Orchestrator",
            "Workers",
            "Validators, scrutiny",
            "Validators, user testing",
            "Integration",
            "Human",
            "Outside",
            "Time, Waste, Cost and models",
        ]
    );

    let numbers: Vec<u8> = Lane::ALL.iter().map(|lane| lane.number()).collect();
    assert_eq!(numbers, [1, 2, 3, 4, 5, 6, 7, 8, 9]);

    assert_eq!(Lane::for_role(Role::Orchestrator), Lane::Orchestrator);
    assert_eq!(Lane::for_role(Role::Worker), Lane::Workers);
    assert_eq!(Lane::for_role(Role::ScrutinyValidator), Lane::Scrutiny);
    assert_eq!(
        Lane::for_role(Role::UserTestingValidator),
        Lane::UserTesting
    );
    assert_eq!(Lane::for_role(Role::Human), Lane::Human);
    assert_eq!(Lane::for_role(Role::External), Lane::Outside);
    assert_eq!(Lane::for_role(Role::Unresolved), Lane::Workers);
}

fn timestamp() -> Timestamp {
    Timestamp::parse("2026-10-22T14:54:47.240Z").expect("valid RFC 3339")
}

fn jsonl_pointer() -> SourcePtr {
    SourcePtr::Jsonl {
        path: PathBuf::from("fixtures/streams/2026-10-22-observatory-v1-s.jsonl"),
        byte_offset: 1_204,
        byte_len: 311,
    }
}

fn event_with(source: SourcePtr, kind: EventKind) -> Event {
    Event {
        id: EventId("0f2c:1204:0".to_owned()),
        mission_id: MissionId("e194b9e3".to_owned()),
        milestone_id: Some(MilestoneId("m0".to_owned())),
        feature_id: Some(FeatureId("s-core-event-model".to_owned())),
        lane: Lane::Workers,
        kind,
        role: Role::Worker,
        session_id: Some(SessionId("0f2c".to_owned())),
        model: Some(ModelId("claude-sonnet-5-5".to_owned())),
        at: timestamp(),
        duration: Some(Duration::from_millis(1_500)),
        usage: Some(TokenUsage {
            input: 10,
            output: 20,
            cache_read: 30,
            cache_write: 40,
            credits: Some(0.5),
        }),
        source,
        mapping_version: MappingVersion::LATEST,
    }
}

#[test]
fn event_round_trips_through_serde() {
    let pointers = [
        jsonl_pointer(),
        SourcePtr::Rpc {
            path: PathBuf::from("spool/host.jsonl"),
            byte_offset: 0,
            byte_len: 12,
        },
        SourcePtr::Git {
            repo: PathBuf::from("/work/repo"),
            sha: "7f64f5a".to_owned(),
        },
        SourcePtr::GitHub {
            url: "https://api.github.com/repos/o/r/pulls/1".to_owned(),
        },
        SourcePtr::App { action_id: 7 },
    ];

    for source in pointers {
        let event = event_with(source, EventKind::FindingBlocking);
        let json = serde_json::to_string(&event).expect("event serializes");
        let back: Event = serde_json::from_str(&json).expect("event deserializes");
        assert_eq!(back, event, "wire form: {json}");
    }
}

#[test]
fn event_wire_form_is_snake_case_with_milliseconds() {
    let event = event_with(jsonl_pointer(), EventKind::GateCommandRun);
    let value = serde_json::to_value(&event).expect("event serializes");

    assert_eq!(value["kind"], "gate_command_run");
    assert_eq!(value["lane"], "workers");
    assert_eq!(value["role"], "worker");
    assert_eq!(value["at"], "2026-10-22T14:54:47.240Z");
    assert_eq!(value["duration_ms"], 1_500);
    assert_eq!(value["source"]["source"], "jsonl");
    assert_eq!(value["source"]["byte_offset"], 1_204);
    assert_eq!(value["source"]["byte_len"], 311);
}

#[test]
fn event_rejects_unknown_fields() {
    let event = event_with(jsonl_pointer(), EventKind::GateCommandRun);
    let mut value = serde_json::to_value(&event).expect("event serializes");
    value["surprise"] = serde_json::json!(true);
    assert!(serde_json::from_value::<Event>(value).is_err());
}

#[test]
fn every_event_carries_the_mapping_version() {
    let pointers = [
        jsonl_pointer(),
        SourcePtr::Git {
            repo: PathBuf::from("/work/repo"),
            sha: "7f64f5a".to_owned(),
        },
        SourcePtr::App { action_id: 7 },
    ];

    for source in pointers {
        let event = event_with(source, EventKind::TaskDone);
        assert_eq!(event.mapping_version, MappingVersion::LATEST);

        let mut value = serde_json::to_value(&event).expect("event serializes");
        assert_eq!(value["mapping_version"], "v1");

        value
            .as_object_mut()
            .expect("event is an object")
            .remove("mapping_version");
        assert!(
            serde_json::from_value::<Event>(value).is_err(),
            "an event without mapping_version must not deserialize"
        );
    }
}

#[test]
fn unknown_event_keeps_raw_type_and_byte_offset() {
    let kind = EventKind::Unknown {
        raw_type: "todo_state".to_owned(),
    };
    assert_eq!(kind.prd_name(), "unknown.event");
    assert_eq!(kind.lane(), None);

    let mut event = event_with(jsonl_pointer(), kind);
    event.role = Role::Unresolved;
    event.lane = Lane::for_role(event.role);

    let json = serde_json::to_value(&event).expect("event serializes");
    assert_eq!(
        json["kind"],
        serde_json::json!({"unknown": {"raw_type": "todo_state"}})
    );
    assert_eq!(json["source"]["byte_offset"], 1_204);

    let back: Event = serde_json::from_value(json).expect("event deserializes");
    assert_eq!(
        back.kind,
        EventKind::Unknown {
            raw_type: "todo_state".to_owned()
        }
    );
    assert_eq!(back.source, jsonl_pointer());
}
