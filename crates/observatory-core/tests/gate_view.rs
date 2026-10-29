//! What the Gate lane and the card draw (FR-B3, FR-B4, FR-G5, FR-G6): the group of required checks,
//! reviews, ruleset and attestations, the return arc of a fix cycle, the merge offer and the
//! `handed to CD` reading with the two clocks. The lanes come from the default template and are
//! addressed through it.

use observatory_core::ingest::forge::{
    CheckConclusion, GateFacts, MergeMethod, MergePolicy, RequiredCheck, RuleAttribution,
};
use observatory_core::mapping::MappingVersion;
use observatory_core::model::{
    CycleTemplate, Event, EventId, EventKind, MilestoneId, MissionId, Role, SourcePtr, Timestamp,
};
use observatory_core::view::{BoardState, GateBadge, GateGroup, ViewConfig};

const MILESTONE: &str = "m4";
const MERGE_SHA: &str = "a3ec578735fbcbbd15db8dfc7612fd9f16901a63";

fn event(kind: EventKind, role: Role, second: i64) -> Event {
    let template = CycleTemplate::engineering();
    Event {
        id: EventId(format!("e{second}")),
        mission_id: MissionId("mission".into()),
        milestone_id: Some(MilestoneId(MILESTONE.into())),
        feature_id: None,
        lane: template.lane_of(&kind, role),
        overlay: None,
        kind,
        role,
        session_id: None,
        model: None,
        at: Timestamp(
            Timestamp::parse("2026-10-29T10:00:00Z").unwrap().0 + time::Duration::seconds(second),
        ),
        duration: None,
        usage: None,
        source: SourcePtr::GitHub {
            url: "https://api.github.com/repos/dctellya/droid-observatory/pulls/24".into(),
        },
        mapping_version: MappingVersion::LATEST,
    }
}

fn facts(conclusions: &[(&str, CheckConclusion)], attestations: &[&str]) -> GateFacts {
    GateFacts {
        policy: MergePolicy {
            requires_human: true,
            allowed_methods: vec![MergeMethod::Squash],
            decided_by: RuleAttribution::Ruleset {
                id: 26_112_925,
                name: "demo-gate".into(),
            },
            required_checks: vec!["demo-gate".into()],
        },
        required_checks: conclusions
            .iter()
            .map(|(name, conclusion)| RequiredCheck {
                name: (*name).into(),
                conclusion: *conclusion,
            })
            .collect(),
        reviews: 0,
        attestation_ids: attestations.iter().map(|id| (*id).to_owned()).collect(),
        pr_open: true,
    }
}

fn opened_green() -> Vec<Event> {
    vec![
        event(EventKind::PrOpened, Role::External, 0),
        event(EventKind::CheckPassed, Role::External, 1),
    ]
}

#[test]
fn the_gate_group_lists_checks_reviews_ruleset_and_attestations() {
    let template = CycleTemplate::engineering();
    let group = GateGroup::of(&opened_green())
        .with_facts(&facts(&[("demo-gate", CheckConclusion::Passed)], &[]));
    let view = group.view(&template);

    assert_eq!(view.badge, GateBadge::CiGreen);
    assert_eq!(view.badge_text, "CI green");
    assert_eq!(view.pr_number, Some(24));
    let checks: Vec<_> = view
        .checks
        .iter()
        .map(|row| (row.name.as_str(), row.conclusion.as_str()))
        .collect();
    assert_eq!(checks, [("demo-gate", "passed")]);
    assert_eq!(view.reviews, 0, "a count of 0 is shown, not omitted");
    assert_eq!(view.ruleset.as_deref(), Some("demo-gate"));
    assert!(
        view.attestations.is_empty(),
        "no attestation yet: the group still has the entry"
    );
    assert_eq!(view.attestations_text, "attestations: 0");
    assert_eq!(view.reviews_text, "reviews: 0");
    assert_eq!(view.ruleset_text, "ruleset: demo-gate");
}

#[test]
fn the_ruleset_line_says_who_decides_when_no_ruleset_does() {
    let template = CycleTemplate::engineering();
    let mut by_config = facts(&[("demo-gate", CheckConclusion::Passed)], &[]);
    by_config.policy.decided_by = RuleAttribution::ConfigOverride("human".into());
    let view = GateGroup::of(&opened_green())
        .with_facts(&by_config)
        .view(&template);
    assert_eq!(view.ruleset, None);
    assert_eq!(
        view.ruleset_text,
        "decided by: [github].merge_policy = human"
    );
}

#[test]
fn the_merge_offer_follows_the_policy_and_the_checks_through_the_view() {
    let template = CycleTemplate::engineering();
    let red = [
        event(EventKind::PrOpened, Role::External, 0),
        event(EventKind::CheckFailed, Role::External, 1),
    ];
    let shown = |events: &[Event], facts: &GateFacts| {
        GateGroup::of(events)
            .with_facts(facts)
            .view(&template)
            .merge
    };

    let offered = shown(
        &opened_green(),
        &facts(&[("demo-gate", CheckConclusion::Passed)], &[]),
    )
    .expect("human decides and the check is green");
    assert_eq!(offered.pr_number, 24);
    assert_eq!(offered.label, "Merge");

    assert!(
        shown(&red, &facts(&[("demo-gate", CheckConclusion::Failed)], &[])).is_none(),
        "no Merge while red"
    );
    let mut policy_merges = facts(&[("demo-gate", CheckConclusion::Passed)], &[]);
    policy_merges.policy.requires_human = false;
    assert!(shown(&opened_green(), &policy_merges).is_none());
}

#[test]
fn a_scrubbed_board_never_offers_merge_from_a_later_poll() {
    let template = CycleTemplate::engineering();
    let polled = GateGroup::of(&opened_green())
        .with_facts(&facts(&[("demo-gate", CheckConclusion::Passed)], &[]));
    assert!(polled.merge_offered);
    let earlier = GateGroup::of(&opened_green()[..1]).with_polled(&polled);
    assert_eq!(earlier.badge, GateBadge::ChecksRunning);
    assert!(!earlier.merge_offered, "the scrubbed events were not green");
    assert!(earlier.view(&template).merge.is_none());
    let now = GateGroup::of(&opened_green()).with_polled(&polled);
    assert!(now.merge_offered);
    assert_eq!(now.decided_by, polled.decided_by);
}

#[test]
fn a_merged_pr_reads_handed_to_cd_with_sha_pr_attestations_and_merger() {
    let template = CycleTemplate::engineering();
    let handed = |role| {
        let mut events = opened_green();
        events.push(event(
            EventKind::HandedToCd {
                merge_sha: MERGE_SHA.into(),
                pr_number: 24,
                attestation_ids: vec!["att-1".into(), "att-2".into()],
            },
            role,
            2,
        ));
        GateGroup::of(&events).view(&template)
    };

    let by_human = handed(Role::Human);
    assert_eq!(by_human.badge, GateBadge::HumanMerged);
    assert_eq!(by_human.badge_text, "handed to CD");
    assert_eq!(
        by_human.handed_to_cd.as_deref(),
        Some("handed to CD: a3ec578 · PR #24 · attestations att-1, att-2 · merged by human")
    );
    assert!(by_human.merge.is_none(), "nothing left to press");
    assert_eq!(by_human.attestations, ["att-1", "att-2"]);

    let by_policy = handed(Role::External);
    assert_eq!(by_policy.badge, GateBadge::PolicyMerged);
    assert!(
        by_policy
            .handed_to_cd
            .as_deref()
            .is_some_and(|text| text.ends_with("merged by policy"))
    );
}

#[test]
fn the_gate_header_names_both_clocks_and_computes_only_the_first() {
    let template = CycleTemplate::engineering();
    let pending = GateGroup::of(&[]).view(&template);
    let notes: Vec<_> = pending
        .clocks
        .iter()
        .map(|clock| (clock.label.as_str(), clock.note.as_str()))
        .collect();
    assert_eq!(
        notes,
        [
            ("engineering cycle time", "closed at handed_to_cd"),
            (
                "lead time to production: delivery platform's clock",
                "starts at the merge sha; not computed here"
            ),
        ]
    );

    let mut events = opened_green();
    events.push(event(
        EventKind::HandedToCd {
            merge_sha: MERGE_SHA.into(),
            pr_number: 24,
            attestation_ids: Vec::new(),
        },
        Role::Human,
        2,
    ));
    let merged = GateGroup::of(&events).view(&template);
    assert_eq!(
        merged.clocks[1].note, "starts at a3ec578; not computed here",
        "the platform's clock starts at the merge sha"
    );
}

#[test]
fn red_required_check_draws_a_return_arc_from_the_final_lane_to_the_build_lane() {
    let template = CycleTemplate::engineering();
    let cfg = ViewConfig::default();
    let mut events = vec![event(EventKind::PrOpened, Role::External, 0)];

    let at_gate = BoardState::fold(&events, &cfg, &template);
    assert_eq!(at_gate.cards[0].return_arc(&template), None, "no cycle yet");

    events.push(event(EventKind::CheckFailed, Role::External, 1));
    let red = BoardState::fold(&events, &cfg, &template);
    let card = &red.cards[0];
    let arc = card
        .return_arc(&template)
        .expect("a red check draws the arc");
    assert_eq!(&arc.from, template.final_lane());
    assert_eq!(&arc.to, template.home(Role::Worker));
    assert_eq!(card.lane, arc.to, "the card is back in the build lane");
    assert_eq!(arc.cycle, 1);
    assert_eq!(arc.label, "cycle 1");
    assert_eq!(arc.from_name, template.name_of(template.final_lane()));
    assert_eq!(arc.to_name, template.name_of(template.home(Role::Worker)));
    assert!(arc.active, "the card is still being fixed");

    events.push(event(EventKind::CheckPassed, Role::External, 2));
    let green = BoardState::fold(&events, &cfg, &template);
    let arc = green.cards[0]
        .return_arc(&template)
        .expect("the count stays on the card");
    assert_eq!((arc.cycle, arc.active), (1, false));
    assert_eq!(&green.cards[0].lane, template.final_lane());
}
