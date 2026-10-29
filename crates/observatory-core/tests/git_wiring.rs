//! What one git poll leaves in the store (FR-G1, FR-G2; VAL-GATE-007, -008, -009): the mission
//! branch, one `integration.merged` row per first-parent merge, and the fast-check runs of the
//! branch as `gate_command` rows attributed to the milestone integrated last. The repository is
//! built with the real `git`; the run list comes through the real `GitHubClient` over a fake
//! transport.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use observatory_core::ingest::git::{GitOptions, GitSource, git_sync};
use observatory_core::ingest::github::{GitHubClient, HttpResponse, HttpTransport};
use observatory_core::model::{CycleTemplate, MissionId};
use observatory_core::store::Store;

const MISSION: &str = "mission-1";
const REPO: &str = "o/r";

/// A repository with a pre-mission merge on `main`, the branch `mission/demo`, two `wt/*`
/// worktrees merged `--no-ff`, a side merge inside `wt/m1` and a commit after the last merge.
struct Repo {
    _dir: tempfile::TempDir,
    main: PathBuf,
    root: PathBuf,
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

    /// A worktree on a new `wt/<name>` branch with one commit, merged into the mission branch.
    fn integrate(&self, name: &str, subject: &str) {
        let wt = self.root.join(format!("wt-{name}"));
        self.git(&[
            "worktree",
            "add",
            "-q",
            "-b",
            &format!("wt/{name}"),
            wt.to_str().unwrap(),
        ]);
        self.commit(&wt, &format!("{name}.txt"), &format!("{name} work"));
        self.git(&["merge", "--no-ff", "-m", subject, &format!("wt/{name}")]);
    }

    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        let main = root.join("repo");
        std::fs::create_dir(&main).unwrap();
        let repo = Self {
            main,
            root,
            _dir: dir,
            clock: std::cell::Cell::new(0),
        };
        repo.git(&["init", "-q", "-b", "main"]);
        repo.commit(&repo.main, "base.txt", "base");
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

        let wt_m1 = repo.root.join("wt-m1");
        repo.git(&[
            "worktree",
            "add",
            "-q",
            "-b",
            "wt/m1",
            wt_m1.to_str().unwrap(),
        ]);
        repo.commit(&wt_m1, "m1-a.txt", "m1 work");
        repo.git_in(&wt_m1, &["checkout", "-q", "-b", "side"]);
        repo.commit(&wt_m1, "m1-side.txt", "m1 side");
        repo.git_in(&wt_m1, &["checkout", "-q", "wt/m1"]);
        repo.git_in(
            &wt_m1,
            &["merge", "--no-ff", "-m", "Merge side into wt/m1", "side"],
        );
        repo.git(&["merge", "--no-ff", "-m", "Integrate m1: wt/m1", "wt/m1"]);
        repo.integrate("m2", "Integrate m2: board and attach");
        repo.commit(&repo.main, "after.txt", "docs after the last merge");
        repo
    }

    fn mission_merges(&self) -> Vec<String> {
        let fork = self.git(&["merge-base", "main", "mission/demo"]);
        self.git(&[
            "log",
            "--first-parent",
            "--merges",
            "--format=%H",
            &format!("{fork}..mission/demo"),
        ])
        .lines()
        .map(str::to_owned)
        .collect()
    }

    fn first_parent_merges(&self) -> Vec<String> {
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
            MissionId(MISSION.into()),
            "mission/demo",
            CycleTemplate::engineering(),
            GitOptions::from_branch_pattern("mission/<name>"),
        )
        .expect("the repository and the branch exist")
    }
}

/// Answers the one workflow-runs query the poll makes and refuses everything else.
struct Runs(String);

impl HttpTransport for Runs {
    fn get(&self, url: &str) -> Result<HttpResponse, String> {
        let wanted = format!(
            "/repos/{REPO}/actions/workflows/mission-branch.yml/runs?branch=mission/demo&per_page=100"
        );
        let (status, body) = if url.ends_with(&wanted) {
            (200, self.0.clone())
        } else {
            (404, r#"{"message":"Not Found"}"#.to_owned())
        };
        Ok(HttpResponse {
            status,
            body,
            rate_limit_remaining: None,
            rate_limit_reset: None,
        })
    }
}

fn run_json(id: u64, head: &str, status: &str, conclusion: Option<&str>) -> String {
    let conclusion = conclusion.map_or("null".to_owned(), |text| format!("\"{text}\""));
    format!(
        r#"{{"id":{id},"run_number":{id},"name":"mission-checks","status":"{status}","conclusion":{conclusion},
        "html_url":"https://github.com/{REPO}/actions/runs/{id}","head_sha":"{head}",
        "created_at":"2026-10-29T10:00:00Z","run_started_at":"2026-10-29T10:00:05Z",
        "updated_at":"2026-10-29T10:03:05Z"}}"#
    )
}

fn runs_from(
    heads: &[(u64, String, &str, Option<&str>)],
) -> Vec<observatory_core::ingest::git::FastCheckRun> {
    let body = heads
        .iter()
        .map(|(id, head, status, conclusion)| run_json(*id, head, status, *conclusion))
        .collect::<Vec<_>>()
        .join(",");
    let client = GitHubClient::new(
        Runs(format!(r#"{{"total_count":0,"workflow_runs":[{body}]}}"#)),
        REPO,
    );
    client
        .mission_branch_runs("mission/demo")
        .expect("the run list parses")
}

fn store_with_mission(dir: &Path) -> (Store, rusqlite::Connection) {
    let path = dir.join("o.sqlite");
    let store = Store::open(&path).unwrap();
    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.execute(
        "INSERT INTO missions (id, name, state, started_at) VALUES ('mission-1', 'demo', 'building', '2026-10-29T09:00:00Z')",
        [],
    )
    .unwrap();
    (store, conn)
}

fn column<T: rusqlite::types::FromSql>(conn: &rusqlite::Connection, sql: &str) -> Vec<T> {
    let mut statement = conn.prepare(sql).unwrap();
    statement
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

fn event_count(conn: &rusqlite::Connection) -> i64 {
    conn.query_row("SELECT count(*) FROM events", [], |row| row.get(0))
        .unwrap()
}

#[test]
fn the_git_poll_records_integration_and_gate_command_rows_in_the_store() {
    let repo = Repo::new();
    let merges = repo.first_parent_merges();
    let (m2_merge, m1_merge, pr_merge) = (merges[0].clone(), merges[1].clone(), merges[2].clone());
    let after = repo.git(&["rev-parse", "mission/demo"]);
    let runs = runs_from(&[
        (1001, m1_merge, "completed", Some("success")),
        (1002, m2_merge, "completed", Some("failure")),
        (1003, after, "completed", Some("cancelled")),
        (1004, pr_merge, "completed", Some("success")),
    ]);
    assert_eq!(runs.len(), 4, "the client lists every run of the branch");

    let dir = tempfile::tempdir().unwrap();
    let (mut store, conn) = store_with_mission(dir.path());
    let mission = MissionId(MISSION.into());

    let first = git_sync(&mut store, &repo.source(), &runs).unwrap();
    assert_eq!(first.merges, 3);
    assert_eq!(first.runs, 4);
    assert_eq!(first.new_events.len(), 3 + 4 + 3, "merges, runs, verdicts");
    assert_eq!(event_count(&conn), 10);

    // VAL-GATE-008: the merge shas of the mission range, each attributed.
    let rows: Vec<String> = column(
        &conn,
        "SELECT sha FROM events WHERE kind='integration.merged' AND lane = 'integrate' AND source='git'",
    );
    let expected: BTreeSet<String> = repo.mission_merges().into_iter().collect();
    assert_eq!(expected.len(), 2, "m1 and m2");
    let stored: BTreeSet<String> = rows.into_iter().collect();
    assert!(
        expected.is_subset(&stored),
        "{expected:?} within {stored:?}"
    );
    let all: BTreeSet<String> = repo.first_parent_merges().into_iter().collect();
    assert_eq!(stored, all, "the pre-mission merge is present too");
    for sha in &expected {
        let milestone: Option<String> = conn
            .query_row(
                "SELECT milestone_id FROM events WHERE kind='integration.merged' AND sha = ?1",
                [sha],
                |row| row.get(0),
            )
            .unwrap();
        assert!(milestone.is_some(), "{sha} has a milestone");
    }
    let unattributed: Vec<String> = column(
        &conn,
        "SELECT sha FROM events WHERE kind='integration.merged' AND milestone_id IS NULL",
    );
    assert_eq!(
        unattributed,
        repo.git(&["rev-parse", "main"])
            .lines()
            .map(str::to_owned)
            .collect::<Vec<_>>()
    );

    // VAL-GATE-009: one .run and one verdict per run id (a cancelled run: .run only).
    let gate: Vec<(String, String, Option<String>)> = {
        let mut statement = conn
            .prepare(
                "SELECT kind, url, milestone_id FROM events WHERE kind IN \
                 ('gate_command.run','gate_command.passed','gate_command.failed') \
                 AND lane = 'integrate' AND source='github' ORDER BY url, kind",
            )
            .unwrap();
        statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect()
    };
    let expect = |id: u64, kinds: &[&str], milestone: Option<&str>| {
        let mine: Vec<&(String, String, Option<String>)> = gate
            .iter()
            .filter(|(_, url, _)| url.contains(&format!("/actions/runs/{id}")))
            .collect();
        let mut kinds_found: Vec<&str> = mine.iter().map(|row| row.0.as_str()).collect();
        kinds_found.sort_unstable();
        assert_eq!(kinds_found, kinds, "run {id}");
        for row in mine {
            assert_eq!(row.2.as_deref(), milestone, "run {id} milestone");
        }
    };
    expect(
        1001,
        &["gate_command.passed", "gate_command.run"],
        Some("m1"),
    );
    expect(
        1002,
        &["gate_command.failed", "gate_command.run"],
        Some("m2"),
    );
    expect(1003, &["gate_command.run"], Some("m2"));
    expect(1004, &["gate_command.passed", "gate_command.run"], None);
    assert_eq!(gate.len(), 7);

    // VAL-GATE-007: the branch row and every worktree git lists.
    let listed: Vec<String> = repo
        .git(&[
            "for-each-ref",
            "--format=%(refname:short)",
            "refs/heads/mission/*",
        ])
        .lines()
        .map(str::to_owned)
        .collect();
    assert_eq!(listed, vec!["mission/demo".to_owned()]);
    assert_eq!(
        store.mission_branch(&mission).unwrap().as_deref(),
        Some("mission/demo")
    );
    let branch_row: Vec<String> =
        column(&conn, "SELECT branch FROM missions WHERE id = 'mission-1'");
    assert_eq!(branch_row, listed);
    let porcelain = repo.git(&["worktree", "list", "--porcelain"]);
    let git_paths: BTreeSet<PathBuf> = porcelain
        .lines()
        .filter_map(|line| line.strip_prefix("worktree "))
        .map(|path| Path::new(path).canonicalize().unwrap())
        .collect();
    let snapshot_paths: BTreeSet<PathBuf> = first
        .snapshot
        .worktrees
        .iter()
        .map(|w| w.path.clone())
        .collect();
    assert_eq!(snapshot_paths, git_paths, "{porcelain}");

    // A second poll adds nothing; a new merge adds one row.
    let second = git_sync(&mut store, &repo.source(), &runs).unwrap();
    assert!(second.new_events.is_empty());
    assert_eq!(event_count(&conn), 10);
    repo.integrate("m3", "Integrate m3: more");
    let third = git_sync(&mut store, &repo.source(), &runs).unwrap();
    assert_eq!(third.new_events.len(), 1);
    assert_eq!(event_count(&conn), 11);
}

#[test]
fn a_missing_branch_errors_and_writes_nothing() {
    let repo = Repo::new();
    let dir = tempfile::tempdir().unwrap();
    let (store, conn) = store_with_mission(dir.path());
    let error = GitSource::open(
        &repo.main,
        MissionId(MISSION.into()),
        "mission/absent",
        CycleTemplate::engineering(),
        GitOptions::from_branch_pattern("mission/<name>"),
    )
    .err()
    .expect("a branch the repository lacks is refused");
    assert!(error.to_string().contains("mission/absent"), "{error}");
    assert_eq!(event_count(&conn), 0);
    assert_eq!(
        store.mission_branch(&MissionId(MISSION.into())).unwrap(),
        None
    );
}

#[test]
fn the_branch_pattern_decides_which_branches_are_mission_branches() {
    let repo = Repo::new();
    repo.git(&["branch", "release/v2"]);
    let mission = GitOptions::from_branch_pattern("mission/<name>");
    let release = GitOptions::from_branch_pattern("release/<name>");
    assert_eq!(
        GitSource::mission_branches(&repo.main, &mission).unwrap(),
        vec!["mission/demo".to_owned()]
    );
    assert_eq!(
        GitSource::mission_branches(&repo.main, &release).unwrap(),
        vec!["release/v2".to_owned()]
    );
    assert_eq!(GitOptions::default(), mission);
}
