//! `GitSource` (architecture §4.6, FR-G1/FR-G2): what the repository says about a mission.
//! It reads the mission branch, the `wt/*` milestone branches and their worktrees, turns each
//! first-parent merge on the mission branch into one `integration.merged` event, and turns the
//! fast-check runs a forge reports into `gate_command` events attributed to the milestone whose
//! merge is the run's head or its nearest first-parent ancestor merge.
//!
//! Every event is built with a kind the model knows; nothing here yields `Unknown`. Lanes come
//! from the cycle template (`lane_of`), never from a name (F-041, D-035). The source runs `git`
//! read-only and keeps no raw text: a merge event points at its commit (`SourcePtr::Git`).

mod cmd;
#[cfg(feature = "store")]
mod sync;

#[cfg(feature = "store")]
pub use sync::{GitSync, GitSyncError, git_sync};

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::config::MissionConfig;
use crate::ingest::IngestError;
use crate::mapping::MappingVersion;
use crate::model::{
    CycleTemplate, Event, EventId, EventKind, MilestoneId, MissionId, Role, SourcePtr, Timestamp,
};

use cmd::{RawMerge, git, git_if_found};

/// The placeholder of `[mission].branch_pattern` that stands for the mission name.
const NAME_PLACEHOLDER: &str = "<name>";
/// A full object name in hex: the only text of a run's head that reaches git.
const OBJECT_NAME_LEN: usize = 40;

/// Where the mission's refs live: the mission branches by the prefix of `[mission].branch_pattern`,
/// `wt/<milestone>` per milestone (architecture §16).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitOptions {
    pub mission_prefix: String,
    pub worktree_prefix: String,
}

impl GitOptions {
    /// The options of a `[mission].branch_pattern` such as `mission/<name>`: the mission branches
    /// are the branches that start with the text before the name.
    pub fn from_branch_pattern(pattern: &str) -> Self {
        let prefix = pattern
            .split_once(NAME_PLACEHOLDER)
            .map_or(pattern, |(before, _)| before);
        Self {
            mission_prefix: prefix.to_owned(),
            worktree_prefix: "wt/".into(),
        }
    }
}

impl Default for GitOptions {
    fn default() -> Self {
        Self::from_branch_pattern(&MissionConfig::default().branch_pattern)
    }
}

/// A `wt/<milestone>` branch and where it points.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MilestoneBranch {
    pub milestone: MilestoneId,
    pub branch: String,
    pub tip: String,
}

/// One entry of `git worktree list`. `milestone` is set for a `wt/<m>` branch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Worktree {
    pub path: PathBuf,
    pub branch: Option<String>,
    pub head: String,
    pub milestone: Option<MilestoneId>,
}

/// The repository as it is now, before any merge exists: the placeholder of the final lane
/// names `mission_branch`, and every worktree git lists shows on the board.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitSnapshot {
    pub mission_branch: String,
    pub mission_head: String,
    pub worktrees: Vec<Worktree>,
    pub milestone_branches: Vec<MilestoneBranch>,
}

/// A first-parent merge commit on the mission branch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Merge {
    pub sha: String,
    pub committed_at: Timestamp,
    pub subject: String,
    /// The milestone whose `wt/<m>` branch made the merge; none when no such branch did (a
    /// pull request merged on the target branch before the mission).
    pub milestone: Option<MilestoneId>,
}

/// How a fast-check run ended, when it has ended with a verdict.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunConclusion {
    Passed,
    Failed,
}

impl RunConclusion {
    /// The verdict of a GitHub Actions `conclusion`. A run that was cancelled, skipped or is
    /// still going has no verdict, so it yields no passed or failed event.
    pub fn from_github(conclusion: &str) -> Option<Self> {
        match conclusion {
            "success" => Some(Self::Passed),
            "failure" | "timed_out" | "startup_failure" => Some(Self::Failed),
            _ => None,
        }
    }
}

/// One run of the fast check (`make check`, the `mission-checks` workflow) as a forge reports it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FastCheckRun {
    pub run_id: u64,
    /// The API URL of the run; it names the run id.
    pub url: String,
    pub head_sha: String,
    pub started_at: Timestamp,
    pub finished_at: Option<Timestamp>,
    pub conclusion: Option<RunConclusion>,
}

pub struct GitSource {
    repo: PathBuf,
    mission: MissionId,
    branch: String,
    template: CycleTemplate,
    options: GitOptions,
}

impl GitSource {
    /// The mission branches of the repository, as `git for-each-ref refs/heads/<prefix>*` lists
    /// them, the prefix being the one `options` took from `[mission].branch_pattern`.
    pub fn mission_branches(repo: &Path, options: &GitOptions) -> Result<Vec<String>, IngestError> {
        branches_under(repo, &options.mission_prefix)
    }

    /// A source for `branch` of `repo`. A branch the repository does not have is an error: an
    /// empty source would read as "no merges yet".
    pub fn open(
        repo: &Path,
        mission: MissionId,
        branch: &str,
        template: CycleTemplate,
        options: GitOptions,
    ) -> Result<Self, IngestError> {
        let source = Self {
            repo: repo.canonicalize().unwrap_or_else(|_| repo.to_owned()),
            mission,
            branch: branch.to_owned(),
            template,
            options,
        };
        source.head()?;
        Ok(source)
    }

    pub fn branch(&self) -> &str {
        &self.branch
    }

    fn head(&self) -> Result<String, IngestError> {
        let reference = format!("refs/heads/{}^{{commit}}", self.branch);
        git_if_found(
            &self.repo,
            &["rev-parse", "--verify", "--quiet", &reference],
        )?
        .ok_or_else(|| IngestError::Git {
            repo: self.repo.clone(),
            message: format!("the branch {} does not exist", self.branch),
        })
    }

    fn milestone_of_branch(&self, branch: &str) -> Option<MilestoneId> {
        branch
            .strip_prefix(&self.options.worktree_prefix)
            .filter(|name| !name.is_empty())
            .map(|name| MilestoneId(name.to_owned()))
    }

    /// The mission branch, its head, every worktree and every milestone branch.
    pub fn snapshot(&self) -> Result<GitSnapshot, IngestError> {
        let worktrees =
            cmd::parse_worktrees(&git(&self.repo, &["worktree", "list", "--porcelain"])?)
                .into_iter()
                .map(|raw| Worktree {
                    milestone: raw
                        .branch
                        .as_deref()
                        .and_then(|branch| self.milestone_of_branch(branch)),
                    path: raw.path,
                    branch: raw.branch,
                    head: raw.head,
                })
                .collect();
        Ok(GitSnapshot {
            mission_branch: self.branch.clone(),
            mission_head: self.head()?,
            worktrees,
            milestone_branches: self.milestone_branches()?,
        })
    }

    fn milestone_branches(&self) -> Result<Vec<MilestoneBranch>, IngestError> {
        let pattern = format!("refs/heads/{}", self.options.worktree_prefix);
        let listed = git(
            &self.repo,
            &[
                "for-each-ref",
                "--format=%(refname)\t%(objectname)",
                &pattern,
            ],
        )?;
        Ok(listed
            .lines()
            .filter_map(|line| {
                let (name, tip) = line.split_once('\t')?;
                let branch = name.strip_prefix("refs/heads/")?;
                Some(MilestoneBranch {
                    milestone: self.milestone_of_branch(branch)?,
                    branch: branch.to_owned(),
                    tip: tip.to_owned(),
                })
            })
            .collect())
    }

    /// The first-parent merge commits of the mission branch, oldest first, each attributed to
    /// the milestone whose `wt/<m>` branch made it: the branch the subject names, else the
    /// one whose first-parent line holds the merged tip (nearest to its own tip wins, so a
    /// later milestone branched from the merge does not claim it).
    pub fn merges(&self) -> Result<Vec<Merge>, IngestError> {
        let reference = format!("refs/heads/{}", self.branch);
        let log = git(
            &self.repo,
            &[
                "log",
                "--first-parent",
                "--merges",
                cmd::MERGE_FORMAT,
                &reference,
                "--",
            ],
        )?;
        let raw = cmd::parse_merges(&log);
        let branches = self.milestone_branches()?;
        let mut lines = BranchLines::default();
        let mut merges = Vec::with_capacity(raw.len());
        for merge in raw.into_iter().rev() {
            let milestone = self.attribute(&merge, &branches, &mut lines)?;
            let committed_at =
                Timestamp::parse(&merge.committed_at).map_err(|error| IngestError::Git {
                    repo: self.repo.clone(),
                    message: format!(
                        "commit {} has date {:?}: {error}",
                        merge.sha, merge.committed_at
                    ),
                })?;
            merges.push(Merge {
                sha: merge.sha,
                committed_at,
                subject: merge.subject,
                milestone,
            });
        }
        Ok(merges)
    }

    fn attribute(
        &self,
        merge: &RawMerge,
        branches: &[MilestoneBranch],
        lines: &mut BranchLines,
    ) -> Result<Option<MilestoneId>, IngestError> {
        if let Some(name) = cmd::named_in_subject(&merge.subject, &self.options.worktree_prefix) {
            return Ok(Some(MilestoneId(name.to_owned())));
        }
        let mut best: Option<(usize, MilestoneId)> = None;
        for branch in branches {
            let line = lines.of(&self.repo, &branch.branch)?;
            let nearest = merge
                .parents
                .iter()
                .skip(1)
                .filter_map(|parent| line.get(parent.as_str()))
                .min();
            if let Some(&distance) = nearest
                && best.as_ref().is_none_or(|(seen, _)| distance < *seen)
            {
                best = Some((distance, branch.milestone.clone()));
            }
        }
        Ok(best.map(|(_, milestone)| milestone))
    }

    fn event(
        &self,
        id: String,
        kind: EventKind,
        role: Role,
        at: Timestamp,
        milestone: Option<&MilestoneId>,
        source: SourcePtr,
    ) -> Event {
        Event {
            id: EventId(id),
            mission_id: self.mission.clone(),
            milestone_id: milestone.cloned(),
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
            source,
            mapping_version: MappingVersion::LATEST,
        }
    }

    /// One `integration.merged` per first-parent merge of the mission branch, oldest first.
    pub fn integration_events(&self) -> Result<Vec<Event>, IngestError> {
        Ok(self.integration_events_of(self.merges()?))
    }

    /// The events of `merges`, which a poll reads once and shares with `gate_command_events_of`.
    fn integration_events_of(&self, merges: Vec<Merge>) -> Vec<Event> {
        merges
            .into_iter()
            .map(|merge| {
                self.event(
                    format!("git:{}:0", merge.sha),
                    EventKind::IntegrationMerged,
                    Role::Orchestrator,
                    merge.committed_at,
                    merge.milestone.as_ref(),
                    SourcePtr::Git {
                        repo: self.repo.clone(),
                        sha: merge.sha,
                    },
                )
            })
            .collect()
    }

    /// `gate_command.run` for every run and `gate_command.passed` or `.failed` for each that
    /// has a verdict, attributed to the milestone whose `wt/<m>` merge is the run's head or the
    /// nearest first-parent ancestor of it: the milestone integrated last. A commit after a
    /// merge (docs, a fix) belongs to the section the merge opened. A run whose head has no such
    /// merge behind it has no milestone: guessing one would put the run in the wrong section of
    /// the Integrate lane.
    pub fn gate_command_events(&self, runs: &[FastCheckRun]) -> Result<Vec<Event>, IngestError> {
        self.gate_command_events_of(&self.merges()?, runs)
    }

    fn gate_command_events_of(
        &self,
        merges: &[Merge],
        runs: &[FastCheckRun],
    ) -> Result<Vec<Event>, IngestError> {
        let by_sha: HashMap<&str, &MilestoneId> = merges
            .iter()
            .filter_map(|merge| Some((merge.sha.as_str(), merge.milestone.as_ref()?)))
            .collect();
        let mut heads: HashMap<&str, Option<MilestoneId>> = HashMap::new();
        let mut events = Vec::with_capacity(runs.len() * 2);
        for run in runs {
            let milestone = match heads.get(run.head_sha.as_str()) {
                Some(known) => known.clone(),
                None => {
                    let found = self.milestone_integrated_last(&run.head_sha, &by_sha)?;
                    heads.insert(&run.head_sha, found.clone());
                    found
                }
            };
            let pointer = || SourcePtr::GitHub {
                url: run.url.clone(),
            };
            events.push(self.event(
                format!("github:{}:0", run.run_id),
                EventKind::GateCommandRun,
                Role::External,
                run.started_at,
                milestone.as_ref(),
                pointer(),
            ));
            if let Some(conclusion) = run.conclusion {
                let kind = match conclusion {
                    RunConclusion::Passed => EventKind::GateCommandPassed,
                    RunConclusion::Failed => EventKind::GateCommandFailed,
                };
                let at = run.finished_at.unwrap_or(run.started_at);
                let mut verdict = self.event(
                    format!("github:{}:1", run.run_id),
                    kind,
                    Role::External,
                    at,
                    milestone.as_ref(),
                    pointer(),
                );
                verdict.duration = Duration::try_from(at.0 - run.started_at.0).ok();
                events.push(verdict);
            }
        }
        Ok(events)
    }
}

impl GitSource {
    /// One first-parent walk of merges from `head`; the first one a milestone made is the
    /// answer. A head this clone does not hold has no milestone.
    fn milestone_integrated_last(
        &self,
        head: &str,
        by_sha: &HashMap<&str, &MilestoneId>,
    ) -> Result<Option<MilestoneId>, IngestError> {
        // A run's head comes from a forge; anything but a full object name never reaches git.
        if head.len() < OBJECT_NAME_LEN || !head.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Ok(None);
        }
        let commit = format!("{head}^{{commit}}");
        let walked = cmd::git_or_none(
            &self.repo,
            &["rev-list", "--first-parent", "--merges", &commit, "--"],
        )?;
        Ok(walked.and_then(|walked| {
            walked
                .lines()
                .find_map(|sha| by_sha.get(sha))
                .copied()
                .cloned()
        }))
    }
}

/// The top of the working tree `dir` is in, as `git rev-parse --show-toplevel` prints it.
pub fn toplevel(dir: &Path) -> Result<PathBuf, IngestError> {
    Ok(PathBuf::from(
        git(dir, &["rev-parse", "--show-toplevel"])?.trim(),
    ))
}

fn branches_under(repo: &Path, prefix: &str) -> Result<Vec<String>, IngestError> {
    let pattern = format!("refs/heads/{prefix}");
    let listed = git(repo, &["for-each-ref", "--format=%(refname)", &pattern])?;
    Ok(listed
        .lines()
        .filter_map(|name| name.strip_prefix("refs/heads/"))
        .map(str::to_owned)
        .collect())
}

/// First-parent lines of the milestone branches, read once per branch: sha to its distance
/// from the branch tip.
#[derive(Default)]
struct BranchLines(HashMap<String, HashMap<String, usize>>);

impl BranchLines {
    fn of(&mut self, repo: &Path, branch: &str) -> Result<&HashMap<String, usize>, IngestError> {
        if !self.0.contains_key(branch) {
            let reference = format!("refs/heads/{branch}");
            let listed = git(repo, &["rev-list", "--first-parent", &reference, "--"])?;
            let line = listed
                .lines()
                .enumerate()
                .map(|(distance, sha)| (sha.to_owned(), distance))
                .collect();
            self.0.insert(branch.to_owned(), line);
        }
        Ok(&self.0[branch])
    }
}
