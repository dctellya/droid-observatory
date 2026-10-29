//! `GitSource` (architecture §4.6, FR-G1/FR-G2): the mission branch, the `wt/*` worktrees, one
//! `integration.merged` per first-parent merge and the fast-check runs as `gate_command` events,
//! all read from a repository the test builds with the real `git`.

use std::path::{Path, PathBuf};
use std::process::Command;

use observatory_core::ingest::git::{FastCheckRun, GitOptions, GitSource, RunConclusion};
use observatory_core::mapping::MappingVersion;
use observatory_core::model::{
    CycleTemplate, Event, EventKind, MilestoneId, MissionId, Role, SourcePtr, Timestamp,
};

/// A repository with a mission branch, two milestone worktrees merged `--no-ff`, a side merge
/// inside a worktree branch, and a merge that no worktree branch made.
struct Repo {
    _dir: tempfile::TempDir,
    main: PathBuf,
    wt_m1: PathBuf,
    wt_m2: PathBuf,
    clock: std::cell::Cell<u32>,
}

impl Repo {
    fn git_in(&self, dir: &Path, args: &[&str]) -> String {
        let tick = self.clock.get() + 1;
        self.clock.set(tick);
        let date = format!("2026-10-2{}T10:00:{:02}+02:00", 1 + tick / 60, tick % 60);
        let output = Command::new("git")
            .arg("-C")
            .arg(dir)
            .args([
                "-c",
                "commit.gpgsign=false",
                "-c",
                "core.hooksPath=/dev/null",
            ])
            .args(args)
            .env("GIT_AUTHOR_NAME", "Test")
            .env("GIT_AUTHOR_EMAIL", "test@example.invalid")
            .env("GIT_COMMITTER_NAME", "Test")
            .env("GIT_COMMITTER_EMAIL", "test@example.invalid")
            .env("GIT_AUTHOR_DATE", &date)
            .env("GIT_COMMITTER_DATE", &date)
            .output()
            .expect("git runs");
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap().trim().to_owned()
    }

    fn git(&self, args: &[&str]) -> String {
        self.git_in(&self.main, args)
    }

    fn commit(&self, dir: &Path, file: &str, message: &str) -> String {
        std::fs::write(dir.join(file), message).unwrap();
        self.git_in(dir, &["add", file]);
        self.git_in(dir, &["commit", "-m", message]);
        self.git_in(dir, &["rev-parse", "HEAD"])
    }

    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        let main = root.join("repo");
        std::fs::create_dir(&main).unwrap();
        let repo = Self {
            wt_m1: root.join("wt-m1"),
            wt_m2: root.join("wt-m2"),
            main,
            _dir: dir,
            clock: std::cell::Cell::new(0),
        };
        repo.git(&["init", "-q", "-b", "main"]);
        repo.commit(&repo.main, "base.txt", "base");

        // A PR-style merge on the target branch, before the mission: no worktree branch made it.
        repo.git(&["checkout", "-q", "-b", "docs/spec"]);
        repo.commit(&repo.main, "spec.txt", "spec");
        repo.git(&["checkout", "-q", "main"]);
        repo.git(&[
            "merge",
            "--no-ff",
            "-m",
            "Merge pull request #1 from o/docs/spec",
            "docs/spec",
        ]);
        repo.git(&["checkout", "-q", "-b", "mission/demo"]);

        // m1: the worktree branch carries a side merge of its own; the mission merge names `wt/m1`.
        repo.git(&[
            "worktree",
            "add",
            "-q",
            "-b",
            "wt/m1",
            repo.wt_m1.to_str().unwrap(),
        ]);
        repo.commit(&repo.wt_m1, "m1-a.txt", "m1 work");
        repo.git_in(&repo.wt_m1, &["checkout", "-q", "-b", "side"]);
        repo.commit(&repo.wt_m1, "m1-side.txt", "m1 side");
        repo.git_in(&repo.wt_m1, &["checkout", "-q", "wt/m1"]);
        repo.git_in(
            &repo.wt_m1,
            &["merge", "--no-ff", "-m", "Merge side into wt/m1", "side"],
        );
        repo.git(&["merge", "--no-ff", "-m", "Integrate m1: wt/m1", "wt/m1"]);

        // m2: branched after the m1 merge; the merge subject does not name the branch.
        repo.git(&[
            "worktree",
            "add",
            "-q",
            "-b",
            "wt/m2",
            repo.wt_m2.to_str().unwrap(),
        ]);
        repo.commit(&repo.wt_m2, "m2-a.txt", "m2 work");
        repo.git(&[
            "merge",
            "--no-ff",
            "-m",
            "Integrate m2: board and attach",
            "wt/m2",
        ]);

        repo.commit(&repo.main, "after.txt", "docs after the last merge");
        repo
    }

    fn merge_shas(&self) -> Vec<String> {
        self.git(&[
            "log",
            "--first-parent",
            "--merges",
            "--format=%H",
            "mission/demo",
        ])
        .lines()
        .map(str::to_owned)
        .collect()
    }

    fn source(&self) -> GitSource {
        GitSource::open(
            &self.main,
            MissionId("mission-1".into()),
            "mission/demo",
            CycleTemplate::engineering(),
            GitOptions::default(),
        )
        .expect("the repository and the branch exist")
    }
}

fn integration_lane() -> observatory_core::model::LaneId {
    CycleTemplate::engineering()
        .fixed_lane(&EventKind::IntegrationMerged)
        .expect("the template places integration.merged in a lane")
        .clone()
}

#[test]
fn git_source_lists_mission_branch_and_worktrees() {
    let repo = Repo::new();
    assert_eq!(
        GitSource::mission_branches(&repo.main).unwrap(),
        vec!["mission/demo".to_owned()]
    );

    let snapshot = repo.source().snapshot().unwrap();
    assert_eq!(snapshot.mission_branch, "mission/demo");
    assert_eq!(
        snapshot.mission_head,
        repo.git(&["rev-parse", "refs/heads/mission/demo"])
    );

    let listed = repo.git(&["worktree", "list", "--porcelain"]);
    assert_eq!(
        snapshot.worktrees.len(),
        listed
            .lines()
            .filter(|l| l.starts_with("worktree "))
            .count(),
        "every worktree git lists is present: {listed}"
    );
    let named: Vec<(PathBuf, Option<&str>, Option<&str>)> = snapshot
        .worktrees
        .iter()
        .map(|w| {
            (
                w.path.clone(),
                w.branch.as_deref(),
                w.milestone.as_ref().map(|m| m.0.as_str()),
            )
        })
        .collect();
    assert!(
        named.contains(&(repo.main.clone(), Some("mission/demo"), None)),
        "{named:?}"
    );
    assert!(
        named.contains(&(repo.wt_m1.clone(), Some("wt/m1"), Some("m1"))),
        "{named:?}"
    );
    assert!(
        named.contains(&(repo.wt_m2.clone(), Some("wt/m2"), Some("m2"))),
        "{named:?}"
    );

    let branches: Vec<(&str, &str)> = snapshot
        .milestone_branches
        .iter()
        .map(|b| (b.milestone.0.as_str(), b.branch.as_str()))
        .collect();
    assert_eq!(branches, vec![("m1", "wt/m1"), ("m2", "wt/m2")]);
    assert_eq!(
        snapshot.milestone_branches[0].tip,
        repo.git(&["rev-parse", "refs/heads/wt/m1"])
    );
}

#[test]
fn a_branch_that_does_not_exist_is_an_error_not_an_empty_source() {
    let repo = Repo::new();
    let err = GitSource::open(
        &repo.main,
        MissionId("mission-1".into()),
        "mission/absent",
        CycleTemplate::engineering(),
        GitOptions::default(),
    )
    .err()
    .expect("a missing branch is refused");
    assert!(err.to_string().contains("mission/absent"), "{err}");

    let not_a_repo = tempfile::tempdir().unwrap();
    assert!(GitSource::mission_branches(not_a_repo.path()).is_err());
}

#[test]
fn integration_merges_become_lane6_events_one_per_first_parent_merge() {
    let repo = Repo::new();
    let events = repo.source().integration_events().unwrap();

    let mut shas: Vec<String> = events
        .iter()
        .map(|event| match &event.source {
            SourcePtr::Git { repo: path, sha } => {
                assert_eq!(path, &repo.main);
                sha.clone()
            }
            other => panic!("a git event points at git: {other:?}"),
        })
        .collect();
    let mut expected = repo.merge_shas();
    assert_eq!(expected.len(), 3, "PR merge, m1 and m2: {expected:?}");
    shas.sort();
    expected.sort();
    assert_eq!(
        shas, expected,
        "exactly the first-parent merges, no side merge"
    );

    let lane = integration_lane();
    for event in &events {
        assert_eq!(event.kind, EventKind::IntegrationMerged);
        assert_eq!(event.lane, lane);
        assert_eq!(event.mission_id, MissionId("mission-1".into()));
        assert_eq!(event.role, Role::Orchestrator);
        assert_eq!(event.mapping_version, MappingVersion::LATEST);
        assert_eq!(event.overlay, None);
        assert_eq!(event.session_id, None);
    }

    let milestone_of = |subject_sha: &str| {
        events
            .iter()
            .find(|e| matches!(&e.source, SourcePtr::Git { sha, .. } if sha == subject_sha))
            .and_then(|e| e.milestone_id.clone())
    };
    // newest first, as git prints them: m2, m1, then the PR merge
    let by_git = repo.merge_shas();
    assert_eq!(
        milestone_of(&by_git[0]),
        Some(MilestoneId("m2".into())),
        "by containment"
    );
    assert_eq!(
        milestone_of(&by_git[1]),
        Some(MilestoneId("m1".into())),
        "by subject"
    );
    assert_eq!(milestone_of(&by_git[2]), None, "no worktree branch made it");

    let times: Vec<Timestamp> = events.iter().map(|e| e.at).collect();
    assert!(
        times.windows(2).all(|w| w[0] <= w[1]),
        "oldest first: {times:?}"
    );
    let committed = repo.git(&["log", "-1", "--format=%cI", &by_git[1]]);
    let m1 = events
        .iter()
        .find(|e| e.milestone_id == Some(MilestoneId("m1".into())))
        .unwrap();
    assert_eq!(m1.at, Timestamp::parse(&committed).unwrap());
}

#[test]
fn integration_event_ids_are_stable_across_reads() {
    let repo = Repo::new();
    let first = repo.source().integration_events().unwrap();
    let second = repo.source().integration_events().unwrap();
    assert_eq!(first, second);
    let ids: std::collections::BTreeSet<_> = first.iter().map(|e| e.id.clone()).collect();
    assert_eq!(ids.len(), first.len());
}

fn run(id: u64, head: &str, conclusion: Option<RunConclusion>) -> FastCheckRun {
    let started = Timestamp::parse("2026-10-29T08:00:00Z").unwrap();
    FastCheckRun {
        run_id: id,
        url: format!("https://api.github.com/repos/o/r/actions/runs/{id}"),
        head_sha: head.to_owned(),
        started_at: started,
        finished_at: conclusion.map(|_| Timestamp::parse("2026-10-29T08:01:30Z").unwrap()),
        conclusion,
    }
}

#[test]
fn gate_command_runs_are_lane6_events_attributed_to_the_milestone() {
    let repo = Repo::new();
    let merges = repo.merge_shas();
    let (m2_merge, m1_merge) = (&merges[0], &merges[1]);
    let after = repo.git(&["rev-parse", "mission/demo"]);

    let runs = vec![
        run(101, m1_merge, Some(RunConclusion::Passed)),
        run(102, m2_merge, Some(RunConclusion::Failed)),
        run(103, m2_merge, None),
        run(104, &after, Some(RunConclusion::Passed)),
    ];
    let events = repo.source().gate_command_events(&runs).unwrap();

    let lane = integration_lane();
    let of_run = |id: u64| -> Vec<&Event> {
        let needle = format!("/runs/{id}");
        events
            .iter()
            .filter(|e| matches!(&e.source, SourcePtr::GitHub { url } if url.ends_with(&needle)))
            .collect()
    };
    let kinds = |id: u64| -> Vec<EventKind> { of_run(id).iter().map(|e| e.kind.clone()).collect() };

    assert_eq!(
        kinds(101),
        vec![EventKind::GateCommandRun, EventKind::GateCommandPassed]
    );
    assert_eq!(
        kinds(102),
        vec![EventKind::GateCommandRun, EventKind::GateCommandFailed]
    );
    assert_eq!(
        kinds(103),
        vec![EventKind::GateCommandRun],
        "a run with no verdict yet"
    );
    assert_eq!(
        kinds(104),
        vec![EventKind::GateCommandRun, EventKind::GateCommandPassed]
    );
    assert_eq!(events.len(), 2 + 2 + 1 + 2);

    for event in &events {
        assert_eq!(event.lane, lane);
        assert_eq!(event.role, Role::External);
        assert_eq!(event.mapping_version, MappingVersion::LATEST);
        assert!(!matches!(event.kind, EventKind::Unknown { .. }));
    }
    for (id, milestone) in [
        (101, Some("m1")),
        (102, Some("m2")),
        (103, Some("m2")),
        (104, None),
    ] {
        for event in of_run(id) {
            assert_eq!(
                event.milestone_id.as_ref().map(|m| m.0.as_str()),
                milestone,
                "run {id}: {event:?}"
            );
        }
    }

    let verdict = of_run(101)[1];
    assert_eq!(verdict.duration, Some(std::time::Duration::from_secs(90)));
    assert_eq!(
        verdict.at,
        Timestamp::parse("2026-10-29T08:01:30Z").unwrap()
    );
    assert_eq!(
        of_run(101)[0].at,
        Timestamp::parse("2026-10-29T08:00:00Z").unwrap()
    );

    let ids: std::collections::BTreeSet<_> = events.iter().map(|e| e.id.clone()).collect();
    assert_eq!(ids.len(), events.len(), "event ids are unique");
}

#[test]
fn github_conclusions_map_to_a_verdict_only_when_the_run_has_one() {
    assert_eq!(
        RunConclusion::from_github("success"),
        Some(RunConclusion::Passed)
    );
    for failed in ["failure", "timed_out", "startup_failure"] {
        assert_eq!(
            RunConclusion::from_github(failed),
            Some(RunConclusion::Failed),
            "{failed}"
        );
    }
    for none in [
        "",
        "cancelled",
        "skipped",
        "neutral",
        "action_required",
        "stale",
    ] {
        assert_eq!(RunConclusion::from_github(none), None, "{none:?}");
    }
}

#[cfg(feature = "store")]
mod store {
    use super::*;
    use observatory_core::config::IngestConfig;
    use observatory_core::ingest::{SessionTree, capture_path};
    use observatory_core::store::{CaptureInfo, EventScope, Store, TimeRange};

    fn m0_root() -> PathBuf {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/streams/2026-10-22-observatory-v1-m0.jsonl");
        assert!(capture_path(&root).exists());
        root
    }

    #[test]
    fn lane6_rows_reach_the_store_with_zero_unknown_and_the_branch_on_the_mission() {
        let repo = Repo::new();
        let root = m0_root();
        let tree = SessionTree::discover(&root, &IngestConfig::default()).unwrap();
        let mission = tree.mission_id().clone();
        let db = tempfile::tempdir().unwrap();
        let mut store = Store::open(&db.path().join("o.sqlite")).unwrap();
        store
            .record_tree(&tree, &CaptureInfo::from_session_file(&root).unwrap(), true)
            .unwrap();

        let source = GitSource::open(
            &repo.main,
            mission.clone(),
            "mission/demo",
            CycleTemplate::engineering(),
            GitOptions::default(),
        )
        .unwrap();
        let merges = source.integration_events().unwrap();
        let head = repo.merge_shas()[0].clone();
        let gate = source
            .gate_command_events(&[run(7, &head, Some(RunConclusion::Passed))])
            .unwrap();

        assert_eq!(store.insert_events(&merges).unwrap(), 3);
        assert_eq!(store.insert_events(&gate).unwrap(), 2);
        assert_eq!(
            store.insert_events(&merges).unwrap(),
            0,
            "a re-read adds nothing"
        );
        assert_eq!(store.unknown_count(&mission).unwrap(), 0);

        assert!(store.set_mission_branch(&mission, "mission/demo").unwrap());
        assert_eq!(
            store.mission_branch(&mission).unwrap().as_deref(),
            Some("mission/demo")
        );
        assert!(
            !store
                .set_mission_branch(&MissionId("absent".into()), "x")
                .unwrap()
        );

        let stored = store
            .events(
                &EventScope::Milestone(mission.clone(), MilestoneId("m2".into())),
                TimeRange::ALL,
            )
            .unwrap();
        let kinds: Vec<EventKind> = stored.iter().map(|e| e.kind.clone()).collect();
        assert!(kinds.contains(&EventKind::IntegrationMerged), "{kinds:?}");
        assert!(kinds.contains(&EventKind::GateCommandPassed), "{kinds:?}");
        let all = store
            .events(&EventScope::Mission(mission), TimeRange::ALL)
            .unwrap();
        let in_lane = all.iter().filter(|e| e.lane == integration_lane()).count();
        assert!(
            in_lane >= 5,
            "the lane-6 rows of git and the runs: {in_lane}"
        );
        for event in all
            .iter()
            .filter(|e| matches!(e.source, SourcePtr::Git { .. }))
        {
            assert_eq!(event.lane, integration_lane());
        }
    }
}
