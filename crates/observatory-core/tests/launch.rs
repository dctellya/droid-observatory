//! The launch form (FR-L3): defaults come from `[mission]`, required fields refuse an empty
//! Launch by name, the budget is optional, and a valid form becomes the `droid exec` command.

use observatory_core::config::MissionConfig;
use observatory_core::view::{LaunchConfig, LaunchField, LaunchForm, session_uuid};

fn filled() -> LaunchForm {
    let mut form = LaunchForm::defaults(&MissionConfig::default());
    form.repo = "/work/repo".into();
    form.mission_file = "/work/repo/mission.md".into();
    form.branch = "mission/hello".into();
    form
}

fn refused(form: &LaunchForm) -> Vec<LaunchField> {
    form.validate()
        .expect_err("the form is refused")
        .into_iter()
        .map(|error| error.field)
        .collect()
}

#[test]
fn launch_form_defaults_come_from_the_mission_config() {
    let mission = MissionConfig::default();

    let form = LaunchForm::defaults(&mission);

    assert_eq!(form.orchestrator_model, mission.orchestrator_model);
    assert_eq!(form.worker_model, mission.worker_model);
    assert_eq!(form.validator_model, mission.validator_model);
    assert_eq!(
        form.orchestrator_effort,
        mission.orchestrator_reasoning_effort
    );
    assert_eq!(form.worker_effort, mission.worker_reasoning_effort);
    assert_eq!(form.validator_effort, mission.validator_reasoning_effort);
    assert_eq!(form.autonomy, mission.autonomy);
    assert!(form.repo.is_empty() && form.mission_file.is_empty() && form.branch.is_empty());
    assert!(form.budget.is_empty());
}

#[test]
fn an_empty_required_field_is_refused_by_name_and_only_that_one() {
    for (field, clear) in [
        (
            LaunchField::Repo,
            (|f: &mut LaunchForm| f.repo.clear()) as fn(&mut LaunchForm),
        ),
        (LaunchField::MissionFile, |f| f.mission_file.clear()),
        (LaunchField::Branch, |f| f.branch.clear()),
    ] {
        let mut form = filled();
        clear(&mut form);

        let errors = form.validate().expect_err("an empty field is refused");

        assert_eq!(errors.len(), 1, "{errors:?}");
        assert_eq!(errors[0].field, field);
        assert!(
            errors[0].message.contains(field.label()),
            "the message names the field: {}",
            errors[0].message
        );
    }
}

#[test]
fn an_empty_form_names_every_required_field() {
    let form = LaunchForm::defaults(&MissionConfig::default());

    assert_eq!(
        refused(&form),
        [
            LaunchField::Repo,
            LaunchField::MissionFile,
            LaunchField::Branch
        ]
    );
}

#[test]
fn a_field_of_spaces_counts_as_empty() {
    let mut form = filled();
    form.branch = "   ".into();
    form.worker_model = "\t".into();

    assert_eq!(
        refused(&form),
        [LaunchField::Branch, LaunchField::WorkerModel]
    );
}

#[test]
fn the_budget_is_optional_and_a_number_when_given() {
    let mut form = filled();
    assert_eq!(form.validate().unwrap().budget_usd, None);

    form.budget = " 12.5 ".into();
    assert_eq!(form.validate().unwrap().budget_usd, Some(12.5));

    for bad in ["abc", "0", "-3", "NaN", "inf"] {
        form.budget = bad.into();
        assert_eq!(refused(&form), [LaunchField::Budget], "{bad}");
    }
}

#[test]
fn only_high_autonomy_can_host_a_mission() {
    let mut form = filled();
    form.autonomy = "low".into();

    assert_eq!(refused(&form), [LaunchField::Autonomy]);
}

#[test]
fn a_valid_form_becomes_a_trimmed_launch_config() {
    let mut form = filled();
    form.repo = "  /work/repo ".into();

    let config = form.validate().unwrap();

    assert_eq!(config.repo, "/work/repo");
    assert_eq!(config.mission_file, "/work/repo/mission.md");
    assert_eq!(config.branch, "mission/hello");
    assert_eq!(config.models.worker, MissionConfig::default().worker_model);
}

#[test]
fn the_command_is_droid_exec_with_both_streams_as_jsonrpc() {
    let config: LaunchConfig = filled().validate().unwrap();

    let args = config.command_args();

    assert_eq!(
        args,
        [
            "exec",
            "--input-format",
            "stream-jsonrpc",
            "--output-format",
            "stream-jsonrpc",
            "--auto",
            "high",
            "--model",
            "claude-fable-5.1",
            "--reasoning-effort",
            "high",
        ]
    );
}

#[test]
fn the_session_uuid_is_version_four_and_follows_the_bytes() {
    let id = session_uuid([0xab; 16]);

    assert_eq!(id, "abababab-abab-4bab-abab-abababababab");
    assert_eq!(id.len(), 36);
}
