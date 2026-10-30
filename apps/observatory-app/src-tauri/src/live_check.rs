//! Live self-check of the hosting path with a real `droid` and a scratch mission (F-042). It runs
//! the same `host::launch`, pump, control channel and reaping as the window does, on Tauri's mock
//! runtime, so it needs no display. Ignored by default: it spends model tokens.
//!
//! ```sh
//! OBSERVATORY_LIVE_CONFIG=<scratch config.toml> OBSERVATORY_LIVE_REPO=<scratch repo> \
//! OBSERVATORY_LIVE_MISSION=<mission.md> cargo test -p observatory-app --lib \
//!     live_check -- --ignored --nocapture
//! ```

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use observatory_core::config::Config;
use observatory_core::control::HumanAction;
use observatory_core::ingest::AttachStatus;
use observatory_core::ingest::EventBatch;
use observatory_core::ingest::rpc::HostControl;
use observatory_core::model::{EventId, FeatureId};
use observatory_core::store::Store;
use observatory_core::view::{HostedStatus, LaunchForm, OutputLines, PermissionRow};
use tauri::{Listener, Manager};

use crate::Shell;
use crate::host::{Hosts, launch};
use crate::process::group_members;

fn env_path(name: &str) -> PathBuf {
    PathBuf::from(std::env::var(name).unwrap_or_else(|_| panic!("{name} is not set")))
}

fn now_nanos() -> i128 {
    let since = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the clock is after 1970");
    i128::try_from(since.as_nanos()).expect("nanoseconds fit")
}

/// One event as the `events_appended` listener saw it: when it was stamped (arrival of its line)
/// and when the pump emitted it to the window.
struct Seen {
    id: String,
    kind: String,
    at_nanos: i128,
    emitted_nanos: i128,
}

#[derive(Default)]
struct Log {
    seen: Vec<Seen>,
    output_total: u64,
    unknown_total: u64,
}

async fn wait_until<T>(what: &str, within: Duration, mut probe: impl FnMut() -> Option<T>) -> T {
    let started = Instant::now();
    loop {
        if let Some(found) = probe() {
            return found;
        }
        assert!(
            started.elapsed() < within,
            "gave up after {within:?} waiting for {what}"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

/// The orchestrator is a model: an interrupted or denied turn does not always end in a new
/// proposal, so a quiet minute is answered with a nudge, as a person would.
async fn await_proposal(hosts: &Hosts, id: &str, skip: &str) -> PermissionRow {
    for nudge in 0..4 {
        let started = Instant::now();
        while started.elapsed() < Duration::from_secs(60) {
            let status = hosts.status(id).expect("the run is registered");
            if let Some(found) = status
                .permissions
                .iter()
                .find(|p| p.request_id != skip && p.can_approve && p.can_deny)
            {
                return found.clone();
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        println!("no proposal after 60 s: nudge {nudge}");
        hosts
            .control(
                id,
                &HostControl::Message {
                    text: "Propose the mission now.".to_owned(),
                },
            )
            .expect("the nudge is written");
    }
    panic!("no mission proposal after four nudges");
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    let rank = (p * sorted.len() as f64).ceil().max(1.0) as usize;
    sorted[rank.min(sorted.len()) - 1]
}

fn spool_lines(path: &str) -> usize {
    std::fs::read_to_string(path).map_or(0, |text| text.lines().count())
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "spends model tokens: hosts a scratch mission with a real droid"]
async fn live_check_a_hosted_scratch_mission_takes_every_control() {
    let (cfg, origin) = Config::load_with_origin(Some(&env_path("OBSERVATORY_LIVE_CONFIG")))
        .expect("the scratch config loads");
    println!("config: {origin:?}");
    let store = Store::open(&cfg.store.resolved_path()).expect("the scratch store opens");
    let app = tauri::test::mock_app();
    app.manage(Shell {
        template: Arc::new(cfg.cycle.load_template().template),
        cfg: cfg.clone(),
        store: Mutex::new(store),
        status: Arc::new(Mutex::new(AttachStatus::default())),
    });
    // The host's own log lines (`host_control`, `human_action`, `stall_flag`) are evidence.
    let _ = tracing_subscriber::fmt()
        .with_ansi(false)
        .with_max_level(tracing::Level::INFO)
        .with_writer(std::io::stdout)
        .try_init();
    let (exclude, _excluded) = std::sync::mpsc::channel();
    app.manage(Hosts::new(exclude));
    let hosts = app.state::<Hosts>();
    let handle = app.handle().clone();
    crate::stall::spawn(handle.clone());
    let flags: Arc<Mutex<Vec<String>>> = Arc::default();
    let listened = flags.clone();
    handle.listen("stall_flag", move |event| {
        println!("stall_flag event: {}", event.payload());
        listened.lock().unwrap().push(event.payload().to_owned());
    });

    let log = Arc::new(Mutex::new(Log::default()));
    let listened = log.clone();
    handle.listen("events_appended", move |event| {
        let emitted_nanos = now_nanos();
        let batch: EventBatch = serde_json::from_str(event.payload()).expect("a batch");
        let mut log = listened.lock().unwrap();
        log.unknown_total = batch.unknown_total;
        for event in batch.events {
            log.seen.push(Seen {
                id: event.id.to_string(),
                kind: serde_json::to_value(&event.kind)
                    .ok()
                    .and_then(|v| v.as_str().map(str::to_owned))
                    .unwrap_or_default(),
                at_nanos: event.at.0.unix_timestamp_nanos(),
                emitted_nanos,
            });
        }
    });
    let listened = log.clone();
    handle.listen("live_output", move |event| {
        let lines: OutputLines = serde_json::from_str(event.payload()).expect("output lines");
        listened.lock().unwrap().output_total = lines.total;
    });
    let status: Arc<Mutex<Option<HostedStatus>>> = Arc::new(Mutex::new(None));
    let listened = status.clone();
    handle.listen("hosted_status", move |event| {
        *listened.lock().unwrap() = serde_json::from_str(event.payload()).ok();
    });

    let mut form = LaunchForm::defaults(&cfg.mission);
    form.repo = env_path("OBSERVATORY_LIVE_REPO").display().to_string();
    form.mission_file = env_path("OBSERVATORY_LIVE_MISSION").display().to_string();
    form.branch = "scratch/m2-host".to_owned();
    let config = form.validate().expect("the scratch form is complete");
    println!("models: {:?}", config.models);
    println!("efforts: {:?}", config.efforts);
    println!(
        "tags: {:?} skip_scrutiny: {} skip_user_testing: {}",
        config.tags, config.skip_scrutiny, config.skip_user_testing
    );
    let template = Arc::new(cfg.cycle.load_template().template);
    let launched = launch(&handle, &hosts, &cfg, &template, config)
        .await
        .expect("the scratch mission launches");
    println!("launched: {launched:?}");
    let id = launched.launch_id.clone();
    let pgid = launched.pid;
    let _group = KillGroup(pgid);
    println!("ps (group {pgid}):");
    print_group(pgid);

    let current = || hosts.status(&id).expect("the run is registered");
    let act = |what: &str, action: HumanAction, text: Option<&str>| {
        let at = now_nanos();
        let recorded = hosts
            .human_action(&handle, &id, action, text.map(str::to_owned))
            .unwrap_or_else(|err| panic!("{what} is refused: {err}"));
        println!(
            "control {what}: human_actions row {} overlay event {} at {at}",
            recorded.action_id, recorded.event_id
        );
        at
    };
    let first_after = |kind: &str, after: i128| {
        log.lock()
            .unwrap()
            .seen
            .iter()
            .find(|seen| seen.kind == kind && seen.emitted_nanos >= after)
            .map(|seen| (seen.id.clone(), seen.at_nanos))
    };
    // The acknowledgement events of `control::ack_event` carry the sub id 2^30.
    let acks_after = |after: i128| -> Vec<String> {
        log.lock()
            .unwrap()
            .seen
            .iter()
            .filter(|seen| seen.emitted_nanos >= after && seen.id.ends_with(":1073741824"))
            .map(|seen| format!("{} {}", seen.kind, seen.id))
            .collect()
    };
    let ack_of = |what: &'static str, after: i128| {
        wait_until(what, Duration::from_secs(60), move || {
            let acks = acks_after(after);
            (!acks.is_empty()).then_some(acks)
        })
    };

    let question = wait_until(
        "the orchestrator's question",
        Duration::from_secs(240),
        || current().questions.first().cloned(),
    )
    .await;
    let (question_event, question_at) =
        wait_until("the question event", Duration::from_secs(5), || {
            first_after("question_asked", 0)
        })
        .await;
    println!("question: event {question_event} at {question_at}");
    let hold: u64 = std::env::var("OBSERVATORY_LIVE_HOLD_S")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(0);
    if hold > 0 {
        println!("holding the question unanswered for {hold} s");
        tokio::time::sleep(Duration::from_secs(hold)).await;
    }
    println!("stall flags raised so far: {}", flags.lock().unwrap().len());

    let answered = act(
        "answer",
        HumanAction::answer(EventId(question.event_id.clone()), "salud"),
        Some("salud"),
    );
    let (answered_event, answered_at) =
        wait_until("intent_answered", Duration::from_secs(30), || {
            first_after("intent_answered", answered)
        })
        .await;
    println!(
        "answer -> intent.answered event {answered_event} at {answered_at} (question at {question_at}, later: {})",
        answered_at > question_at
    );

    let message = act(
        "message",
        HumanAction::message("Keep it to the one feature."),
        Some("Keep it to the one feature."),
    );
    println!("message -> {:?}", ack_of("the message ack", message).await);

    let paused = act("pause", HumanAction::Pause, None);
    let (paused_event, paused_at) = wait_until("mission_paused", Duration::from_secs(30), || {
        first_after("mission_paused", paused)
    })
    .await;
    println!(
        "pause -> mission.paused event {paused_event} at {paused_at} (question at {question_at}, later: {})",
        paused_at > question_at
    );
    let resume = act("resume", HumanAction::Resume, None);
    println!("resume -> {:?}", ack_of("the resume ack", resume).await);

    let proposal = await_proposal(&hosts, &id, "").await;
    println!(
        "proposal waits: request {} event {} tools {:?}",
        proposal.request_id, proposal.event_id, proposal.tools
    );
    let denied = act(
        "deny",
        HumanAction::Deny {
            request_event: EventId(proposal.event_id.clone()),
        },
        None,
    );
    println!("deny -> {:?}", ack_of("the deny ack", denied).await);
    let closed = current()
        .permissions
        .iter()
        .all(|p| p.request_id != proposal.request_id);
    println!("deny: proposal left the waiting list: {closed}");
    let again = act(
        "message",
        HumanAction::message("Denied once. Propose the same mission again."),
        Some("Denied once. Propose the same mission again."),
    );
    println!("after the denial: message at {again}");

    let second = await_proposal(&hosts, &id, &proposal.request_id).await;
    println!(
        "second proposal waits: request {} event {}",
        second.request_id, second.event_id
    );
    let approved = act(
        "approve",
        HumanAction::Approve {
            request_event: EventId(second.event_id.clone()),
        },
        None,
    );
    println!("approve -> {:?}", ack_of("the approve ack", approved).await);
    let closed = current()
        .permissions
        .iter()
        .all(|p| p.request_id != second.request_id);
    println!("approve: proposal left the waiting list: {closed}");

    act(
        "mark complete",
        HumanAction::MarkComplete {
            feature_id: FeatureId("write-salute".to_owned()),
        },
        None,
    );
    act(
        "mark skipped",
        HumanAction::mark_skipped(FeatureId("write-salute".to_owned()), "scratch: not needed"),
        Some("scratch: not needed"),
    );
    tokio::time::sleep(Duration::from_secs(5)).await;
    println!("acks after the marks: {:?}", acks_after(approved));
    println!(
        "stall flags raised in total: {}",
        flags.lock().unwrap().len()
    );

    // VAL-HOST-013: the count the panel shows (live_output total) against `wc -l` of the spool,
    // at two moments at least 30 s apart; the panel is read inside a 500 ms window of two
    // `wc -l` reads, the poll interval the contract allows.
    let mut moments = Vec::new();
    for moment in 0..2 {
        tokio::time::sleep(Duration::from_secs(if moment == 0 { 5 } else { 31 })).await;
        let before = spool_lines(&launched.spool);
        let read_at = now_nanos();
        tokio::time::sleep(Duration::from_millis(500)).await;
        let panel = log.lock().unwrap().output_total as usize;
        let after = spool_lines(&launched.spool);
        println!(
            "moment {moment}: at {read_at} wc_l_before {before} panel_lines {panel} wc_l_after {after}"
        );
        moments.push((before, panel, after));
    }
    assert!(moments[1].2 > moments[0].0, "the spool kept growing");

    tokio::time::sleep(Duration::from_secs(10)).await;
    let reaping = Instant::now();
    hosts
        .reap_all(Duration::from_millis(cfg.mission.reap_grace_ms))
        .await;
    println!("reaped in {:?}", reaping.elapsed());
    println!(
        "ps after reap (group {pgid}): members {}",
        group_members(pgid)
    );
    print_group(pgid);

    let log = log.lock().unwrap();
    let mut latencies: Vec<f64> = log
        .seen
        .iter()
        .map(|seen| (seen.emitted_nanos - seen.at_nanos) as f64 / 1_000_000.0)
        .collect();
    latencies.sort_by(f64::total_cmp);
    println!(
        "latency (arrival stamp to emit, ms) over {} events: p50 {:.1} p95 {:.1} max {:.1}",
        latencies.len(),
        percentile(&latencies, 0.50),
        percentile(&latencies, 0.95),
        latencies.last().copied().unwrap_or(0.0)
    );
    println!("unknown_total: {}", log.unknown_total);
    let settings = std::path::PathBuf::from(std::env::var("HOME").expect("HOME is set"))
        .join(".factory/missions")
        .join(&launched.session_id)
        .join("model-settings.json");
    let settings = std::fs::read_to_string(&settings).expect("the mission has model settings");
    println!("model-settings.json: {}", settings.replace('\n', " "));
    println!("spool: {}", launched.spool);
    println!("session: {}", launched.session_id);

    assert_eq!(group_members(pgid), 0, "the process group is gone");
    assert!(
        answered_at > question_at,
        "the answer is stamped after the question"
    );
    assert_eq!(log.unknown_total, 0, "no unknown events");
    assert_eq!(
        settings.matches("claude-haiku-4-5-20251001").count(),
        2,
        "worker and validator run the cheapest model: {settings}"
    );
    for (before, panel, after) in moments {
        assert!(
            before <= panel && panel <= after,
            "the panel count agrees with wc -l"
        );
    }
}

/// A failed wait panics before the reaping step; the group still must not outlive the test.
struct KillGroup(u32);

impl Drop for KillGroup {
    fn drop(&mut self) {
        let _ = std::process::Command::new("kill")
            .args(["-KILL", &format!("-{}", self.0)])
            .status();
    }
}

fn print_group(pgid: u32) {
    let out = std::process::Command::new("ps")
        .args(["-axo", "pid,pgid,command"])
        .output()
        .expect("ps runs");
    let text = String::from_utf8_lossy(&out.stdout);
    let rows: Vec<&str> = text
        .lines()
        .filter(|line| line.split_whitespace().nth(1) == Some(&pgid.to_string()))
        .collect();
    if rows.is_empty() {
        println!("  (no process in group {pgid})");
    }
    for row in rows {
        println!("  {row}");
    }
}
