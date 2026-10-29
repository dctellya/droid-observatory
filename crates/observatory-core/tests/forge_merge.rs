//! The merge side of the gate (architecture §4.6, FR-G4..G6): which rule decides the merge, the
//! `[github].merge_policy` override, `Forge::merge` and its 403, and the one `handed_to_cd` event
//! a merge observed becomes. Responses are the recorded ones of `fixtures/github/demo-smoke-1/`;
//! the artifact and attestation bodies, which that repository has none of, are written here.
#![cfg(feature = "github")]

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use observatory_core::config::{GithubConfig, MergePolicy as ConfigPolicy};
use observatory_core::ingest::forge::{
    CheckConclusion, Forge, ForgeError, GateFacts, MergeMethod, MergePolicy, Merged, RequiredCheck,
    RuleAttribution,
};
use observatory_core::ingest::github::{GitHubClient, GitHubSource, HttpResponse, HttpTransport};
use observatory_core::mapping::MappingVersion;
use observatory_core::model::{
    CycleTemplate, Event, EventId, EventKind, MissionId, Role, SourcePtr, Timestamp,
};
use observatory_core::store::{EventScope, Store, TimeRange};
use observatory_core::view::{GateBadge, GateGroup};
use serde::Deserialize;

const REPO: &str = "dctellya/droid-observatory";
const BRANCH: &str = "demo/smoke-1";
const API: &str = "https://api.github.com";
const HEAD_SHA: &str = "1d994af00fb141466e46021e9c6f8adbf314db55";
const GREEN_RUN: u64 = 40_853_894_595;
const MERGE_SHA: &str = "a3ec578735fbcbbd15db8dfc7612fd9f16901a63";
const MERGED_AT: &str = "2026-10-29T10:05:00Z";
const RULESET_ID: u64 = 26_112_925;

fn fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/github/demo-smoke-1")
}

fn fixture(name: &str) -> String {
    std::fs::read_to_string(fixture_dir().join(name)).unwrap()
}

#[derive(Deserialize)]
struct Route {
    path: String,
    status: u16,
    body: String,
}

#[derive(Deserialize)]
struct Routes {
    routes: Vec<Route>,
}

/// Answers from the recorded routes, then from the overrides a test sets, and remembers every
/// request. A PUT is answered by `put_answer`.
#[derive(Clone)]
struct Replay {
    routes: Rc<Vec<(String, u16, String)>>,
    overrides: Rc<RefCell<Vec<(String, HttpResponse)>>>,
    asked: Rc<RefCell<Vec<String>>>,
    puts: Rc<RefCell<Vec<(String, String)>>>,
    put_answer: Rc<RefCell<Option<HttpResponse>>>,
}

fn response(status: u16, body: &str) -> HttpResponse {
    HttpResponse {
        status,
        body: body.to_owned(),
        rate_limit_remaining: None,
        rate_limit_reset: None,
    }
}

impl Replay {
    fn new() -> Self {
        let routes: Routes = serde_json::from_str(&fixture("routes.json")).unwrap();
        Self {
            routes: Rc::new(
                routes
                    .routes
                    .into_iter()
                    .map(|route| (route.path, route.status, fixture(&route.body)))
                    .collect(),
            ),
            overrides: Rc::default(),
            asked: Rc::default(),
            puts: Rc::default(),
            put_answer: Rc::default(),
        }
    }

    fn answer(&self, path: &str, status: u16, body: &str) {
        self.overrides
            .borrow_mut()
            .push((path.to_owned(), response(status, body)));
    }

    fn asked(&self) -> Vec<String> {
        self.asked.borrow().clone()
    }
}

impl HttpTransport for Replay {
    fn get(&self, url: &str) -> Result<HttpResponse, String> {
        let path = url.strip_prefix(API).expect("the client asks the API host");
        self.asked.borrow_mut().push(path.to_owned());
        if let Some((_, answer)) = self
            .overrides
            .borrow()
            .iter()
            .rev()
            .find(|(candidate, _)| candidate == path)
        {
            return Ok(answer.clone());
        }
        Ok(match self.routes.iter().find(|route| route.0 == path) {
            Some((_, status, body)) => response(*status, body),
            None if path.contains("/pulls?") => response(200, "[]"),
            None => response(404, r#"{"message":"Not Found"}"#),
        })
    }

    fn put(&self, url: &str, body: &str) -> Result<HttpResponse, String> {
        let path = url.strip_prefix(API).expect("the client asks the API host");
        self.puts
            .borrow_mut()
            .push((path.to_owned(), body.to_owned()));
        self.put_answer
            .borrow()
            .clone()
            .ok_or_else(|| "no PUT answer set".to_owned())
    }
}

fn mission() -> MissionId {
    MissionId("mission-demo".into())
}

fn config(policy: ConfigPolicy) -> GithubConfig {
    GithubConfig {
        target_branch: "demo/base".into(),
        repo: REPO.into(),
        merge_policy: policy,
        ..GithubConfig::default()
    }
}

fn source(replay: &Replay, policy: ConfigPolicy) -> GitHubSource<GitHubClient<Replay>> {
    GitHubSource::new(
        GitHubClient::new(replay.clone(), REPO),
        mission(),
        BRANCH,
        &config(policy),
        CycleTemplate::engineering(),
    )
}

fn now() -> Timestamp {
    Timestamp::parse("2026-10-29T10:06:00Z").unwrap()
}

fn list_path() -> String {
    format!("/repos/{REPO}/pulls?head=dctellya:{BRANCH}&base=demo/base&state=all")
}

/// The recorded pull request list with PR 24 changed by `change`.
fn pulls_list(change: impl FnOnce(&mut serde_json::Value)) -> String {
    let mut list: serde_json::Value = serde_json::from_str(&fixture("pulls.json")).unwrap();
    change(&mut list[0]);
    list.to_string()
}

/// PR 24 as an open pull request, so a merge is still to come.
fn open_pr(replay: &Replay) {
    replay.answer(
        &list_path(),
        200,
        &pulls_list(|pull| pull["state"] = "open".into()),
    );
}

/// PR 24 merged at `MERGED_AT` by `merger`, as the list and the single-PR endpoint tell it.
fn merged_pr(replay: &Replay, merger: &str) {
    replay.answer(
        &list_path(),
        200,
        &pulls_list(|pull| {
            pull["merged_at"] = MERGED_AT.into();
            pull["merge_commit_sha"] = MERGE_SHA.into();
        }),
    );
    let mut single: serde_json::Value = serde_json::from_str(&fixture("pull-24.json")).unwrap();
    single["merged"] = true.into();
    single["merged_at"] = MERGED_AT.into();
    single["merge_commit_sha"] = MERGE_SHA.into();
    single["merged_by"] = serde_json::json!({ "login": merger });
    replay.answer(&format!("/repos/{REPO}/pulls/24"), 200, &single.to_string());
}

const DIGEST: &str = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

/// The green run of PR 24 has one artifact, and two attestations vouch for it.
fn with_attestations(replay: &Replay) {
    replay.answer(
        &format!("/repos/{REPO}/actions/runs/{GREEN_RUN}/artifacts"),
        200,
        &format!(
            r#"{{"total_count":1,"artifacts":[{{"id":9,"name":"bundle","digest":"{DIGEST}"}}]}}"#
        ),
    );
    replay.answer(
        &format!("/repos/{REPO}/attestations/{DIGEST}"),
        200,
        r#"{"attestations":[{"id":777,"bundle_url":"https://example.invalid/a"},{"id":778,"bundle_url":"https://example.invalid/b"}]}"#,
    );
}

fn store_with_mission(dir: &Path) -> (Store, rusqlite::Connection) {
    let path = dir.join("o.sqlite");
    let store = Store::open(&path).unwrap();
    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.execute(
        "INSERT INTO missions (id, name, state, started_at) VALUES ('mission-demo', 'demo', 'at_gate', '2026-10-29T09:00:00Z')",
        [],
    )
    .unwrap();
    (store, conn)
}

fn event(kind: EventKind, second: i64) -> Event {
    let role = Role::External;
    let template = CycleTemplate::engineering();
    Event {
        id: EventId(format!("e{second}")),
        mission_id: mission(),
        milestone_id: None,
        feature_id: None,
        lane: template.lane_of(&kind, role),
        overlay: None,
        kind,
        role,
        session_id: None,
        model: None,
        at: Timestamp(now().0 + time::Duration::seconds(second)),
        duration: None,
        usage: None,
        source: SourcePtr::GitHub {
            url: format!("{API}/repos/{REPO}/pulls/24"),
        },
        mapping_version: MappingVersion::LATEST,
    }
}

fn handed(poll: &observatory_core::ingest::forge::Poll) -> Vec<&Event> {
    poll.events
        .iter()
        .filter(|event| matches!(event.kind, EventKind::HandedToCd { .. }))
        .collect()
}

fn facts(requires_human: bool, checks: &[CheckConclusion], pr_open: bool) -> GateFacts {
    GateFacts {
        policy: MergePolicy {
            requires_human,
            allowed_methods: vec![MergeMethod::Squash],
            decided_by: RuleAttribution::NoRule,
            required_checks: Vec::new(),
        },
        required_checks: checks
            .iter()
            .enumerate()
            .map(|(index, conclusion)| RequiredCheck {
                name: format!("check-{index}"),
                conclusion: *conclusion,
            })
            .collect(),
        reviews: 0,
        attestation_ids: Vec::new(),
        pr_open,
    }
}

#[test]
fn merge_button_shows_only_when_policy_is_human_and_checks_are_green() {
    use CheckConclusion::{Failed, Passed, RequiredMissing, Running};
    let green = [
        event(EventKind::PrOpened, 0),
        event(EventKind::CheckPassed, 1),
    ];
    let red = [
        event(EventKind::PrOpened, 0),
        event(EventKind::CheckFailed, 1),
    ];
    let running = [
        event(EventKind::PrOpened, 0),
        event(EventKind::CheckStarted, 1),
    ];
    let offered = |events: &[Event], facts: &GateFacts| {
        let group = GateGroup::of(events).with_facts(facts);
        (group.merge_offered, group.badge)
    };

    assert_eq!(
        offered(&green, &facts(true, &[Passed], true)),
        (true, GateBadge::CiGreen),
        "a human decides and every required check is green"
    );
    assert!(
        !offered(&green, &facts(false, &[Passed], true)).0,
        "policy merges: nothing to press"
    );
    assert!(
        !offered(&red, &facts(true, &[Failed], true)).0,
        "a red check"
    );
    assert!(
        !offered(&running, &facts(true, &[Running], true)).0,
        "a check still running"
    );
    assert!(
        !offered(&green, &facts(true, &[Passed, RequiredMissing], true)).0,
        "a required check that nothing reports"
    );
    assert!(
        !offered(&green, &facts(true, &[Passed, Failed], true)).0,
        "one of two required checks is red"
    );
    assert!(
        !offered(&green, &facts(true, &[Passed], false)).0,
        "the PR is not open"
    );
    assert!(
        !offered(&[], &facts(true, &[Passed], true)).0,
        "no PR event yet"
    );
}

#[test]
fn rule_attribution_names_the_ruleset() {
    let replay = Replay::new();
    let client = GitHubClient::new(replay.clone(), REPO);
    let policy = client.policy("demo/base").unwrap();
    assert_eq!(
        policy.decided_by,
        RuleAttribution::Ruleset {
            id: RULESET_ID,
            name: "demo-gate".into()
        },
        "the rules of demo/base come from ruleset {RULESET_ID}, which the list names"
    );
    assert!(policy.requires_human);
    assert_eq!(policy.required_checks, ["demo-gate"]);
    assert_eq!(
        policy.allowed_methods,
        [MergeMethod::Merge, MergeMethod::Squash, MergeMethod::Rebase]
    );

    open_pr(&replay);
    let poll = source(&replay, ConfigPolicy::Auto).poll(now()).unwrap();
    let gate = poll.gate.as_ref().expect("the poll read the gate facts");
    let group = GateGroup::of(&poll.events).with_facts(gate);
    assert_eq!(
        group.decided_by,
        Some(RuleAttribution::Ruleset {
            id: RULESET_ID,
            name: "demo-gate".into()
        })
    );
    assert!(group.requires_human);
    assert_eq!(
        group.required_checks,
        [RequiredCheck {
            name: "demo-gate".into(),
            conclusion: CheckConclusion::Passed
        }]
    );
    assert!(
        group.merge_offered,
        "green, open, and a ruleset asks a person"
    );
    let payload = serde_json::to_value(&group).unwrap();
    assert_eq!(
        payload["decided_by"],
        serde_json::json!({"ruleset": {"id": RULESET_ID, "name": "demo-gate"}})
    );
    assert_eq!(payload["requires_human"], true);
}

#[test]
fn without_a_ruleset_the_policy_reads_branch_protection_or_names_no_rule() {
    let replay = Replay::new();
    replay.answer(&format!("/repos/{REPO}/rules/branches/main"), 200, "[]");
    let client = GitHubClient::new(replay.clone(), REPO);

    replay.answer(
        &format!("/repos/{REPO}/branches/main/protection"),
        404,
        r#"{"message":"Branch not protected"}"#,
    );
    let none = client.policy("main").unwrap();
    assert_eq!(none.decided_by, RuleAttribution::NoRule);
    assert!(!none.requires_human);
    assert!(none.required_checks.is_empty());

    replay.answer(
        &format!("/repos/{REPO}/branches/main/protection"),
        200,
        r#"{"required_status_checks":{"contexts":["ci"]},"required_pull_request_reviews":{"required_approving_review_count":1}}"#,
    );
    let protected = client.policy("main").unwrap();
    assert_eq!(protected.decided_by, RuleAttribution::BranchProtection);
    assert!(protected.requires_human);
    assert_eq!(protected.required_checks, ["ci"]);
}

#[test]
fn merge_policy_override_hides_merge_and_names_the_config() {
    let replay = Replay::new();
    open_pr(&replay);
    let group = |policy| {
        let poll = source(&replay, policy).poll(now()).unwrap();
        GateGroup::of(&poll.events).with_facts(poll.gate.as_ref().unwrap())
    };

    let by_policy = group(ConfigPolicy::Policy);
    assert_eq!(
        by_policy.decided_by,
        Some(RuleAttribution::ConfigOverride("policy".into()))
    );
    assert!(!by_policy.requires_human);
    assert!(!by_policy.merge_offered, "policy never shows Merge");
    assert_eq!(
        serde_json::to_value(&by_policy).unwrap()["decided_by"],
        serde_json::json!({"config_override": "policy"})
    );

    let by_human = group(ConfigPolicy::Human);
    assert_eq!(
        by_human.decided_by,
        Some(RuleAttribution::ConfigOverride("human".into()))
    );
    assert!(by_human.requires_human && by_human.merge_offered);

    let back = group(ConfigPolicy::Auto);
    assert!(matches!(
        back.decided_by,
        Some(RuleAttribution::Ruleset { .. })
    ));
    assert!(back.requires_human && back.merge_offered);
}

#[test]
fn a_pr_with_auto_merge_enabled_asks_for_no_one_under_auto() {
    let replay = Replay::new();
    replay.answer(
        &list_path(),
        200,
        &pulls_list(|pull| {
            pull["state"] = "open".into();
            pull["auto_merge"] = serde_json::json!({"merge_method": "squash"});
        }),
    );
    let poll = source(&replay, ConfigPolicy::Auto).poll(now()).unwrap();
    let group = GateGroup::of(&poll.events).with_facts(poll.gate.as_ref().unwrap());
    assert!(!group.requires_human && !group.merge_offered);
}

#[test]
fn policy_merged_detected_without_human_action() {
    let replay = Replay::new();
    merged_pr(&replay, "some-colleague");
    let mut source = source(&replay, ConfigPolicy::Auto);
    let poll = source.poll(now()).unwrap();

    let handed_events = handed(&poll);
    let [done] = handed_events.as_slice() else {
        panic!("one hand-over: {:?}", poll.events);
    };
    assert_eq!(
        done.role,
        Role::External,
        "the merger is not the token owner"
    );
    assert_eq!(done.at, Timestamp::parse(MERGED_AT).unwrap());
    assert_eq!(
        poll.events.last().map(|event| &event.id),
        Some(&done.id),
        "handed_to_cd is the cycle's last event"
    );

    let dir = tempfile::tempdir().unwrap();
    let (mut store, conn) = store_with_mission(dir.path());
    store.record_poll(&mission(), &poll).unwrap();
    let merge_rows: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM human_actions WHERE kind = 'merge' AND target LIKE '%24%'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(merge_rows, 0, "the app requested no merge");
    let role: String = conn
        .query_row(
            "SELECT role FROM events WHERE kind = 'handed_to_cd'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(role, "external");
    let merged_by: String = conn
        .query_row("SELECT merged_by FROM prs WHERE number = 24", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(merged_by, "some-colleague");
    let events = store
        .events(&EventScope::Mission(mission()), TimeRange::ALL)
        .unwrap();
    assert_eq!(GateGroup::of(&events).badge, GateBadge::PolicyMerged);
}

#[test]
fn handed_to_cd_is_emitted_once_with_merge_sha_pr_and_attestations() {
    let replay = Replay::new();
    merged_pr(&replay, "dctellya");
    with_attestations(&replay);
    let mut source = source(&replay, ConfigPolicy::Auto);
    let first = source.poll(now()).unwrap();

    let handed_events = handed(&first);
    let [done] = handed_events.as_slice() else {
        panic!("one hand-over: {:?}", first.events);
    };
    assert_eq!(
        done.kind,
        EventKind::HandedToCd {
            merge_sha: MERGE_SHA.into(),
            pr_number: 24,
            attestation_ids: vec!["777".into(), "778".into()],
        }
    );
    assert_eq!(done.role, Role::Human, "the merger is the token owner");
    assert_eq!(
        done.lane,
        CycleTemplate::engineering().lane_of(&done.kind, done.role)
    );
    assert!(matches!(&done.source, SourcePtr::GitHub { url } if url.ends_with("/pulls/24")));
    assert_eq!(
        first.attestations.len(),
        2,
        "the poll carries them for the store"
    );

    let second = source
        .poll(Timestamp(now().0 + time::Duration::seconds(30)))
        .unwrap();
    assert!(handed(&second).is_empty(), "announced once per PR");

    let dir = tempfile::tempdir().unwrap();
    let (mut store, conn) = store_with_mission(dir.path());
    store.record_poll(&mission(), &first).unwrap();
    store.record_poll(&mission(), &second).unwrap();

    let stored: Vec<Event> = store
        .events(&EventScope::Mission(mission()), TimeRange::ALL)
        .unwrap()
        .into_iter()
        .filter(|event| matches!(event.kind, EventKind::HandedToCd { .. }))
        .collect();
    assert_eq!(stored.len(), 1);
    assert_eq!(
        stored[0].kind, done.kind,
        "the store round trip keeps the sha, the PR and the attestation ids"
    );
    assert_eq!(stored[0].role, Role::Human);
    let (sha, url): (String, String) = conn
        .query_row(
            "SELECT sha, url FROM events WHERE kind = 'handed_to_cd'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(sha, MERGE_SHA);
    assert!(url.ends_with("/pulls/24"));
    let attestations: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM attestations WHERE pr_number = 24",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(attestations, 2, "recording the poll twice keeps two rows");
}

#[test]
fn nothing_is_handed_over_while_a_required_check_is_not_green() {
    let replay = Replay::new();
    merged_pr(&replay, "dctellya");
    let mut runs: serde_json::Value = serde_json::from_str(&fixture("runs.json")).unwrap();
    runs["workflow_runs"]
        .as_array_mut()
        .unwrap()
        .retain(|run| run["id"].as_u64() != Some(GREEN_RUN));
    replay.answer(
        &format!("/repos/{REPO}/actions/runs?head_sha={HEAD_SHA}"),
        200,
        &runs.to_string(),
    );
    let poll = source(&replay, ConfigPolicy::Auto).poll(now()).unwrap();
    assert!(
        handed(&poll).is_empty(),
        "demo-gate last failed: {:?}",
        poll.events
    );
}

#[test]
fn a_merge_completes_the_mission_at_the_hand_over() {
    let replay = Replay::new();
    merged_pr(&replay, "dctellya");
    let mut source = source(&replay, ConfigPolicy::Auto);
    let first = source.poll(now()).unwrap();
    let done_at = handed(&first)[0].at;

    let dir = tempfile::tempdir().unwrap();
    let (mut store, conn) = store_with_mission(dir.path());
    let state = |conn: &rusqlite::Connection| -> (String, Option<String>) {
        conn.query_row(
            "SELECT state, completed_at FROM missions WHERE id = 'mission-demo'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap()
    };
    assert_eq!(state(&conn), ("at_gate".into(), None));
    store.record_poll(&mission(), &first).unwrap();
    let (state_text, completed) = state(&conn);
    assert_eq!(state_text, "complete");
    assert_eq!(
        Timestamp::parse(&completed.clone().unwrap()).unwrap(),
        done_at,
        "completed_at is the at of handed_to_cd"
    );

    let later = source
        .poll(Timestamp(now().0 + time::Duration::seconds(30)))
        .unwrap();
    store.record_poll(&mission(), &later).unwrap();
    assert_eq!(state(&conn).1, completed, "a later poll moves nothing");
    let row = &store.missions().unwrap()[0];
    assert_eq!(row.state, "complete");
}

#[test]
fn reviews_are_read_and_recorded_for_the_pr() {
    let replay = Replay::new();
    open_pr(&replay);
    replay.answer(
        &format!("/repos/{REPO}/pulls/24/reviews"),
        200,
        r#"[{"id":501,"state":"COMMENTED","html_url":"https://github.com/x/pull/24#r501","submitted_at":"2026-10-29T10:00:00Z"},
            {"id":502,"state":"APPROVED","html_url":"https://github.com/x/pull/24#r502","submitted_at":"2026-10-29T10:02:00Z"}]"#,
    );
    let poll = source(&replay, ConfigPolicy::Auto).poll(now()).unwrap();
    assert_eq!(poll.reviews.len(), 2);
    assert_eq!(poll.gate.as_ref().unwrap().reviews, 2);

    let dir = tempfile::tempdir().unwrap();
    let (mut store, conn) = store_with_mission(dir.path());
    store.record_poll(&mission(), &poll).unwrap();
    store.record_poll(&mission(), &poll).unwrap();
    let rows: Vec<(i64, String)> = conn
        .prepare("SELECT id, state FROM reviews WHERE pr_number = 24 ORDER BY id")
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(
        rows,
        [(501, "commented".to_owned()), (502, "approved".to_owned())]
    );
}

#[test]
fn merge_returns_forbidden_on_a_403_and_does_not_retry() {
    let replay = Replay::new();
    let client = GitHubClient::new(replay.clone(), REPO);
    let pr = client.pr(BRANCH, "demo/base").unwrap().unwrap();
    *replay.put_answer.borrow_mut() = Some(response(
        403,
        r#"{"message":"Resource not accessible by personal access token"}"#,
    ));

    let error = client.merge(&pr, MergeMethod::Squash).unwrap_err();
    assert_eq!(
        error,
        ForgeError::Forbidden {
            pr: 24,
            target: "demo/base".into(),
            missing_permission: "pull_requests:write",
        }
    );
    assert_eq!(
        error.to_string(),
        "Merge of PR #24 into demo/base refused: token lacks pull_requests:write. Merge in GitHub; the badge updates from the next poll."
    );
    assert_eq!(
        *replay.puts.borrow(),
        [(
            format!("/repos/{REPO}/pulls/24/merge"),
            r#"{"merge_method":"squash"}"#.to_owned()
        )],
        "one PUT, with the method the caller chose"
    );
}

#[test]
fn merge_reports_the_merge_commit_and_other_refusals_as_http_errors() {
    let replay = Replay::new();
    let client = GitHubClient::new(replay.clone(), REPO);
    let pr = client.pr(BRANCH, "demo/base").unwrap().unwrap();

    *replay.put_answer.borrow_mut() = Some(response(
        200,
        &format!(
            r#"{{"sha":"{MERGE_SHA}","merged":true,"message":"Pull Request successfully merged"}}"#
        ),
    ));
    assert_eq!(
        client.merge(&pr, MergeMethod::Rebase).unwrap(),
        Merged {
            sha: MERGE_SHA.into()
        }
    );
    assert_eq!(replay.puts.borrow()[0].1, r#"{"merge_method":"rebase"}"#);

    *replay.put_answer.borrow_mut() = Some(response(405, r#"{"message":"Not mergeable"}"#));
    assert!(matches!(
        client.merge(&pr, MergeMethod::Merge),
        Err(ForgeError::Http { status: 405, .. })
    ));
}

#[test]
fn the_gate_reads_no_check_runs_api_and_only_gets_besides_the_merge() {
    let replay = Replay::new();
    merged_pr(&replay, "dctellya");
    with_attestations(&replay);
    source(&replay, ConfigPolicy::Auto).poll(now()).unwrap();
    let asked = replay.asked();
    assert!(
        asked
            .iter()
            .all(|path| !path.contains("check-runs") && !path.contains("check-suites")),
        "{asked:?}"
    );
    assert!(replay.puts.borrow().is_empty(), "a poll never merges");
}
