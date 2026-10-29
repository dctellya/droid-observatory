//! The e2e suite's stand-in for a GitHub poll: a scenario of one pull request's checks goes through
//! the same record, `events_appended` and `gate_changed` path a real poll takes. Only the `e2e`
//! feature builds it; the release app has no such command.

use observatory_core::ingest::forge::{
    CheckConclusion, GateFacts, MergeMethod, MergePolicy, Poll, Pr, PrState, RequiredCheck,
    RuleAttribution,
};
use observatory_core::mapping::MappingVersion;
use observatory_core::model::{Event, EventId, EventKind, MilestoneId, Role, SourcePtr, Timestamp};
use observatory_core::view::MilestoneWorktree;
use tauri::{AppHandle, Manager};

use crate::Shell;
use crate::commands::resolve_branch;

const MILESTONE: &str = "e2e-gate";
const WORKTREE_NAME: &str = "wt-e2e-gate";
const WORKTREE_PATH: &str = "/e2e/worktrees/wt-e2e-gate";
const PR_NUMBER: u64 = 24;
const RULESET_ID: u64 = 26_112_925;
const RULESET_NAME: &str = "demo-gate";
const REQUIRED_CHECK: &str = "demo-gate";
const MERGE_SHA: &str = "a3ec578735fbcbbd15db8dfc7612fd9f16901a63";
const TARGET: &str = "demo/base";
const HEAD: &str = "demo/e2e-gate";
const PR_URL: &str = "https://api.github.com/repos/dctellya/droid-observatory/pulls/24";

/// What a scenario adds: its new events, the conclusion the required check has and whether the PR
/// is still open.
struct Scenario {
    kinds: Vec<(EventKind, Role)>,
    conclusion: CheckConclusion,
    pr_open: bool,
}

fn scenario(name: &str) -> Option<Scenario> {
    let external = |kind| (kind, Role::External);
    Some(match name {
        "red" => Scenario {
            kinds: vec![
                external(EventKind::PrOpened),
                external(EventKind::CheckStarted),
                external(EventKind::CheckFailed),
            ],
            conclusion: CheckConclusion::Failed,
            pr_open: true,
        },
        "green" => Scenario {
            kinds: vec![external(EventKind::CheckPassed)],
            conclusion: CheckConclusion::Passed,
            pr_open: true,
        },
        "merged" => Scenario {
            kinds: vec![(
                EventKind::HandedToCd {
                    merge_sha: MERGE_SHA.to_owned(),
                    pr_number: PR_NUMBER,
                    attestation_ids: Vec::new(),
                },
                Role::Human,
            )],
            conclusion: CheckConclusion::Passed,
            pr_open: false,
        },
        _ => return None,
    })
}

/// What a poll reads while the mission branch has no pull request yet.
fn pending_poll(now: Timestamp) -> Poll {
    Poll {
        observed_at: now,
        pr: None,
        checks: Vec::new(),
        reviews: Vec::new(),
        attestations: Vec::new(),
        gate: None,
        events: Vec::new(),
        windows: Vec::new(),
    }
}

/// Runs the named scenario (`pending`, `red`, `green`, `merged`) on the first live mission.
#[tauri::command(rename_all = "snake_case")]
pub async fn e2e_gate_poll(app: AppHandle, scenario_name: String) -> Result<String, String> {
    let mission = crate::stall::live_missions(&app)
        .into_iter()
        .next()
        .ok_or("no live mission")?;
    let shell = app.state::<Shell>();
    let branch = {
        let store = lock_store(&shell);
        let name = store
            .missions()
            .map_err(|error| error.to_string())?
            .into_iter()
            .find(|row| row.id == mission)
            .map(|row| row.name)
            .ok_or("the mission has no row")?;
        resolve_branch(&store, &shell.cfg.mission, &mission, &name)
    };
    if scenario_name == "pending" {
        let mut shown = None;
        crate::github::apply_poll(
            &app,
            &mut lock_store(&shell),
            &mission,
            &branch,
            pending_poll(Timestamp::now()),
            &mut shown,
        );
        return Ok(mission.to_string());
    }
    let scenario =
        scenario(&scenario_name).ok_or_else(|| format!("no scenario {scenario_name}"))?;
    let template = shell.template.clone();
    let now = Timestamp::now();
    let events = scenario
        .kinds
        .into_iter()
        .enumerate()
        .map(|(index, (kind, role))| Event {
            id: EventId(format!("e2e-gate-{mission}-{scenario_name}-{index}")),
            mission_id: mission.clone(),
            milestone_id: Some(MilestoneId(MILESTONE.to_owned())),
            feature_id: None,
            lane: template.lane_of(&kind, role),
            overlay: None,
            kind,
            role,
            session_id: None,
            model: None,
            at: now,
            duration: None,
            usage: None,
            source: SourcePtr::GitHub {
                url: PR_URL.to_owned(),
            },
            mapping_version: MappingVersion::LATEST,
        })
        .collect();
    let poll = Poll {
        observed_at: now,
        pr: Some(Pr {
            number: PR_NUMBER,
            head_sha: MERGE_SHA.to_owned(),
            head: HEAD.to_owned(),
            base: TARGET.to_owned(),
            url: PR_URL.to_owned(),
            created_at: now,
            state: if scenario.pr_open {
                PrState::Open
            } else {
                PrState::Closed
            },
            auto_merge: false,
        }),
        checks: Vec::new(),
        reviews: Vec::new(),
        attestations: Vec::new(),
        gate: Some(GateFacts {
            policy: MergePolicy {
                requires_human: true,
                allowed_methods: vec![MergeMethod::Squash],
                decided_by: RuleAttribution::Ruleset {
                    id: RULESET_ID,
                    name: RULESET_NAME.to_owned(),
                },
                required_checks: vec![REQUIRED_CHECK.to_owned()],
            },
            required_checks: vec![RequiredCheck {
                name: REQUIRED_CHECK.to_owned(),
                conclusion: scenario.conclusion,
            }],
            reviews: 0,
            attestation_ids: Vec::new(),
            pr_open: scenario.pr_open,
            target: TARGET.to_owned(),
        }),
        events,
        windows: Vec::new(),
    };
    // The worktree list a git poll would have cached for the mission, so the card names it.
    app.state::<crate::git::GitCache>().put(
        &mission,
        vec![MilestoneWorktree {
            milestone: MilestoneId(MILESTONE.to_owned()),
            name: WORKTREE_NAME.to_owned(),
            path: WORKTREE_PATH.to_owned(),
        }],
    );
    let mut store = lock_store(&shell);
    let mut shown = None;
    crate::github::apply_poll(&app, &mut store, &mission, &branch, poll, &mut shown);
    Ok(mission.to_string())
}

fn lock_store<'a>(shell: &'a Shell) -> std::sync::MutexGuard<'a, observatory_core::store::Store> {
    shell
        .store
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
