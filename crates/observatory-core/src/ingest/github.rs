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
    Attestation, Check, CheckConclusion, CheckSource, Forge, ForgeError, GateFacts, MergeMethod,
    MergePolicy, Merged, Poll, Pr, PrState, RequiredCheck, Review, RuleAttribution, WindowChange,
};
use super::git::{FastCheckRun, RunConclusion};
use crate::config::{GithubConfig, MergePolicy as ConfigMergePolicy};
use crate::mapping::MappingVersion;
use crate::model::{
    CycleTemplate, Event, EventId, EventKind, MilestoneId, MissionId, Role, SourcePtr, Timestamp,
};
use crate::secrets::Secret;

/// The production API host; tests point the client at a loopback server instead.
const API_BASE: &str = "https://api.github.com";
const API_VERSION: &str = "2022-11-28";
/// The workflow that runs the fast check on every push to a mission branch.
const MISSION_BRANCH_WORKFLOW: &str = "mission-branch.yml";
/// The most runs the API returns per page; a branch has far fewer than this many in flight.
const RUNS_PER_PAGE: u32 = 100;

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

    /// A blocking PUT with a JSON body, the verb of the merge. A transport that never writes
    /// keeps the default.
    fn put(&self, _url: &str, _body: &str) -> Result<HttpResponse, String> {
        Err("this transport does not write".to_owned())
    }
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

impl UreqTransport {
    fn bearer(&self) -> Option<String> {
        self.token
            .as_ref()
            .map(|token| format!("Bearer {}", token.expose()))
    }
}

/// The status, body and rate-limit headers of a response, read once.
fn read_response(mut response: ureq::http::Response<ureq::Body>) -> Result<HttpResponse, String> {
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

impl HttpTransport for UreqTransport {
    fn get(&self, url: &str) -> Result<HttpResponse, String> {
        let mut request = self
            .agent
            .get(url)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", API_VERSION)
            .header("User-Agent", "droid-observatory");
        if let Some(bearer) = self.bearer() {
            request = request.header("Authorization", bearer);
        }
        read_response(request.call().map_err(|error| error.to_string())?)
    }

    fn put(&self, url: &str, body: &str) -> Result<HttpResponse, String> {
        let mut request = self
            .agent
            .put(url)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", API_VERSION)
            .header("User-Agent", "droid-observatory")
            .header("Content-Type", "application/json");
        if let Some(bearer) = self.bearer() {
            request = request.header("Authorization", bearer);
        }
        read_response(request.send(body).map_err(|error| error.to_string())?)
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

    fn repo_url(&self, path_and_query: &str) -> String {
        format!("{}/repos/{}{path_and_query}", self.base, self.repo)
    }

    /// One request, logged as `github.request{method, url, status}`; the body of a PUT and every
    /// header stay out of the log.
    fn call(
        &self,
        method: &str,
        url: &str,
        body: Option<&str>,
    ) -> Result<HttpResponse, ForgeError> {
        let sent = match body {
            Some(body) => self.transport.put(url, body),
            None => self.transport.get(url),
        };
        match sent {
            Ok(response) => {
                tracing::info!(method, url = %url, status = response.status, "github.request");
                Ok(response)
            }
            Err(error) => {
                tracing::warn!(method, url = %url, error = %error, "github.request");
                Err(ForgeError::Other(format!("{method} {url}: {error}")))
            }
        }
    }

    /// The body of a 2xx answer; the other statuses as the error they are.
    fn ok_body(url: &str, response: HttpResponse) -> Result<String, ForgeError> {
        match response.status {
            200..=299 => Ok(response.body),
            429 => Err(ForgeError::RateLimited {
                reset_at: response.rate_limit_reset,
            }),
            403 if response.rate_limit_remaining == Some(0) => Err(ForgeError::RateLimited {
                reset_at: response.rate_limit_reset,
            }),
            status => Err(ForgeError::Http {
                status,
                url: url.to_owned(),
            }),
        }
    }

    fn get_url<D: DeserializeOwned>(&self, url: &str) -> Result<D, ForgeError> {
        let body = Self::ok_body(url, self.call("GET", url, None)?)?;
        serde_json::from_str(&body)
            .map_err(|error| ForgeError::Other(format!("GET {url}: unexpected body: {error}")))
    }

    fn get_json<D: DeserializeOwned>(&self, path_and_query: &str) -> Result<D, ForgeError> {
        self.get_url(&self.repo_url(path_and_query))
    }

    /// A `GET` whose 404 means "nothing there", as for a branch without protection.
    fn get_json_or_none<D: DeserializeOwned>(
        &self,
        path_and_query: &str,
    ) -> Result<Option<D>, ForgeError> {
        match self.get_json(path_and_query) {
            Ok(found) => Ok(Some(found)),
            Err(ForgeError::Http { status: 404, .. }) => Ok(None),
            Err(error) => Err(error),
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
        Self::new(transport, &config.repo).with_base_url(&config.api_base)
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
    auto_merge: Option<serde_json::Value>,
    /// Only the single-PR endpoint carries it.
    merged_by: Option<RawUser>,
}

#[derive(Deserialize)]
struct RawUser {
    login: String,
}

#[derive(Deserialize)]
struct RawRule {
    #[serde(rename = "type")]
    kind: String,
    parameters: Option<serde_json::Value>,
    ruleset_id: Option<u64>,
}

#[derive(Deserialize)]
struct RawRuleset {
    id: u64,
    name: String,
}

#[derive(Deserialize)]
struct RawProtection {
    required_status_checks: Option<RawProtectedChecks>,
    required_pull_request_reviews: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct RawProtectedChecks {
    #[serde(default)]
    contexts: Vec<String>,
    #[serde(default)]
    checks: Vec<RawContext>,
}

#[derive(Deserialize)]
struct RawContext {
    context: String,
}

#[derive(Deserialize)]
struct RawReview {
    id: u64,
    state: String,
    html_url: Option<String>,
}

#[derive(Deserialize)]
struct RawArtifacts {
    artifacts: Vec<RawArtifact>,
}

#[derive(Deserialize)]
struct RawArtifact {
    digest: Option<String>,
}

#[derive(Deserialize)]
struct RawAttestations {
    attestations: Vec<RawAttestation>,
}

#[derive(Deserialize)]
struct RawAttestation {
    id: Option<serde_json::Value>,
    bundle_url: Option<String>,
}

#[derive(Deserialize)]
struct RawMerged {
    sha: String,
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
    /// Absent from the recorded fixtures of the per-head poll, which never read them.
    #[serde(default)]
    head_sha: Option<String>,
    #[serde(default)]
    run_started_at: Option<String>,
    #[serde(default)]
    created_at: Option<String>,
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
                merged_by: self.merged_by.map(|user| user.login),
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
            auto_merge: self.auto_merge.is_some_and(|value| !value.is_null()),
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

/// The newest run of each workflow, by run number, leaving out the runs that say nothing.
fn latest_runs(runs: &RawRuns) -> BTreeMap<String, &RawRun> {
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
    latest
}

/// The id of an attestation: the forge's own when it gives one, else the subject and position.
fn attestation_id(raw: &RawAttestation, subject: &str, index: usize) -> String {
    match &raw.id {
        Some(serde_json::Value::Number(number)) => number.to_string(),
        Some(serde_json::Value::String(text)) if !text.is_empty() => text.clone(),
        _ => format!("{subject}#{index}"),
    }
}

impl<T: HttpTransport> GitHubClient<T> {
    /// The runs of the `mission-branch.yml` workflow on `branch`, newest first as the forge
    /// lists them, as fast-check runs: the head they ran on, when they started and, once they
    /// have a verdict, when it came. A run the forge reports without a head or a start time says
    /// nothing a gate event could be attributed by and is left out.
    pub fn mission_branch_runs(&self, branch: &str) -> Result<Vec<FastCheckRun>, ForgeError> {
        let listed: RawRuns = self.get_json(&format!(
            "/actions/workflows/{MISSION_BRANCH_WORKFLOW}/runs?branch={branch}&per_page={RUNS_PER_PAGE}"
        ))?;
        let mut runs = Vec::with_capacity(listed.workflow_runs.len());
        for run in listed.workflow_runs {
            let (Some(head_sha), Some(started)) = (
                run.head_sha.clone(),
                run.run_started_at.as_deref().or(run.created_at.as_deref()),
            ) else {
                continue;
            };
            let started_at = parse_time(started, "run_started_at")?;
            let completed = run.status == "completed";
            runs.push(FastCheckRun {
                run_id: run.id,
                url: run.html_url.clone(),
                head_sha,
                started_at,
                finished_at: if completed {
                    parse_optional_time(run.updated_at.as_deref(), "updated_at")?
                } else {
                    None
                },
                conclusion: run
                    .conclusion
                    .as_deref()
                    .filter(|_| completed)
                    .and_then(RunConclusion::from_github),
            });
        }
        Ok(runs)
    }

    fn head_runs(&self, head_sha: &str) -> Result<RawRuns, ForgeError> {
        self.get_json(&format!("/actions/runs?head_sha={head_sha}"))
    }

    fn ruleset_name(&self, id: u64) -> String {
        let listed: Result<Vec<RawRuleset>, _> = self.get_json("/rulesets");
        match listed {
            Ok(rulesets) => rulesets
                .into_iter()
                .find(|ruleset| ruleset.id == id)
                .map(|ruleset| ruleset.name),
            Err(error) => {
                tracing::warn!(%error, "github ruleset names unavailable");
                None
            }
        }
        .unwrap_or_else(|| format!("ruleset {id}"))
    }

    /// The policy the rulesets that apply to `target` make, if any rule does.
    fn ruleset_policy(&self, rules: &[RawRule]) -> MergePolicy {
        let pull_rule = rules.iter().find(|rule| rule.kind == "pull_request");
        let id = pull_rule
            .or_else(|| rules.first())
            .and_then(|rule| rule.ruleset_id)
            .unwrap_or_default();
        let methods = pull_rule
            .and_then(|rule| rule.parameters.as_ref())
            .and_then(|parameters| parameters["allowed_merge_methods"].as_array())
            .map(|methods| {
                methods
                    .iter()
                    .filter_map(|method| method.as_str().and_then(MergeMethod::parse))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_else(|| vec![MergeMethod::Merge, MergeMethod::Squash, MergeMethod::Rebase]);
        let mut required_checks: Vec<String> = Vec::new();
        for rule in rules
            .iter()
            .filter(|rule| rule.kind == "required_status_checks")
        {
            let contexts = rule
                .parameters
                .as_ref()
                .and_then(|parameters| parameters["required_status_checks"].as_array())
                .into_iter()
                .flatten()
                .filter_map(|check| check["context"].as_str());
            for context in contexts {
                if !required_checks.iter().any(|known| known == context) {
                    required_checks.push(context.to_owned());
                }
            }
        }
        MergePolicy {
            requires_human: pull_rule.is_some(),
            allowed_methods: methods,
            decided_by: RuleAttribution::Ruleset {
                id,
                name: self.ruleset_name(id),
            },
            required_checks,
        }
    }
}

impl<T: HttpTransport> Forge for GitHubClient<T> {
    fn pr(&self, branch: &str, target: &str) -> Result<Option<Pr>, ForgeError> {
        let owner = self.owner();
        let pulls: Vec<RawPull> = self.get_json(&format!(
            "/pulls?head={owner}:{branch}&base={target}&state=all"
        ))?;
        let newest = pulls
            .into_iter()
            .filter(|pull| pull.head.name == branch && pull.base.name == target)
            .max_by_key(|pull| pull.number)
            .map(RawPull::into_pr)
            .transpose()?;
        let Some(mut pr) = newest else {
            return Ok(None);
        };
        // The list leaves out who merged; a wrong attribution would be recorded once and kept, so
        // a failure here fails the poll instead of guessing.
        if let PrState::Merged { merged_by, .. } = &mut pr.state {
            let single: RawPull = self.get_json(&format!("/pulls/{}", pr.number))?;
            *merged_by = single.merged_by.map(|user| user.login);
        }
        Ok(Some(pr))
    }

    fn checks(&self, pr: &Pr) -> Result<Vec<Check>, ForgeError> {
        if pr.head_sha.is_empty() || !pr.head_sha.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(ForgeError::Other(format!(
                "PR #{} has head {:?}, not a commit",
                pr.number, pr.head_sha
            )));
        }
        let runs = self.head_runs(&pr.head_sha)?;
        let mut checks = Vec::new();
        for (name, run) in latest_runs(&runs) {
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

    fn policy(&self, target: &str) -> Result<MergePolicy, ForgeError> {
        let rules: Vec<RawRule> = self.get_json(&format!("/rules/branches/{target}"))?;
        if !rules.is_empty() {
            return Ok(self.ruleset_policy(&rules));
        }
        let protection: Option<RawProtection> =
            self.get_json_or_none(&format!("/branches/{target}/protection"))?;
        let Some(protection) = protection else {
            return Ok(MergePolicy {
                requires_human: false,
                allowed_methods: vec![MergeMethod::Merge, MergeMethod::Squash, MergeMethod::Rebase],
                decided_by: RuleAttribution::NoRule,
                required_checks: Vec::new(),
            });
        };
        let mut required_checks = Vec::new();
        if let Some(checks) = protection.required_status_checks {
            let named = checks
                .contexts
                .into_iter()
                .chain(checks.checks.into_iter().map(|check| check.context));
            for context in named {
                if !required_checks.contains(&context) {
                    required_checks.push(context);
                }
            }
        }
        Ok(MergePolicy {
            requires_human: protection.required_pull_request_reviews.is_some(),
            allowed_methods: vec![MergeMethod::Merge, MergeMethod::Squash, MergeMethod::Rebase],
            decided_by: RuleAttribution::BranchProtection,
            required_checks,
        })
    }

    fn reviews(&self, pr: &Pr) -> Result<Vec<Review>, ForgeError> {
        let reviews: Vec<RawReview> = self.get_json(&format!("/pulls/{}/reviews", pr.number))?;
        Ok(reviews
            .into_iter()
            .map(|review| Review {
                id: review.id,
                state: review.state.to_ascii_lowercase(),
                url: review.html_url.unwrap_or_default(),
            })
            .collect())
    }

    fn attestations(&self, pr: &Pr) -> Result<Vec<Attestation>, ForgeError> {
        let runs = self.head_runs(&pr.head_sha)?;
        let mut found = Vec::new();
        for run in latest_runs(&runs).values() {
            if run_conclusion(run) != Some(CheckConclusion::Passed) {
                continue;
            }
            let artifacts: Option<RawArtifacts> =
                self.get_json_or_none(&format!("/actions/runs/{}/artifacts", run.id))?;
            let digests = artifacts
                .into_iter()
                .flat_map(|list| list.artifacts)
                .filter_map(|artifact| artifact.digest);
            for subject in digests {
                let listed: Option<RawAttestations> =
                    self.get_json_or_none(&format!("/attestations/{subject}"))?;
                for (index, raw) in listed.into_iter().flat_map(|l| l.attestations).enumerate() {
                    found.push(Attestation {
                        id: attestation_id(&raw, &subject, index),
                        url: raw.bundle_url.unwrap_or_default(),
                        subject: subject.clone(),
                        verified: false,
                    });
                }
            }
        }
        Ok(found)
    }

    fn token_owner(&self) -> Result<Option<String>, ForgeError> {
        let url = format!("{}/user", self.base);
        match self.get_url::<RawUser>(&url) {
            Ok(user) => Ok(Some(user.login)),
            Err(ForgeError::Http {
                status: 401 | 403 | 404,
                ..
            }) => Ok(None),
            Err(error) => Err(error),
        }
    }

    fn merge(&self, pr: &Pr, method: MergeMethod) -> Result<Merged, ForgeError> {
        let url = self.repo_url(&format!("/pulls/{}/merge", pr.number));
        let body = format!(r#"{{"merge_method":"{}"}}"#, method.wire());
        let response = self.call("PUT", &url, Some(&body))?;
        if response.status == 403 && response.rate_limit_remaining != Some(0) {
            return Err(ForgeError::Forbidden {
                pr: pr.number,
                target: pr.base.clone(),
                missing_permission: "pull_requests:write",
            });
        }
        let body = Self::ok_body(&url, response)?;
        let merged: RawMerged = serde_json::from_str(&body)
            .map_err(|error| ForgeError::Other(format!("PUT {url}: unexpected body: {error}")))?;
        Ok(Merged { sha: merged.sha })
    }
}

/// What the Gate header shows of the rules and the checks of `pr` under `config`.
fn gate_facts(
    pr: &Pr,
    checks: &[Check],
    reviews: &[Review],
    attestations: &[Attestation],
    policy: MergePolicy,
    config: ConfigMergePolicy,
) -> GateFacts {
    let required_checks = policy
        .required_checks
        .iter()
        .map(|name| RequiredCheck {
            name: name.clone(),
            conclusion: checks
                .iter()
                .find(|check| &check.name == name)
                .map_or(CheckConclusion::RequiredMissing, |check| check.conclusion),
        })
        .collect();
    GateFacts {
        policy: policy.with_override(config, pr.auto_merge),
        required_checks,
        reviews: u64::try_from(reviews.len()).unwrap_or(u64::MAX),
        attestation_ids: attestations.iter().map(|a| a.id.clone()).collect(),
        pr_open: pr.state == PrState::Open,
        target: pr.base.clone(),
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
    /// Pull requests whose hand-over to CD was announced.
    handed: HashSet<u64>,
    /// What `[github].merge_policy` says about who decides.
    merge_policy: ConfigMergePolicy,
    /// The token owner, read once the first merged pull request needs it.
    token_owner: Option<Option<String>>,
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
            handed: HashSet::new(),
            merge_policy: config.merge_policy,
            token_owner: None,
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
                reviews: Vec::new(),
                attestations: Vec::new(),
                gate: None,
                events: Vec::new(),
                windows: Vec::new(),
            });
        };
        let checks = self.forge.checks(&pr)?;
        let merged = matches!(pr.state, PrState::Merged { .. });
        // The merge is the one moment a missing answer would be recorded wrong for good, so
        // there the poll fails and is repeated; before it these are display facts that wait.
        let attestations = match self.forge.attestations(&pr) {
            Ok(found) => found,
            Err(error) if merged => return Err(error),
            Err(error) => {
                tracing::warn!(%error, pr = pr.number, "github attestations unavailable");
                Vec::new()
            }
        };
        let reviews = self.forge.reviews(&pr).unwrap_or_else(|error| {
            tracing::warn!(%error, pr = pr.number, "github reviews unavailable");
            Vec::new()
        });
        let policy = match self.forge.policy(&self.target) {
            Ok(policy) => Some(policy),
            Err(error) => {
                tracing::warn!(%error, target = %self.target, "github merge policy unavailable");
                None
            }
        };

        let mut events = Vec::new();
        if self.announced.insert(pr.number) {
            events.push(self.event(
                EventId(format!("github:pr:{}:opened", pr.number)),
                EventKind::PrOpened,
                Role::External,
                pr.created_at,
                &pr.url,
                None,
            ));
        }
        let mut windows = Vec::new();
        for check in &checks {
            self.check_result(&pr, check, now, &mut events, &mut windows);
        }
        let gate = policy.map(|policy| {
            gate_facts(
                &pr,
                &checks,
                &reviews,
                &attestations,
                policy,
                self.merge_policy,
            )
        });
        if let PrState::Merged {
            at,
            merge_sha,
            merged_by,
        } = &pr.state
        {
            let green = gate.as_ref().map_or_else(
                || {
                    checks
                        .iter()
                        .all(|check| check.conclusion == CheckConclusion::Passed)
                },
                |gate| {
                    gate.required_checks
                        .iter()
                        .all(|check| check.conclusion == CheckConclusion::Passed)
                },
            );
            if green && !self.handed.contains(&pr.number) {
                let owner = self.token_owner()?;
                let role = match (&owner, merged_by) {
                    (Some(owner), Some(merger)) if owner.eq_ignore_ascii_case(merger) => {
                        Role::Human
                    }
                    _ => Role::External,
                };
                self.handed.insert(pr.number);
                events.push(self.event(
                    EventId(format!("github:pr:{}:handed_to_cd", pr.number)),
                    EventKind::HandedToCd {
                        merge_sha: merge_sha.clone().unwrap_or_default(),
                        pr_number: pr.number,
                        attestation_ids: attestations.iter().map(|a| a.id.clone()).collect(),
                    },
                    role,
                    *at,
                    &pr.url,
                    None,
                ));
            }
        }
        Ok(Poll {
            observed_at: now,
            pr: Some(pr),
            checks,
            reviews,
            attestations,
            gate,
            events,
            windows,
        })
    }

    fn token_owner(&mut self) -> Result<Option<String>, ForgeError> {
        if let Some(owner) = &self.token_owner {
            return Ok(owner.clone());
        }
        let owner = self.forge.token_owner()?;
        self.token_owner = Some(owner.clone());
        Ok(owner)
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
            Role::External,
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
        role: Role,
        at: Timestamp,
        url: &str,
        milestone: Option<MilestoneId>,
    ) -> Event {
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

#[cfg(test)]
mod tests {
    use super::merger_role;
    use crate::model::Role;

    #[test]
    fn a_merger_other_than_the_token_owner_is_attributed_as_external() {
        assert_eq!(
            merger_role(Some("dctellya"), Some("some-colleague")),
            Role::External
        );
    }

    #[test]
    fn a_merger_the_forge_cannot_name_is_attributed_as_external() {
        assert_eq!(merger_role(Some("dctellya"), None), Role::External);
        assert_eq!(merger_role(None, Some("dctellya")), Role::External);
    }

    #[test]
    fn the_token_owner_merging_is_attributed_as_human_ignoring_case() {
        assert_eq!(merger_role(Some("dctellya"), Some("DCTellya")), Role::Human);
    }
}
