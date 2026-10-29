//! `GitHubSource` behind the `Forge` trait (architecture §4.6, FR-G3): PR detection for the
//! mission branch, the target branch from config, the two read endpoints for checks, and a log
//! that carries method, URL and status and never the token. Responses are recorded
//! (`fixtures/github/demo-smoke-1/`, taken with `gh api` on the demo pair); no test calls GitHub.
#![cfg(feature = "github")]

use std::cell::{Cell, RefCell};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Mutex;
use std::time::Duration;

use observatory_core::config::GithubConfig;
use observatory_core::ingest::forge::{CheckConclusion, CheckSource, Forge, ForgeError, PrState};
use observatory_core::ingest::github::{
    GitHubClient, GitHubSource, HttpResponse, HttpTransport, UreqTransport,
};
use observatory_core::model::{CycleTemplate, EventKind, MissionId, Role, SourcePtr, Timestamp};
use observatory_core::secrets::Secret;
use observatory_core::store::Store;
use serde::Deserialize;

const REPO: &str = "dctellya/droid-observatory";
const BRANCH: &str = "demo/smoke-1";
const API: &str = "https://api.github.com";
const HEAD_SHA: &str = "1d994af00fb141466e46021e9c6f8adbf314db55";
/// 40 characters after the prefix: the shape of a fine-grained PAT, and not a real one.
const TOKEN: &str = "github_pat_11AAAAAAA0abcdefghijklmnopqrst_0123456789abcdefghijklmnopqrstuvwxyz0123456789ABCDEF";

fn fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/github/demo-smoke-1")
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

fn recorded_routes() -> Vec<(String, u16, String)> {
    let text = std::fs::read_to_string(fixture_dir().join("routes.json")).unwrap();
    let routes: Routes = serde_json::from_str(&text).unwrap();
    routes
        .routes
        .into_iter()
        .map(|route| {
            let body = std::fs::read_to_string(fixture_dir().join(&route.body)).unwrap();
            (route.path, route.status, body)
        })
        .collect()
}

/// Answers from the recorded routes and remembers every URL it was asked for. A pulls query
/// the recording does not hold lists nothing, as GitHub does.
#[derive(Clone)]
struct Replay {
    routes: Rc<Vec<(String, u16, String)>>,
    asked: Rc<RefCell<Vec<String>>>,
    /// False until the PR "exists": the recorded empty list answers the pulls query.
    pr_visible: Rc<Cell<bool>>,
    overrides: Rc<RefCell<Vec<(String, HttpResponse)>>>,
}

impl Replay {
    fn new() -> Self {
        Self {
            routes: Rc::new(recorded_routes()),
            asked: Rc::default(),
            pr_visible: Rc::new(Cell::new(true)),
            overrides: Rc::default(),
        }
    }

    fn answer(&self, path: &str, status: u16, body: &str) {
        self.overrides.borrow_mut().push((
            path.to_owned(),
            HttpResponse {
                status,
                body: body.to_owned(),
                rate_limit_remaining: None,
                rate_limit_reset: None,
            },
        ));
    }

    fn asked(&self) -> Vec<String> {
        self.asked.borrow().clone()
    }
}

impl HttpTransport for Replay {
    fn get(&self, url: &str) -> Result<HttpResponse, String> {
        let path = url.strip_prefix(API).expect("the client asks the API host");
        self.asked.borrow_mut().push(path.to_owned());
        if let Some((_, response)) = self
            .overrides
            .borrow()
            .iter()
            .find(|(candidate, _)| candidate == path)
        {
            return Ok(response.clone());
        }
        let hit = |wanted: &str| self.routes.iter().find(|route| route.0 == wanted);
        let found = if path.contains("/pulls?") && !self.pr_visible.get() {
            hit(&format!(
                "/repos/{REPO}/pulls?head=dctellya:demo/none-yet&base=demo/base&state=all"
            ))
        } else {
            hit(path)
        };
        let (status, body) = match found {
            Some((_, status, body)) => (*status, body.clone()),
            None if path.contains("/pulls?") => (200, "[]".to_owned()),
            None => (404, r#"{"message":"Not Found"}"#.to_owned()),
        };
        Ok(HttpResponse {
            status,
            body,
            rate_limit_remaining: None,
            rate_limit_reset: None,
        })
    }
}

fn config(target: &str) -> GithubConfig {
    GithubConfig {
        target_branch: target.into(),
        repo: REPO.into(),
        poll_interval_s: 30,
        ..GithubConfig::default()
    }
}

fn mission() -> MissionId {
    MissionId("mission-demo".into())
}

fn source(replay: &Replay, target: &str) -> GitHubSource<GitHubClient<Replay>> {
    GitHubSource::new(
        GitHubClient::new(replay.clone(), REPO),
        mission(),
        BRANCH,
        &config(target),
        CycleTemplate::engineering(),
    )
}

fn at(base: Timestamp, seconds: i64) -> Timestamp {
    Timestamp(base.0 + time::Duration::seconds(seconds))
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

#[test]
fn pr_for_mission_branch_is_detected_within_poll_interval() {
    let replay = Replay::new();
    replay.pr_visible.set(false);
    let mut source = source(&replay, "demo/base");
    let created = Timestamp::parse("2026-10-29T09:58:30Z").unwrap();

    // A one-second test clock starts 20 s before the PR exists; the source decides when it is due.
    let mut found = None;
    for second in -20..120 {
        let now = at(created, second);
        if second == 0 {
            replay.pr_visible.set(true);
        }
        if let Some(poll) = source.poll_if_due(now).unwrap()
            && poll.pr.is_some()
        {
            found = Some((now, poll));
            break;
        }
    }
    let (observed, poll) = found.expect("the PR is found");
    let delay = observed.0 - created.0;
    assert!(
        delay <= time::Duration::seconds(30),
        "detected {delay} after it was opened, interval is 30 s"
    );
    assert!(delay >= time::Duration::ZERO);

    let pr = poll.pr.as_ref().unwrap();
    assert_eq!(
        (pr.number, pr.head.as_str(), pr.base.as_str()),
        (24, BRANCH, "demo/base")
    );
    assert_eq!(pr.head_sha, HEAD_SHA);
    assert_eq!(
        pr.state,
        PrState::Closed,
        "the recorded PR was closed unmerged"
    );

    let template = CycleTemplate::engineering();
    let [opened] = poll.events.as_slice() else {
        panic!("one pr.opened: {:?}", poll.events);
    };
    assert_eq!(opened.kind, EventKind::PrOpened);
    assert_eq!(
        opened.lane,
        template.lane_of(&EventKind::PrOpened, opened.role)
    );
    assert_eq!(
        opened.at, created,
        "the event is stamped with the PR's creation time"
    );
    assert_eq!(opened.mission_id, mission());
    assert!(matches!(&opened.source, SourcePtr::GitHub { url } if url.ends_with("/pulls/24")));

    // not due again before the interval; due after it; the PR opens once
    assert!(source.poll_if_due(at(observed, 10)).unwrap().is_none());
    let again = source.poll_if_due(at(observed, 30)).unwrap().expect("due");
    assert!(again.events.is_empty(), "pr.opened is emitted once per PR");

    let dir = tempfile::tempdir().unwrap();
    let (mut store, conn) = store_with_mission(dir.path());
    store.record_poll(&mission(), &poll).unwrap();
    let (head, base, state, observed_at, url): (String, String, String, String, String) = conn
        .query_row(
            "SELECT head, base, state, observed_at, url FROM prs WHERE number = 24",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .unwrap();
    assert_eq!(
        (head.as_str(), base.as_str(), state.as_str()),
        (BRANCH, "demo/base", "closed")
    );
    assert_eq!(Timestamp::parse(&observed_at).unwrap(), observed);
    assert!(url.ends_with("/pulls/24"));
    let (kind, lane, src, event_url): (String, String, String, String) = conn
        .query_row(
            "SELECT kind, lane, source, url FROM events WHERE kind = 'pr.opened'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .unwrap();
    assert_eq!((kind.as_str(), src.as_str()), ("pr.opened", "github"));
    assert_eq!(lane, opened.lane.as_str());
    assert!(event_url.ends_with("/pulls/24"));
}

#[test]
fn recording_a_poll_twice_keeps_one_pr_row_and_its_first_observation() {
    let replay = Replay::new();
    let mut source = source(&replay, "demo/base");
    let first_at = Timestamp::parse("2026-10-29T10:00:00Z").unwrap();
    let first = source.poll_if_due(first_at).unwrap().unwrap();
    let second = source.poll_if_due(at(first_at, 30)).unwrap().unwrap();

    let dir = tempfile::tempdir().unwrap();
    let (mut store, conn) = store_with_mission(dir.path());
    store.record_poll(&mission(), &first).unwrap();
    store.record_poll(&mission(), &second).unwrap();
    let rows: i64 = conn
        .query_row("SELECT COUNT(*) FROM prs", [], |r| r.get(0))
        .unwrap();
    let opened: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM events WHERE kind = 'pr.opened'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let observed: String = conn
        .query_row("SELECT observed_at FROM prs WHERE number = 24", [], |r| {
            r.get(0)
        })
        .unwrap();
    let checks: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM checks WHERE pr_number = 24",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!((rows, opened, checks), (1, 1, 1));
    assert_eq!(Timestamp::parse(&observed).unwrap(), first_at);
    assert_eq!(store.unknown_count(&mission()).unwrap(), 0);
}

#[test]
fn target_branch_is_read_from_config_not_hardcoded() {
    for target in ["demo/base", "release/2026"] {
        let replay = Replay::new();
        let mut source = source(&replay, target);
        source
            .poll_if_due(Timestamp::parse("2026-10-29T10:00:00Z").unwrap())
            .unwrap();
        let asked = replay.asked();
        let lookup = asked
            .iter()
            .find(|path| path.contains("/pulls?"))
            .expect("a pull request lookup");
        assert!(
            lookup.contains(&format!("base={target}")),
            "{lookup} names the configured target"
        );
        assert!(
            lookup.contains("head=dctellya:demo/smoke-1"),
            "{lookup} names the owner and the mission branch"
        );
        assert!(
            asked.iter().all(|path| !path.contains("base=main")),
            "no request targets main: {asked:?}"
        );
    }

    for file in ["forge.rs", "github.rs"] {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/ingest")
            .join(file);
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(!text.contains("\"main\""), "{file} holds no literal main");
    }
}

fn comment_free(text: &str) -> Vec<(usize, String)> {
    text.lines()
        .enumerate()
        .filter(|(_, line)| !line.trim_start().starts_with("//"))
        .map(|(index, line)| (index + 1, line.to_owned()))
        .collect()
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn check_runs_api_is_never_called() {
    let replay = Replay::new();
    let mut source = source(&replay, "demo/base");
    source
        .poll_if_due(Timestamp::parse("2026-10-29T10:00:00Z").unwrap())
        .unwrap();

    let asked = replay.asked();
    let prefix = format!("/repos/{REPO}");
    assert_eq!(asked.len(), 3, "pulls, runs, status: {asked:?}");
    assert!(
        asked[0].starts_with(&format!("{prefix}/pulls?")),
        "{asked:?}"
    );
    assert_eq!(
        asked[1],
        format!("{prefix}/actions/runs?head_sha={HEAD_SHA}")
    );
    assert_eq!(asked[2], format!("{prefix}/commits/{HEAD_SHA}/status"));
    assert!(
        asked
            .iter()
            .all(|path| !path.contains("check-runs") && !path.contains("check-suites")),
        "{asked:?}"
    );

    // the client has no function for it either
    let mut files = Vec::new();
    rust_files(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut files,
    );
    for file in files {
        let text = std::fs::read_to_string(&file).unwrap();
        for (number, line) in comment_free(&text) {
            assert!(
                !line.contains("check-runs") && !line.contains("check-suites"),
                "{}:{number}: {line}",
                file.display()
            );
        }
    }
}

#[test]
fn checks_come_from_actions_runs_and_commit_statuses() {
    let replay = Replay::new();
    let client = GitHubClient::new(replay.clone(), REPO);
    let pr = client.pr(BRANCH, "demo/base").unwrap().expect("the PR");
    let checks = client.checks(&pr).unwrap();
    let [check] = checks.as_slice() else {
        panic!("the latest run of demo-gate only: {checks:?}");
    };
    assert_eq!(check.name, "demo-gate");
    assert_eq!(
        check.source,
        CheckSource::ActionsRun {
            run_id: 40_853_894_595
        }
    );
    assert_eq!(
        check.conclusion,
        CheckConclusion::Passed,
        "run 3 passed after run 2 failed"
    );
    assert!(check.url.contains("/actions/runs/40853894595"));

    let path = format!("/repos/{REPO}/commits/{HEAD_SHA}/status");
    replay.answer(
        &path,
        200,
        r#"{"state":"failure","statuses":[
            {"context":"ci/lint","state":"success","target_url":"https://ci.example.invalid/1"},
            {"context":"ci/unit","state":"failure","target_url":"https://ci.example.invalid/2"},
            {"context":"ci/slow","state":"pending","target_url":null}]}"#,
    );
    let checks = client.checks(&pr).unwrap();
    let status = |name: &str| {
        checks
            .iter()
            .find(|check| check.name == name)
            .unwrap_or_else(|| panic!("{name} in {checks:?}"))
    };
    assert_eq!(status("ci/lint").conclusion, CheckConclusion::Passed);
    assert_eq!(status("ci/unit").conclusion, CheckConclusion::Failed);
    assert_eq!(status("ci/slow").conclusion, CheckConclusion::Running);
    assert_eq!(
        status("ci/unit").source,
        CheckSource::CommitStatus {
            context: "ci/unit".into()
        }
    );
}

#[test]
fn http_errors_become_forge_errors_with_the_status_and_url() {
    let replay = Replay::new();
    let client = GitHubClient::new(replay.clone(), REPO);

    let missing =
        format!("/repos/{REPO}/pulls?head=dctellya:demo/smoke-1&base=demo/base&state=all");
    replay.answer(&missing, 404, r#"{"message":"Not Found"}"#);
    match client.pr(BRANCH, "demo/base") {
        Err(ForgeError::Http { status: 404, url }) => assert!(url.ends_with("state=all"), "{url}"),
        other => panic!("404 is an Http error: {other:?}"),
    }

    let limited = HttpResponse {
        status: 403,
        body: r#"{"message":"API rate limit exceeded"}"#.into(),
        rate_limit_remaining: Some(0),
        rate_limit_reset: Some(1_793_000_000),
    };
    let client = GitHubClient::new(Limited(limited), REPO);
    assert!(matches!(
        client.pr(BRANCH, "demo/base"),
        Err(ForgeError::RateLimited {
            reset_at: Some(1_793_000_000)
        })
    ));
}

struct Limited(HttpResponse);

impl HttpTransport for Limited {
    fn get(&self, _url: &str) -> Result<HttpResponse, String> {
        Ok(self.0.clone())
    }
}

/// Everything the process logs. A global subscriber, not `with_default`: tracing caches a
/// callsite's interest per process, and a test that ran first without a subscriber would
/// silence the callsite for a scoped one.
static LOG: Mutex<Vec<u8>> = Mutex::new(Vec::new());

struct Capture;

impl Write for Capture {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        LOG.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Serves the recorded routes over loopback HTTP and keeps each request's header block.
fn serve(connections: usize) -> (String, std::thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let routes = recorded_routes();
    let handle = std::thread::spawn(move || {
        let mut seen = Vec::new();
        for stream in listener.incoming().take(connections) {
            let mut stream = stream.unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut head = String::new();
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if line == "\r\n" || line.is_empty() {
                    break;
                }
                head.push_str(&line);
            }
            let target = head
                .split_whitespace()
                .nth(1)
                .unwrap_or_default()
                .to_owned();
            let (status, body) = routes
                .iter()
                .find(|route| route.0 == target)
                .map_or((404, "{}".to_owned()), |route| (route.1, route.2.clone()));
            write!(
                stream,
                "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
            let mut rest = Vec::new();
            let _ = stream.set_read_timeout(Some(Duration::from_millis(50)));
            let _ = reader.read_to_end(&mut rest);
            seen.push(head);
        }
        seen
    });
    (base, handle)
}

#[test]
fn token_is_redacted_from_logs() {
    let (base, server) = serve(3);
    let transport = UreqTransport::new(Some(Secret::new(TOKEN.into())), Duration::from_secs(5));
    let client = GitHubClient::new(transport, REPO).with_base_url(&base);
    let mut source = GitHubSource::new(
        client,
        mission(),
        BRANCH,
        &config("demo/base"),
        CycleTemplate::engineering(),
    );

    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .with_ansi(false)
        .with_writer(|| Capture)
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();
    let poll = source
        .poll_if_due(Timestamp::parse("2026-10-29T10:00:00Z").unwrap())
        .unwrap()
        .expect("due");
    let requests = server.join().unwrap();

    assert_eq!(
        poll.pr.as_ref().map(|pr| pr.number),
        Some(24),
        "the loopback server answered"
    );
    assert_eq!(requests.len(), 3);
    for head in &requests {
        let lower = head.to_ascii_lowercase();
        assert!(
            lower.contains(&format!(
                "authorization: bearer {}",
                TOKEN.to_ascii_lowercase()
            )),
            "the token is sent to the API: {head}"
        );
    }

    let log = String::from_utf8(LOG.lock().unwrap().clone()).unwrap();
    let lines: Vec<&str> = log
        .lines()
        .filter(|line| line.contains("github.request") && line.contains(&base))
        .collect();
    assert_eq!(lines.len(), 3, "one line per request: {log}");
    assert!(
        lines[0].contains("GET") && lines[0].contains("status=200"),
        "{}",
        lines[0]
    );
    assert!(
        lines[0].contains(&format!(
            "{base}/repos/{REPO}/pulls?head=dctellya:demo/smoke-1&base=demo/base&state=all"
        )),
        "{}",
        lines[0]
    );
    assert!(lines[1].contains("/actions/runs?head_sha="), "{}", lines[1]);
    assert!(lines[2].contains("/status"), "{}", lines[2]);
    let lower = log.to_ascii_lowercase();
    for needle in [TOKEN, "github_pat_", "authorization", "bearer"] {
        assert!(
            !lower.contains(&needle.to_ascii_lowercase()),
            "the log holds {needle}"
        );
    }
    assert!(!format!("{poll:?}").contains("github_pat_"));

    let dir = tempfile::tempdir().unwrap();
    let (mut store, _conn) = store_with_mission(dir.path());
    store.record_poll(&mission(), &poll).unwrap();
    drop(store);
    let bytes = std::fs::read(dir.path().join("o.sqlite")).unwrap();
    let wal = std::fs::read(dir.path().join("o.sqlite-wal")).unwrap_or_default();
    for file in [bytes, wal] {
        assert!(!file.windows(11).any(|window| window == b"github_pat_"));
    }
}

#[test]
fn the_role_of_a_pr_event_is_the_forge_not_a_person() {
    let replay = Replay::new();
    let mut source = source(&replay, "demo/base");
    let poll = source
        .poll_if_due(Timestamp::parse("2026-10-29T10:00:00Z").unwrap())
        .unwrap()
        .unwrap();
    assert!(poll.events.iter().all(|event| event.role == Role::External));
    assert!(poll.events.iter().all(|event| event.overlay.is_none()));
}
