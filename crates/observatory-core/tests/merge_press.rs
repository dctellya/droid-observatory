//! The Merge press (FR-G5, architecture §4.6 and §8): one human-overlay `approval.given` event on
//! the template's final lane and one `human_actions` row per press, the forge asked once, the
//! 403 of a read-only token answered with the exact refusal text and no retry. The lanes come
//! from the default template and are addressed through it.

use std::cell::Cell;

use observatory_core::control::HumanAction;
use observatory_core::control::merge::{MergeState, merge_method, press};
use observatory_core::ingest::forge::{
    Attestation, Check, Forge, ForgeError, MergeMethod, MergePolicy, Merged, Pr, PrState, Review,
    RuleAttribution,
};
use observatory_core::model::{CycleTemplate, EventKind, MissionId, Overlay, Role, SourcePtr};
use observatory_core::model::{SessionId, Timestamp};
use observatory_core::store::{HostedRun, Store};

const PR: u64 = 24;
const TARGET: &str = "demo/base";
const REFUSED: &str = "Merge of PR #24 into demo/base refused: token lacks pull_requests:write. Merge in GitHub; the badge updates from the next poll.";

/// A forge whose merge answers `answer` and counts how often it is asked.
struct Forge1 {
    answer: Result<Merged, ForgeError>,
    asked: Cell<u32>,
}

impl Forge1 {
    fn new(answer: Result<Merged, ForgeError>) -> Self {
        Self {
            answer,
            asked: Cell::new(0),
        }
    }

    fn forbidden() -> Self {
        Self::new(Err(ForgeError::Forbidden {
            pr: PR,
            target: TARGET.to_owned(),
            missing_permission: "pull_requests:write",
        }))
    }
}

impl Forge for Forge1 {
    fn pr(&self, _branch: &str, _target: &str) -> Result<Option<Pr>, ForgeError> {
        Ok(None)
    }
    fn checks(&self, _pr: &Pr) -> Result<Vec<Check>, ForgeError> {
        Ok(Vec::new())
    }
    fn policy(&self, _target: &str) -> Result<MergePolicy, ForgeError> {
        Ok(MergePolicy {
            requires_human: true,
            allowed_methods: vec![MergeMethod::Squash],
            decided_by: RuleAttribution::NoRule,
            required_checks: Vec::new(),
        })
    }
    fn reviews(&self, _pr: &Pr) -> Result<Vec<Review>, ForgeError> {
        Ok(Vec::new())
    }
    fn attestations(&self, _pr: &Pr) -> Result<Vec<Attestation>, ForgeError> {
        Ok(Vec::new())
    }
    fn token_owner(&self) -> Result<Option<String>, ForgeError> {
        Ok(None)
    }
    fn merge(&self, _pr: &Pr, _method: MergeMethod) -> Result<Merged, ForgeError> {
        self.asked.set(self.asked.get() + 1);
        self.answer.clone()
    }
}

fn pr() -> Pr {
    Pr {
        number: PR,
        head_sha: "f4de0fe".to_owned(),
        head: "demo/red-1".to_owned(),
        base: TARGET.to_owned(),
        url: "https://api.github.com/repos/dctellya/droid-observatory/pulls/24".to_owned(),
        created_at: Timestamp::parse("2026-10-29T10:00:00Z").expect("valid time"),
        state: PrState::Open,
        auto_merge: false,
    }
}

fn mission() -> MissionId {
    MissionId("mission-1".to_owned())
}

fn scratch_store(dir: &tempfile::TempDir) -> Store {
    let mut store = Store::open(&dir.path().join("observatory.sqlite")).expect("store opens");
    store
        .record_hosted(&HostedRun {
            mission_name: "hosted".into(),
            session: SessionId("mission-1".to_owned()),
            spool: dir.path().join("hosted.jsonl"),
            droid_version: None,
            started_at: "2026-10-29T09:00:00.000Z".into(),
        })
        .expect("hosted run recorded");
    store
}

fn merge_action() -> HumanAction {
    HumanAction::Merge {
        pr_number: PR,
        target: TARGET.to_owned(),
    }
}

#[test]
fn a_merge_press_is_one_human_overlay_event_on_the_final_lane() {
    let template = CycleTemplate::engineering();
    let dir = tempfile::tempdir().expect("scratch dir");
    let mut store = scratch_store(&dir);
    let recorded = store
        .record_human_action(
            &merge_action(),
            &mission(),
            Timestamp::now(),
            &template,
            None,
        )
        .expect("the press is recorded");
    let event = &recorded.event;
    assert_eq!(event.kind, EventKind::ApprovalGiven);
    assert_eq!(event.overlay, Some(Overlay::Human));
    assert_eq!(event.role, Role::Human);
    assert_eq!(&event.lane, template.final_lane());
    assert!(
        matches!(event.source, SourcePtr::App { action_id } if action_id == recorded.action_id)
    );
    let rows = store.human_actions(&mission()).expect("rows");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].kind, "merge");
    assert_eq!(rows[0].state, "requested");
    assert!(
        rows[0]
            .target
            .as_deref()
            .is_some_and(|target| target.contains("24")),
        "the row names the PR: {:?}",
        rows[0].target
    );
    println!("merge press row: {:?}, event lane {}", rows[0], event.lane);
}

#[test]
fn a_forbidden_merge_is_refused_with_the_exact_message_and_one_request() {
    let forge = Forge1::forbidden();
    let outcome = press(&forge, &pr(), &[MergeMethod::Squash]);
    assert_eq!(outcome.state, MergeState::Refused);
    assert_eq!(outcome.message, REFUSED);
    assert_eq!(forge.asked.get(), 1, "no retry");
}

#[test]
fn a_merge_that_goes_through_names_the_commit() {
    let forge = Forge1::new(Ok(Merged {
        sha: "36f287b84fe4d9a1deea3b081bb96ccee114b152".to_owned(),
    }));
    let outcome = press(&forge, &pr(), &[MergeMethod::Squash]);
    assert_eq!(outcome.state, MergeState::Merged);
    assert!(outcome.message.contains("PR #24"), "{}", outcome.message);
    assert!(outcome.message.contains("demo/base"), "{}", outcome.message);
    assert!(outcome.message.contains("36f287b"), "{}", outcome.message);
}

#[test]
fn any_other_failure_is_failed_and_carries_its_text() {
    let forge = Forge1::new(Err(ForgeError::Http {
        status: 405,
        url: "https://api.github.com/repos/dctellya/droid-observatory/pulls/24/merge".to_owned(),
    }));
    let outcome = press(&forge, &pr(), &[MergeMethod::Squash]);
    assert_eq!(outcome.state, MergeState::Failed);
    assert!(outcome.message.contains("405"), "{}", outcome.message);
    assert_eq!(forge.asked.get(), 1);
}

#[test]
fn the_method_is_squash_when_allowed_else_the_first_the_rule_allows() {
    assert_eq!(
        merge_method(&[MergeMethod::Merge, MergeMethod::Squash]),
        MergeMethod::Squash
    );
    assert_eq!(merge_method(&[MergeMethod::Rebase]), MergeMethod::Rebase);
    assert_eq!(merge_method(&[]), MergeMethod::Squash);
}

#[test]
fn two_presses_after_a_refusal_are_two_requests_and_two_refused_rows() {
    let template = CycleTemplate::engineering();
    let dir = tempfile::tempdir().expect("scratch dir");
    let mut store = scratch_store(&dir);
    let forge = Forge1::forbidden();
    for _ in 0..2 {
        let recorded = store
            .record_human_action(
                &merge_action(),
                &mission(),
                Timestamp::now(),
                &template,
                None,
            )
            .expect("the press is recorded");
        let outcome = press(&forge, &pr(), &[MergeMethod::Squash]);
        store
            .set_human_action_state(recorded.action_id, outcome.state.wire())
            .expect("the row takes the outcome");
    }
    assert_eq!(forge.asked.get(), 2, "one request per press");
    let rows = store.human_actions(&mission()).expect("rows");
    let refused = rows
        .iter()
        .filter(|row| row.kind == "merge" && row.state == "refused")
        .count();
    assert_eq!(refused, 2, "{rows:?}");
}

#[test]
fn a_merge_press_reaches_no_host_and_has_no_acknowledgement() {
    assert_eq!(merge_action().kind(), "merge");
    assert_eq!(merge_action().state(), "requested");
    assert_eq!(merge_action().ack_kind(), None);
    assert_eq!(merge_action().event_kind(), EventKind::ApprovalGiven);
}
