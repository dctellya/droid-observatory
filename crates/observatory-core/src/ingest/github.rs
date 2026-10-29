//! `GitHubClient` (the `Forge` over GitHub's REST API) and `GitHubSource`, which polls it for
//! the mission branch's pull request (architecture §4.6, FR-G3).
//!
//! The client reads three things: the pull request of a branch, the Actions runs of its head and
//! that head's commit statuses. The token has no Checks permission (planning note 6), so there is
//! no function for the other API and nothing here names it. Every request is logged as
//! `github.request{method, url, status}`; headers and bodies never reach a log, and the token
//! only ever lives in the `Authorization` header `UreqTransport` builds.

use std::collections::{BTreeMap, HashSet};
use std::time::Duration;

use serde::Deserialize;
use serde::de::DeserializeOwned;

use super::forge::{
    Check, CheckConclusion, CheckSource, Forge, ForgeError, Poll, Pr, PrState, WindowChange,
};
use crate::config::GithubConfig;
use crate::mapping::MappingVersion;
use crate::model::{
    CycleTemplate, Event, EventId, EventKind, MilestoneId, MissionId, Role, SourcePtr, Timestamp,
};
use crate::secrets::Secret;

/// The production API host; tests point the client at a loopback server instead.
const API_BASE: &str = "https://api.github.com";
const API_VERSION: &str = "2022-11-28";

/// One response, reduced to what the client reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpResponse {
    pub status: u16,
    pub body: String,
    pub rate_limit_remaining: Option<u64>,
    pub rate_limit_reset: Option<u64>,
}

/// A blocking GET. An `Err` is a failure below HTTP (DNS, connect, TLS, timeout); an HTTP error
/// status is an `Ok` response.
pub trait HttpTransport {
    fn get(&self, url: &str) -> Result<HttpResponse, String>;
}

/// `ureq` with the system trust store and the token as a bearer header.
pub struct UreqTransport {
    agent: ureq::Agent,
    token: Option<Secret>,
}

impl UreqTransport {
    pub fn new(token: Option<Secret>, timeout: Duration) -> Self {
        // native-tls without webpki-roots has no roots of its own: name the system store.
        let tls = ureq::tls::TlsConfig::builder()
            .provider(ureq::tls::TlsProvider::NativeTls)
            .root_certs(ureq::tls::RootCerts::PlatformVerifier)
            .build();
        let config = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_global(Some(timeout))
            .tls_config(tls)
            .build();
        Self {
            agent: ureq::Agent::new_with_config(config),
            token,
        }
    }
}

impl HttpTransport for UreqTransport {
    fn get(&self, url: &str) -> Result<HttpResponse, String> {
        let mut request = self
            .agent
            .get(url)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", API_VERSION)
            .header("User-Agent", "droid-observatory");
        if let Some(token) = &self.token {
            request = request.header("Authorization", format!("Bearer {}", token.expose()));
        }
        let mut response = request.call().map_err(|error| error.to_string())?;
        let number = |name: &str| {
            response
                .headers()
                .get(name)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse::<u64>().ok())
        };
        let (remaining, reset) = (number("x-ratelimit-remaining"), number("x-ratelimit-reset"));
        let status = response.status().as_u16();
        let body = response
            .body_mut()
            .read_to_string()
            .map_err(|error| error.to_string())?;
        Ok(HttpResponse {
            status,
            body,
            rate_limit_remaining: remaining,
            rate_limit_reset: reset,
        })
    }
}

pub struct GitHubClient<T> {
    transport: T,
    base: String,
    repo: String,
}

impl<T: HttpTransport> GitHubClient<T> {
    /// A client for `repo` (`owner/name`).
    pub fn new(transport: T, repo: &str) -> Self {
        Self {
            transport,
            base: API_BASE.to_owned(),
            repo: repo.to_owned(),
        }
    }

    #[must_use]
    pub fn with_base_url(mut self, base: &str) -> Self {
        base.trim_end_matches('/').clone_into(&mut self.base);
        self
    }

    fn owner(&self) -> &str {
        self.repo.split_once('/').map_or("", |(owner, _)| owner)
    }

    fn get_json<D: DeserializeOwned>(&self, path_and_query: &str) -> Result<D, ForgeError> {
        let url = format!("{}/repos/{}{path_and_query}", self.base, self.repo);
        let response = match self.transport.get(&url) {
            Ok(response) => response,
            Err(error) => {
                tracing::warn!(method = "GET", url = %url, error = %error, "github.request");
                return Err(ForgeError::Other(format!("GET {url}: {error}")));
            }
        };
        tracing::info!(method = "GET", url = %url, status = response.status, "github.request");
        match response.status {
            200..=299 => serde_json::from_str(&response.body)
                .map_err(|error| ForgeError::Other(format!("GET {url}: unexpected body: {error}"))),
            429 => Err(ForgeError::RateLimited {
                reset_at: response.rate_limit_reset,
            }),
            403 if response.rate_limit_remaining == Some(0) => Err(ForgeError::RateLimited {
                reset_at: response.rate_limit_reset,
            }),
            status => Err(ForgeError::Http { status, url }),
        }
    }
}

impl GitHubClient<UreqTransport> {
    /// The client of `[github]`: the token from `GITHUB_TOKEN`, else the keychain. Without one the
    /// requests go out unauthenticated, which public repositories answer within a lower rate
    /// limit.
    pub fn from_config(config: &GithubConfig) -> Result<Self, crate::secrets::SecretError> {
        Ok(Self::with_token(config, crate::secrets::github_token()?))
    }

    /// The client of `[github]` for a token the caller already resolved, so one process reads
    /// the secret once however many clients it builds.
    pub fn with_token(config: &GithubConfig, token: Option<Secret>) -> Self {
        if token.is_none() {
            tracing::warn!(
                "no GitHub token in GITHUB_TOKEN or the keychain: requests are unauthenticated"
            );
        }
        let transport = UreqTransport::new(token, Duration::from_secs(config.request_timeout_s));
        Self::new(transport, &config.repo)
    }
}

#[derive(Deserialize)]
struct RawRef {
    #[serde(rename = "ref")]
    name: String,
    sha: String,
}

#[derive(Deserialize)]
struct RawPull {
    number: u64,
    url: String,
    state: String,
    created_at: String,
    merged_at: Option<String>,
    merge_commit_sha: Option<String>,
    head: RawRef,
    base: RawRef,
}

#[derive(Deserialize)]
struct RawRuns {
    workflow_runs: Vec<RawRun>,
}

#[derive(Deserialize)]
struct RawRun {
    id: u64,
    run_number: u64,
    name: Option<String>,
    status: String,
    conclusion: Option<String>,
    html_url: String,
    updated_at: Option<String>,
}

#[derive(Deserialize)]
struct RawStatuses {
    statuses: Vec<RawStatus>,
}

#[derive(Deserialize)]
struct RawStatus {
    context: String,
    state: String,
    target_url: Option<String>,
    updated_at: Option<String>,
}

fn parse_time(text: &str, what: &str) -> Result<Timestamp, ForgeError> {
    Timestamp::parse(text).map_err(|error| ForgeError::Other(format!("{what} {text:?}: {error}")))
}

fn parse_optional_time(text: Option<&str>, what: &str) -> Result<Option<Timestamp>, ForgeError> {
    text.map(|text| parse_time(text, what)).transpose()
}

impl RawPull {
    fn into_pr(self) -> Result<Pr, ForgeError> {
        let state = match (&self.merged_at, self.state.as_str()) {
            (Some(at), _) => PrState::Merged {
                at: parse_time(at, "merged_at")?,
                merge_sha: self.merge_commit_sha.clone(),
            },
            (None, "open") => PrState::Open,
            (None, _) => PrState::Closed,
        };
        Ok(Pr {
            number: self.number,
            head_sha: self.head.sha,
            head: self.head.name,
            base: self.base.name,
            url: self.url,
            created_at: parse_time(&self.created_at, "created_at")?,
            state,
        })
    }
}

/// The verdict of a workflow run, or `None` for a run that says nothing about the head
/// (cancelled by a newer push, skipped, neutral).
fn run_conclusion(run: &RawRun) -> Option<CheckConclusion> {
    if run.status != "completed" {
        return Some(match run.status.as_str() {
            "in_progress" => CheckConclusion::Running,
            _ => CheckConclusion::Queued,
        });
    }
    match run.conclusion.as_deref()? {
        "success" => Some(CheckConclusion::Passed),
        "failure" | "timed_out" | "startup_failure" | "action_required" => {
            Some(CheckConclusion::Failed)
        }
        _ => None,
    }
}

impl<T: HttpTransport> Forge for GitHubClient<T> {
    fn pr(&self, branch: &str, target: &str) -> Result<Option<Pr>, ForgeError> {
        let owner = self.owner();
        let pulls: Vec<RawPull> = self.get_json(&format!(
            "/pulls?head={owner}:{branch}&base={target}&state=all"
        ))?;
        pulls
            .into_iter()
            .filter(|pull| pull.head.name == branch && pull.base.name == target)
            .max_by_key(|pull| pull.number)
            .map(RawPull::into_pr)
            .transpose()
    }

    fn checks(&self, pr: &Pr) -> Result<Vec<Check>, ForgeError> {
        if pr.head_sha.is_empty() || !pr.head_sha.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(ForgeError::Other(format!(
                "PR #{} has head {:?}, not a commit",
                pr.number, pr.head_sha
            )));
        }
        let runs: RawRuns = self.get_json(&format!("/actions/runs?head_sha={}", pr.head_sha))?;
        let mut latest: BTreeMap<String, &RawRun> = BTreeMap::new();
        for run in &runs.workflow_runs {
            let Some(name) = run.name.clone() else {
                continue;
            };
            if run_conclusion(run).is_none() {
                continue;
            }
            if latest
                .get(&name)
                .is_none_or(|kept| kept.run_number < run.run_number)
            {
                latest.insert(name, run);
            }
        }
        let mut checks = Vec::new();
        for (name, run) in latest {
            let Some(conclusion) = run_conclusion(run) else {
                continue;
            };
            checks.push(Check {
                name,
                source: CheckSource::ActionsRun { run_id: run.id },
                conclusion,
                url: run.html_url.clone(),
                at: parse_optional_time(run.updated_at.as_deref(), "run updated_at")?,
            });
        }

        let statuses: RawStatuses = self.get_json(&format!("/commits/{}/status", pr.head_sha))?;
        let status_url = format!(
            "{}/repos/{}/commits/{}/status",
            self.base, self.repo, pr.head_sha
        );
        for status in statuses.statuses {
            checks.push(Check {
                name: status.context.clone(),
                source: CheckSource::CommitStatus {
                    context: status.context,
                },
                conclusion: match status.state.as_str() {
                    "success" => CheckConclusion::Passed,
                    "failure" | "error" => CheckConclusion::Failed,
                    _ => CheckConclusion::Running,
                },
                url: status
                    .target_url
                    .filter(|url| !url.is_empty())
                    .unwrap_or_else(|| status_url.clone()),
                at: parse_optional_time(status.updated_at.as_deref(), "status updated_at")?,
            });
        }
        Ok(checks)
    }
}

/// Polls a forge for the mission branch's pull request, on the `[github].poll_interval_s`
/// rhythm. The caller owns the clock: `poll_if_due(now)` says whether the interval has passed.
pub struct GitHubSource<F> {
    forge: F,
    mission: MissionId,
    branch: String,
    target: String,
    interval: Duration,
    template: CycleTemplate,
    last_poll: Option<Timestamp>,
    announced: HashSet<u64>,
    /// Event ids of the check results already turned into events; a run is one event.
    announced_checks: HashSet<EventId>,
    /// The card a red check sends back to the build lane.
    affected: Option<MilestoneId>,
}

impl<F: Forge> GitHubSource<F> {
    /// A source for the pull request from `branch` into `[github].target_branch`.
    pub fn new(
        forge: F,
        mission: MissionId,
        branch: &str,
        config: &GithubConfig,
        template: CycleTemplate,
    ) -> Self {
        Self {
            forge,
            mission,
            branch: branch.to_owned(),
            target: config.target_branch.clone(),
            interval: Duration::from_secs(config.poll_interval_s),
            template,
            last_poll: None,
            announced: HashSet::new(),
            announced_checks: HashSet::new(),
            affected: None,
        }
    }

    /// The card (milestone) the fix cycle of a red check belongs to: the caller knows which
    /// milestone was integrated last, the forge does not. Without one the check events still
    /// land in the Gate lane but open no window.
    pub fn set_affected_milestone(&mut self, milestone: Option<MilestoneId>) {
        self.affected = milestone;
    }

    pub fn forge(&self) -> &F {
        &self.forge
    }

    pub fn is_due(&self, now: Timestamp) -> bool {
        self.last_poll.is_none_or(|last| {
            Duration::try_from(now.0 - last.0).is_ok_and(|elapsed| elapsed >= self.interval)
        })
    }

    /// A poll when the interval has passed since the last one, else `None`. The interval counts
    /// from the attempt, so a failing forge is not asked again before it is due.
    pub fn poll_if_due(&mut self, now: Timestamp) -> Result<Option<Poll>, ForgeError> {
        if !self.is_due(now) {
            return Ok(None);
        }
        self.poll(now).map(Some)
    }

    pub fn poll(&mut self, now: Timestamp) -> Result<Poll, ForgeError> {
        self.last_poll = Some(now);
        let pr = self.forge.pr(&self.branch, &self.target)?;
        let Some(pr) = pr else {
            return Ok(Poll {
                observed_at: now,
                pr: None,
                checks: Vec::new(),
                events: Vec::new(),
                windows: Vec::new(),
            });
        };
        let checks = self.forge.checks(&pr)?;
        let mut events = Vec::new();
        if self.announced.insert(pr.number) {
            events.push(self.event(
                EventId(format!("github:pr:{}:opened", pr.number)),
                EventKind::PrOpened,
                pr.created_at,
                &pr.url,
                None,
            ));
        }
        let mut windows = Vec::new();
        for check in &checks {
            self.check_result(&pr, check, now, &mut events, &mut windows);
        }
        Ok(Poll {
            observed_at: now,
            pr: Some(pr),
            checks,
            events,
            windows,
        })
    }

    /// A finished run or status is one event, `check.failed` or `check.passed`, announced once;
    /// a result that is still running says nothing yet. The id names the run (or the status
    /// context on that commit), so a restarted source records the same event again, not a new one.
    fn check_result(
        &mut self,
        pr: &Pr,
        check: &Check,
        now: Timestamp,
        events: &mut Vec<Event>,
        windows: &mut Vec<WindowChange>,
    ) {
        let (kind, verb) = match check.conclusion {
            CheckConclusion::Failed => (EventKind::CheckFailed, "failed"),
            CheckConclusion::Passed => (EventKind::CheckPassed, "passed"),
            _ => return,
        };
        let subject = match &check.source {
            CheckSource::ActionsRun { run_id } => run_id.to_string(),
            CheckSource::CommitStatus { context } => format!("{}:{context}", pr.head_sha),
        };
        let id = EventId(format!("github:check:{}:{subject}:{verb}", pr.number));
        if !self.announced_checks.insert(id.clone()) {
            return;
        }
        events.push(self.event(
            id.clone(),
            kind,
            check.at.unwrap_or(now),
            &check.url,
            self.affected.clone(),
        ));
        if check.conclusion == CheckConclusion::Passed {
            windows.push(WindowChange::Close {
                event: id,
                pr_number: pr.number,
                check: check.name.clone(),
            });
        } else if let Some(card) = &self.affected {
            windows.push(WindowChange::Open {
                event: id,
                pr_number: pr.number,
                check: check.name.clone(),
                card: card.clone(),
            });
        }
    }

    fn event(
        &self,
        id: EventId,
        kind: EventKind,
        at: Timestamp,
        url: &str,
        milestone: Option<MilestoneId>,
    ) -> Event {
        let role = Role::External;
        Event {
            id,
            mission_id: self.mission.clone(),
            milestone_id: milestone,
            feature_id: None,
            lane: self.template.lane_of(&kind, role),
            overlay: None,
            kind,
            role,
            session_id: None,
            model: None,
            at,
            duration: None,
            usage: None,
            source: SourcePtr::GitHub {
                url: url.to_owned(),
            },
            mapping_version: MappingVersion::LATEST,
        }
    }
}
