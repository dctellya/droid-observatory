//! What the hosted run shows the human: the first message the orchestrator reads and the state
//! of the run as controls are acknowledged.

use observatory_core::ingest::rpc::{Ack, AckOutcome, method};
use observatory_core::view::{HostedState, HostedStatus, LaunchConfig, PerRole};

fn config(budget: Option<f64>) -> LaunchConfig {
    let same = |text: &str| PerRole {
        orchestrator: text.to_owned(),
        worker: text.to_owned(),
        validator: text.to_owned(),
    };
    LaunchConfig {
        repo: "/r".into(),
        mission_file: "/r/mission.md".into(),
        branch: "mission/tiny".into(),
        models: same("m"),
        efforts: same("high"),
        autonomy: "high".into(),
        budget_usd: budget,
    }
}

fn ack(id: &str, method: &str, outcome: AckOutcome) -> Ack {
    Ack {
        id: id.into(),
        method: method.into(),
        outcome,
    }
}

#[test]
fn the_first_message_is_the_mission_file_with_the_branch_name() {
    let message = config(None).first_message("Build a tiny thing.\n");

    assert!(message.starts_with("Build a tiny thing."), "{message}");
    assert!(message.contains("mission/tiny"), "{message}");
    assert!(!message.contains('$'), "no budget was given: {message}");
}

#[test]
fn the_first_message_states_the_budget_when_one_is_given() {
    let message = config(Some(2.5)).first_message("Build.");

    assert!(message.contains("$2.50"), "{message}");
}

#[test]
fn a_run_is_running_once_the_session_is_initialised_and_lists_every_ack() {
    let mut status = HostedStatus::starting("l1");
    assert_eq!(status.state, HostedState::Starting);

    status.record_ack(&ack("1", method::INITIALIZE_SESSION, AckOutcome::Accepted));
    status.record_ack(&ack(
        "2",
        method::INTERRUPT_SESSION,
        AckOutcome::Rejected {
            code: -32602,
            message: "nothing to interrupt".into(),
        },
    ));

    assert_eq!(status.state, HostedState::Running);
    let rows: Vec<(&str, bool)> = status
        .acks
        .iter()
        .map(|row| (row.request_id.as_str(), row.accepted))
        .collect();
    assert_eq!(rows, [("1", true), ("2", false)]);
    assert_eq!(
        status.acks[1].message.as_deref(),
        Some("nothing to interrupt")
    );
}

#[test]
fn a_rejected_initialisation_does_not_start_the_run() {
    let mut status = HostedStatus::starting("l1");

    status.record_ack(&ack(
        "1",
        method::INITIALIZE_SESSION,
        AckOutcome::Rejected {
            code: -1,
            message: "bad".into(),
        },
    ));

    assert_eq!(status.state, HostedState::Starting);
}

#[test]
fn an_exited_run_keeps_its_exit_code() {
    let mut status = HostedStatus::starting("l1");

    status.exited(Some(0));

    assert_eq!(status.state, HostedState::Exited { code: Some(0) });
}
