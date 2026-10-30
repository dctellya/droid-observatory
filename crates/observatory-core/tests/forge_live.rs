//! An owner-run row (attended class, M3 item 18), not a gate test and never a worker's: the real GitHub, the
//! owner's token and the demo pair. It is ignored so CI never runs it, and it returns early unless
//! `FORGE_LIVE=1`. The token comes from `GITHUB_TOKEN` only; the keychain is never read here (each rebuilt
//! test binary has a new code hash and would prompt for the item anew). The owner runs it in his terminal:
//! `FORGE_LIVE=1 GITHUB_TOKEN="$(gh auth token)" OBSERVATORY_LIVE_BRANCH=demo/<name> cargo test -p observatory-core --test forge_live -- --ignored --nocapture`.
//! It polls on the configured interval until the branch's pull request shows up and prints the
//! request log lines, the delay between the PR's creation and its detection, and the checks.
#![cfg(feature = "github")]

use std::time::{Duration, Instant};

use observatory_core::config::{GithubConfig, MergePolicy};
use observatory_core::ingest::github::{GitHubClient, GitHubSource};
use observatory_core::model::{CycleTemplate, MilestoneId, MissionId, Timestamp};
use observatory_core::secrets::Secret;
use observatory_core::store::{EventScope, Store, TimeRange};
use observatory_core::view::GateGroup;

#[test]
#[ignore = "calls the live GitHub API"]
fn the_live_demo_pair_is_polled_with_the_owner_run_token() {
    if std::env::var("FORGE_LIVE").as_deref() != Ok("1") {
        println!("skipped: FORGE_LIVE unset (owner-run)");
        return;
    }
    let branch =
        std::env::var("OBSERVATORY_LIVE_BRANCH").unwrap_or_else(|_| "demo/smoke-1".to_owned());
    let wait = std::env::var("OBSERVATORY_LIVE_WAIT_S")
        .ok()
        .and_then(|text| text.parse().ok())
        .unwrap_or(120);
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_ansi(false)
        .with_writer(std::io::stdout)
        .init();
    let config = GithubConfig {
        target_branch: "demo/base".into(),
        ..GithubConfig::default()
    };
    let token = std::env::var("GITHUB_TOKEN")
        .ok()
        .filter(|token| !token.is_empty())
        .expect("GITHUB_TOKEN is set (the keychain is never read here)");
    let client = GitHubClient::with_token(&config, Some(Secret::new(token)));
    let mut source = GitHubSource::new(
        client,
        MissionId("live".into()),
        &branch,
        &config,
        CycleTemplate::engineering(),
    );

    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(wait) {
        let now = Timestamp::now();
        let Some(poll) = source.poll_if_due(now).expect("the poll succeeds") else {
            std::thread::sleep(Duration::from_millis(500));
            continue;
        };
        let Some(pr) = &poll.pr else { continue };
        println!(
            "pr #{} {} -> {} state={} created_at={} observed_at={} delay={}",
            pr.number,
            pr.head,
            pr.base,
            pr.state.wire(),
            pr.created_at.to_rfc3339_millis(),
            poll.observed_at.to_rfc3339_millis(),
            poll.observed_at.0 - pr.created_at.0
        );
        for check in &poll.checks {
            println!(
                "check {} {:?} {}",
                check.name,
                check.source,
                check.conclusion.wire()
            );
        }
        println!(
            "events {:?}",
            poll.events
                .iter()
                .map(|event| event.id.to_string())
                .collect::<Vec<_>>()
        );
        return;
    }
    panic!("no PR from {branch} into demo/base within {wait} s");
}

/// One poll of the live demo PR recorded into a store file, then what the store and the Gate header
/// say: run it while the PR is red, then again after the label is removed, with the same
/// `OBSERVATORY_LIVE_STORE`, and the second run closes the window the first one opened. Every run
/// builds a new source, so it also shows that a restart opens no second window.
/// `FORGE_LIVE=1 GITHUB_TOKEN="$(gh auth token)" OBSERVATORY_LIVE_BRANCH=demo/<name> OBSERVATORY_LIVE_STORE=/tmp/<file>.sqlite
/// OBSERVATORY_LIVE_CARD=<milestone> cargo test -p observatory-core --test forge_live the_live_fix_cycle -- --ignored --nocapture`.
#[test]
#[ignore = "calls the live GitHub API"]
fn the_live_fix_cycle_is_recorded_in_a_store_file() {
    if std::env::var("FORGE_LIVE").as_deref() != Ok("1") {
        println!("skipped: FORGE_LIVE unset (owner-run)");
        return;
    }
    let var = |name: &str| std::env::var(name).expect(name);
    let (branch, store_path, card) = (
        var("OBSERVATORY_LIVE_BRANCH"),
        var("OBSERVATORY_LIVE_STORE"),
        var("OBSERVATORY_LIVE_CARD"),
    );
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_ansi(false)
        .with_writer(std::io::stdout)
        .init();
    let config = GithubConfig {
        target_branch: "demo/base".into(),
        ..GithubConfig::default()
    };
    let token = std::env::var("GITHUB_TOKEN")
        .ok()
        .filter(|token| !token.is_empty())
        .expect("GITHUB_TOKEN is set (the keychain is never read here)");
    let mission = MissionId("live".into());
    let mut source = GitHubSource::new(
        GitHubClient::with_token(&config, Some(Secret::new(token))),
        mission.clone(),
        &branch,
        &config,
        CycleTemplate::engineering(),
    );
    source.set_affected_milestone(Some(MilestoneId(card)));

    let mut store = Store::open(std::path::Path::new(&store_path)).unwrap();
    let conn = rusqlite::Connection::open(&store_path).unwrap();
    conn.execute(
        "INSERT OR IGNORE INTO missions (id, name, state, started_at) VALUES ('live', 'live', 'at_gate', '2026-10-29T00:00:00Z')",
        [],
    )
    .unwrap();
    let poll = source.poll(Timestamp::now()).expect("the poll succeeds");
    store.record_poll(&mission, &poll).unwrap();

    let events = store
        .events(&EventScope::Mission(mission), TimeRange::ALL)
        .unwrap();
    println!("badge {}", GateGroup::of(&events).badge.as_str());
    let rows = |sql: &str| {
        let mut statement = conn.prepare(sql).unwrap();
        let columns = statement.column_count();
        statement
            .query_map([], |row| {
                (0..columns)
                    .map(|i| {
                        row.get::<_, rusqlite::types::Value>(i)
                            .map(|v| format!("{v:?}"))
                    })
                    .collect::<Result<Vec<_>, _>>()
            })
            .unwrap()
            .map(|row| row.unwrap().join(" | "))
            .collect::<Vec<_>>()
    };
    for sql in [
        "SELECT number, head, base, state FROM prs",
        "SELECT name, source, conclusion, url FROM checks",
        "SELECT id, kind, lane, source, url FROM events WHERE kind LIKE 'check.%' ORDER BY at",
        "SELECT id, feature_id, cause, opened_by, closed_by, check_name, pr_number FROM rework_windows",
        "SELECT id, state FROM missions",
    ] {
        println!("{sql}");
        for row in rows(sql) {
            println!("  {row}");
        }
    }
}

/// One poll of the live demo PR under the `[github].merge_policy` named by `OBSERVATORY_LIVE_POLICY`
/// (`auto`, `human` or `policy`), recorded into `OBSERVATORY_LIVE_STORE`, then the Gate header's
/// `decided_by`, `requires_human` and `merge_offered`, and the rows a merge adds: `prs.merged_by`,
/// `handed_to_cd` (role, sha, `at`) and `missions.completed_at`. Run it against an open PR for the
/// attribution rows, and again after the owner merges it (the token owner's own merge reads `human`,
/// another login's reads `external`).
/// `FORGE_LIVE=1 GITHUB_TOKEN="$(gh auth token)" OBSERVATORY_LIVE_BRANCH=demo/<name> OBSERVATORY_LIVE_POLICY=auto
/// OBSERVATORY_LIVE_STORE=/tmp/<file>.sqlite cargo test -p observatory-core --test forge_live the_live_gate -- --ignored --nocapture`.
#[test]
#[ignore = "calls the live GitHub API"]
fn the_live_gate_names_the_rule_and_records_a_merge() {
    if std::env::var("FORGE_LIVE").as_deref() != Ok("1") {
        println!("skipped: FORGE_LIVE unset (owner-run)");
        return;
    }
    let var = |name: &str| std::env::var(name).expect(name);
    let (branch, store_path) = (
        var("OBSERVATORY_LIVE_BRANCH"),
        var("OBSERVATORY_LIVE_STORE"),
    );
    let merge_policy = match var("OBSERVATORY_LIVE_POLICY").as_str() {
        "human" => MergePolicy::Human,
        "policy" => MergePolicy::Policy,
        _ => MergePolicy::Auto,
    };
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_ansi(false)
        .with_writer(std::io::stdout)
        .init();
    let config = GithubConfig {
        target_branch: "demo/base".into(),
        merge_policy,
        ..GithubConfig::default()
    };
    let token = std::env::var("GITHUB_TOKEN")
        .ok()
        .filter(|token| !token.is_empty())
        .expect("GITHUB_TOKEN is set (the keychain is never read here)");
    let mission = MissionId("live".into());
    let mut source = GitHubSource::new(
        GitHubClient::with_token(&config, Some(Secret::new(token))),
        mission.clone(),
        &branch,
        &config,
        CycleTemplate::engineering(),
    );
    let mut store = Store::open(std::path::Path::new(&store_path)).unwrap();
    let conn = rusqlite::Connection::open(&store_path).unwrap();
    conn.execute(
        "INSERT OR IGNORE INTO missions (id, name, state, started_at) VALUES ('live', 'live', 'at_gate', '2026-10-29T00:00:00Z')",
        [],
    )
    .unwrap();
    let poll = source.poll(Timestamp::now()).expect("the poll succeeds");
    store.record_poll(&mission, &poll).unwrap();
    let events = store
        .events(&EventScope::Mission(mission), TimeRange::ALL)
        .unwrap();
    let mut group = GateGroup::of(&events);
    if let Some(facts) = &poll.gate {
        group = group.with_facts(facts);
    }
    println!(
        "badge {} decided_by {:?} requires_human {} merge_offered {} required {:?} reviews {} attestations {:?}",
        group.badge.as_str(),
        group.decided_by,
        group.requires_human,
        group.merge_offered,
        group.required_checks,
        group.reviews,
        group.attestation_ids
    );
    for sql in [
        "SELECT number, state, merged_by FROM prs",
        "SELECT COUNT(*) FROM reviews",
        "SELECT COUNT(*) FROM attestations",
        "SELECT kind, role, lane, sha, url, at FROM events WHERE kind = 'handed_to_cd'",
        "SELECT state, completed_at FROM missions WHERE id = 'live'",
    ] {
        println!("{sql}");
        let mut statement = conn.prepare(sql).unwrap();
        let columns = statement.column_count();
        let rows = statement
            .query_map([], |row| {
                (0..columns)
                    .map(|i| {
                        row.get::<_, rusqlite::types::Value>(i)
                            .map(|v| format!("{v:?}"))
                    })
                    .collect::<Result<Vec<_>, _>>()
            })
            .unwrap();
        for row in rows {
            println!("  {}", row.unwrap().join(" | "));
        }
    }
}
