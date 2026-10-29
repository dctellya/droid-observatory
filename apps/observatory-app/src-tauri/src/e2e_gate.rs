//! The e2e suite's stand-in for a GitHub poll: a scenario of one pull request's checks goes through
//! the same record, `events_appended` and `gate_changed` path a real poll takes. Only the `e2e`
//! feature builds it; the release app has no such command.

use observatory_core::ingest::forge::{
    CheckConclusion, GateFacts, MergeMethod, MergePolicy, Poll, RequiredCheck, RuleAttribution,
};
use observatory_core::mapping::MappingVersion;
use observatory_core::model::{Event, EventId, EventKind, MilestoneId, Role, SourcePtr, Timestamp};
use tauri::{AppHandle, Manager};

use crate::Shell;

const MILESTONE: &str = "e2e-gate";
const PR_NUMBER: u64 = 24;
const RULESET_ID: u64 = 26_112_925;
const RULESET_NAME: &str = "demo-gate";
const REQUIRED_CHECK: &str = "demo-gate";
const MERGE_SHA: &str = "a3ec578735fbcbbd15db8dfc7612fd9f16901a63";
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

/// Runs the named scenario (`red`, `green`, `merged`) on the first live mission.
#[tauri::command(rename_all = "snake_case")]
pub async fn e2e_gate_poll(app: AppHandle, scenario_name: String) -> Result<String, String> {
    let scenario =
        scenario(&scenario_name).ok_or_else(|| format!("no scenario {scenario_name}"))?;
    let mission = crate::stall::live_missions(&app)
        .into_iter()
        .next()
        .ok_or("no live mission")?;
    let shell = app.state::<Shell>();
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
        pr: None,
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
        }),
        events,
        windows: Vec::new(),
    };
    let mut store = shell
        .store
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let mut shown = None;
    crate::github::apply_poll(&app, &mut store, &mission, poll, &mut shown);
    Ok(mission.to_string())
}
