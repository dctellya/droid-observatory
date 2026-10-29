//! Evidence, not a gate test: the real GitHub, the owner's token (keychain `droid-observatory/github-token`,
//! else `GITHUB_TOKEN`) and the demo pair. It is ignored so CI never runs it; run it with
//! `OBSERVATORY_LIVE_BRANCH=demo/<name> cargo test -p observatory-core --test forge_live -- --ignored --nocapture`.
//! It polls on the configured interval until the branch's pull request shows up and prints the
//! request log lines, the delay between the PR's creation and its detection, and the checks.
#![cfg(feature = "github")]

use std::time::{Duration, Instant};

use observatory_core::config::GithubConfig;
use observatory_core::ingest::github::{GitHubClient, GitHubSource};
use observatory_core::model::{CycleTemplate, MissionId, Timestamp};

#[test]
#[ignore = "calls the live GitHub API"]
fn the_live_demo_pair_is_polled_with_the_keychain_token() {
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
    let client = GitHubClient::from_config(&config).expect("the keychain answers");
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
