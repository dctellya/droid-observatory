//! Canonical event model: lanes, roles, events and their wire form (architecture §3).

use std::path::PathBuf;
use std::time::Duration;

use observatory_core::mapping::MappingVersion;
use observatory_core::model::{
    Event, EventId, EventKind, FeatureId, Lane, MilestoneId, MissionId, ModelId, Overlay, Role,
    SessionId, SourcePtr, Timestamp, TokenUsage,
};

#[test]
fn lanes_are_the_seven_phases_in_order() {
    let names: Vec<&str> = Lane::ALL.iter().map(|lane| lane.name()).collect();
    assert_eq!(
        names,
        [
            "Plan",
            "Contract",
            "Build",
            "Scrutiny",
            "User testing",
            "Integrate",
            "Gate",
        ]
    );

    let numbers: Vec<u8> = Lane::ALL.iter().map(|lane| lane.number()).collect();
    assert_eq!(numbers, [1, 2, 3, 4, 5, 6, 7]);

    assert_eq!(Lane::home(Role::Orchestrator), Lane::Plan);
    assert_eq!(Lane::home(Role::Worker), Lane::Build);
    assert_eq!(Lane::home(Role::ScrutinyValidator), Lane::Scrutiny);
    assert_eq!(Lane::home(Role::UserTestingValidator), Lane::UserTesting);
    assert_eq!(Lane::home(Role::Human), Lane::Plan);
    assert_eq!(Lane::home(Role::External), Lane::Gate);
    assert_eq!(Lane::home(Role::Unresolved), Lane::Build);

    let wire: Vec<String> = Lane::ALL
        .iter()
        .map(|lane| {
            serde_json::to_value(lane)
                .unwrap()
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect();
    assert_eq!(
        wire,
        [
            "plan",
            "contract",
            "build",
            "scrutiny",
            "user_testing",
            "integrate",
            "gate"
        ]
    );
    for lane in Lane::ALL {
        assert_eq!(Lane::from_number(lane.number()), Some(lane));
    }
    assert_eq!(Lane::from_number(0), None);
    assert_eq!(Lane::from_number(8), None);
}

const ROLES: [Role; 7] = [
    Role::Orchestrator,
    Role::Worker,
    Role::ScrutinyValidator,
    Role::UserTestingValidator,
    Role::Human,
    Role::External,
    Role::Unresolved,
];

#[test]
fn role_neutral_kinds_take_the_role_home_phase() {
    let neutral = [
        EventKind::TaskAssigned,
        EventKind::TaskSkipped,
        EventKind::ContextCompacted,
        EventKind::SubagentSpawned,
        EventKind::TaskStarted,
        EventKind::FileTouched,
        EventKind::TestsRun,
        EventKind::TaskDone,
        EventKind::SubagentFinished,
        EventKind::ToolError,
        EventKind::PermissionRequested,
        EventKind::PermissionGranted,
        EventKind::PermissionDenied,
        EventKind::HookBlocked,
        EventKind::UsageReported,
        EventKind::ModelChanged,
        EventKind::Unknown {
            raw_type: "future".to_owned(),
        },
    ];
    for kind in &neutral {
        assert_eq!(kind.fixed_lane(), None, "{}", kind.prd_name());
        for role in ROLES {
            assert_eq!(
                kind.lane(role),
                Lane::home(role),
                "{} {role:?}",
                kind.prd_name()
            );
        }
    }
    // A worker's commit stays in Build; only the orchestrator's moves.
    assert_eq!(EventKind::CommitCreated.lane(Role::Worker), Lane::Build);
}

#[test]
fn fixed_phase_kinds_ignore_the_role() {
    let fixed = [
        (EventKind::IntentCreated, Lane::Plan),
        (EventKind::PlanCreated, Lane::Plan),
        (EventKind::FeatureDefined, Lane::Plan),
        (EventKind::EscalationRaised, Lane::Plan),
        (EventKind::SpecVersioned, Lane::Contract),
        (EventKind::FindingBlocking, Lane::Scrutiny),
        (EventKind::ScrutinyPassed, Lane::Scrutiny),
        (EventKind::QaScenarioPassed, Lane::UserTesting),
        (EventKind::QaEvidence, Lane::UserTesting),
        (EventKind::IntegrationMerged, Lane::Integrate),
        (EventKind::PrOpened, Lane::Integrate),
        (EventKind::ApprovalGiven, Lane::Plan),
        (EventKind::ReviewOutcome, Lane::Gate),
        (EventKind::CheckPassed, Lane::Gate),
        (EventKind::AttestationRecorded, Lane::Gate),
    ];
    for (kind, lane) in fixed {
        for role in ROLES {
            assert_eq!(kind.lane(role), lane, "{} {role:?}", kind.prd_name());
        }
    }
}

#[test]
fn gate_command_lands_in_integrate_for_every_role() {
    for kind in [
        EventKind::GateCommandRun,
        EventKind::GateCommandPassed,
        EventKind::GateCommandFailed,
    ] {
        for role in ROLES {
            assert_eq!(
                kind.lane(role),
                Lane::Integrate,
                "{} {role:?}",
                kind.prd_name()
            );
        }
    }
}

#[test]
fn orchestrator_commits_land_in_integrate() {
    assert_eq!(
        EventKind::CommitCreated.lane(Role::Orchestrator),
        Lane::Integrate
    );
    for role in ROLES.into_iter().filter(|role| *role != Role::Orchestrator) {
        assert_eq!(EventKind::CommitCreated.lane(role), Lane::home(role));
    }
    // Its file edits are not commits: they stay in Plan.
    assert_eq!(EventKind::FileTouched.lane(Role::Orchestrator), Lane::Plan);
}

#[test]
fn human_facing_kinds_carry_the_human_overlay() {
    let human = [
        EventKind::IntentCreated,
        EventKind::IntentAmended,
        EventKind::QuestionAsked,
        EventKind::IntentAnswered,
        EventKind::MissionPaused,
        EventKind::MissionResumed,
        EventKind::PermissionRequested,
        EventKind::PermissionGranted,
        EventKind::PermissionDenied,
        EventKind::ApprovalRequested,
        EventKind::ApprovalGiven,
        EventKind::QuestionAnswered,
        EventKind::ReviewOutcome,
    ];
    for kind in &human {
        assert_eq!(kind.overlay(), Some(Overlay::Human), "{}", kind.prd_name());
    }
    let humans = EventKind::KNOWN
        .iter()
        .filter(|kind| kind.overlay().is_some())
        .count();
    assert_eq!(humans, human.len(), "no other kind is human-facing");
    assert!(
        EventKind::KNOWN
            .iter()
            .all(|kind| kind.overlay() != Some(Overlay::Commander)),
        "Commander has no producer in v1"
    );
    assert_eq!(
        EventKind::Unknown {
            raw_type: "x".to_owned()
        }
        .overlay(),
        None
    );
    // Human-facing kinds are placed by kind, not by the asking person's role.
    assert_eq!(
        EventKind::IntentAmended.lane(Role::Orchestrator),
        Lane::Plan
    );
    assert_eq!(EventKind::QuestionAsked.lane(Role::Worker), Lane::Plan);
    assert_eq!(EventKind::ReviewOutcome.lane(Role::Human), Lane::Gate);
}

#[test]
fn handed_to_cd_carries_merge_sha_pr_number_and_attestation_ids() {
    let kind = EventKind::HandedToCd {
        merge_sha: "7f64f5a".to_owned(),
        pr_number: 41,
        attestation_ids: vec!["att-1".to_owned(), "att-2".to_owned()],
    };
    assert_eq!(kind.prd_name(), "handed_to_cd");
    for role in [Role::Human, Role::External] {
        assert_eq!(kind.lane(role), Lane::Gate);
    }
    assert_eq!(kind.overlay(), None);

    let json = serde_json::to_value(&kind).unwrap();
    assert_eq!(
        json,
        serde_json::json!({"handed_to_cd": {"merge_sha": "7f64f5a", "pr_number": 41,
            "attestation_ids": ["att-1", "att-2"]}})
    );
    let back: EventKind = serde_json::from_value(json).unwrap();
    assert_eq!(back, kind);

    let names: Vec<String> = EventKind::KNOWN
        .iter()
        .map(|kind| kind.prd_name().into_owned())
        .collect();
    assert_eq!(names.len(), 60);
    assert!(names.contains(&"handed_to_cd".to_owned()));
    for removed in [
        "pr.merged_by_human",
        "pr.merged_by_policy",
        "deploy.completed",
    ] {
        assert!(!names.contains(&removed.to_owned()), "{removed}");
    }
}

#[test]
fn overlay_is_serialised_only_when_present() {
    let mut event = event_with(jsonl_pointer(), EventKind::IntentAmended);
    let value = serde_json::to_value(&event).unwrap();
    assert!(value.get("overlay").is_none());

    event.overlay = Some(Overlay::Human);
    let value = serde_json::to_value(&event).unwrap();
    assert_eq!(value["overlay"], "human");
    let back: Event = serde_json::from_value(value).unwrap();
    assert_eq!(back, event);

    let mut without = serde_json::to_value(&event).unwrap();
    without.as_object_mut().unwrap().remove("overlay");
    let back: Event = serde_json::from_value(without).unwrap();
    assert_eq!(back.overlay, None);
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
        lane: Lane::Build,
        overlay: None,
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

    let unknown = EventKind::Unknown {
        raw_type: "future_kind".to_owned(),
    };
    let cases = pointers
        .into_iter()
        .map(|source| (source, EventKind::FindingBlocking))
        .chain([(jsonl_pointer(), unknown)]);

    for (source, kind) in cases {
        let event = event_with(source, kind);
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
    assert_eq!(value["lane"], "build");
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
    assert_eq!(kind.fixed_lane(), None);
    assert_eq!(kind.lane(Role::Unresolved), Lane::Build);

    let mut event = event_with(jsonl_pointer(), kind);
    event.role = Role::Unresolved;
    event.lane = Lane::home(event.role);

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
