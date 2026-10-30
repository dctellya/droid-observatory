//! Tokens and cost from `turn.usage` and the sidecar, budget meter, model comparison (architecture §9).

mod metrics_support;

use metrics_support::{
    EventExt, UsageExt, cache_write, ev, m0_events, prices, t, turn, usage_report,
};
use observatory_core::config::OverheadApportionment;
use observatory_core::metrics::{
    CostInput, CostReport, CostUnit, Notes, PriceTable, RowScope, WasteCause, budget_meter,
    cost_report, metrics_report,
};
use observatory_core::model::{
    CycleTemplate, Event, EventKind, FeatureId, ModelId, Role, SessionId, Timestamp, TokenUsage,
};

const FABLE: &str = "claude-fable-5.1";
const SONNET: &str = "claude-sonnet-5-5";
const OPUS: &str = "claude-opus-5-5";
const DATE: &str = "2026-10-22";

fn report_with(events: &[Event], prices: &PriceTable, end: Timestamp) -> CostReport {
    cost_report(&CostInput {
        events,
        template: &CycleTemplate::engineering(),
        notes: &Notes::default(),
        prices,
        end,
        budget: 0.0,
        launch_budget_usd: None,
        apportionment: OverheadApportionment::Separate,
    })
}

fn report(events: &[Event], end: Timestamp) -> CostReport {
    report_with(events, &prices(), end)
}

fn usd(tokens: [u64; 3], rates: [f64; 3]) -> f64 {
    tokens
        .iter()
        .zip(rates)
        .map(|(count, per_m)| *count as f64 * per_m / 1e6)
        .sum()
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

fn m0_end(events: &[Event]) -> Timestamp {
    events
        .iter()
        .filter(|e| !e.kind.is_panel_only())
        .map(|e| e.at)
        .max()
        .expect("events")
}

fn counts(event: &Event) -> [u64; 3] {
    match event.kind {
        EventKind::TurnUsage {
            input,
            output,
            cache_read,
            ..
        } => [input, output, cache_read],
        _ => [0; 3],
    }
}

#[test]
fn cost_names_the_price_table_date() {
    let table = prices();
    assert_eq!(table.effective(), DATE);
    let events = m0_events();
    let report = report_with(&events, &table, m0_end(&events));

    assert_eq!(report.price_table, DATE);
    let costs = report.costs();
    assert!(
        costs.len() > 20,
        "roles, lanes, sessions, rows: {}",
        costs.len()
    );
    for cost in costs {
        assert_eq!(cost.price_table, table.effective());
    }
}

#[test]
fn cost_uses_credits_when_the_stream_exposes_them_else_the_price_table() {
    let mut credited = ev("u1", EventKind::UsageReported, 10)
        .model(SONNET)
        .reporting(TokenUsage {
            input: 1_000_000,
            output: 100_000,
            cache_read: 2_000_000,
            cache_write: 400_000,
            credits: Some(42772.0),
        });
    credited.session_id = Some(SessionId("with-credits".into()));
    let mut priced = ev("u2", EventKind::UsageReported, 20)
        .model(SONNET)
        .reporting(TokenUsage {
            input: 1_000_000,
            output: 100_000,
            cache_read: 2_000_000,
            cache_write: 400_000,
            credits: None,
        });
    priced.session_id = Some(SessionId("no-credits".into()));

    let report = report(&[credited, priced], t(30));

    let by_session = |id: &str| {
        report
            .session_costs
            .iter()
            .find(|row| row.session.0 == id)
            .expect("a row per session")
    };
    let with = &by_session("with-credits").cost;
    assert_eq!(with.unit, CostUnit::Credits);
    assert!(close(with.amount, 42772.0));
    assert_eq!(with.price_table, DATE);

    let without = &by_session("no-credits").cost;
    assert_eq!(without.unit, CostUnit::Usd);
    // 1 M input at 2.0, 0.1 M output at 10.0, 2 M cache read at 0.10, 0.4 M cache write at 2.5.
    assert!(
        close(without.amount, 2.0 + 1.0 + 0.2 + 1.0),
        "{}",
        without.amount
    );
}

#[test]
fn tokens_per_role_and_lane_sum_turn_usage_events() {
    let events = vec![
        turn("a", 10, Role::Worker, SONNET, "build", [100, 10, 1000]),
        turn("b", 20, Role::Worker, SONNET, "build", [50, 5, 500]),
        turn("c", 30, Role::Orchestrator, FABLE, "plan", [10, 20, 30]),
        turn(
            "d",
            40,
            Role::ScrutinyValidator,
            OPUS,
            "scrutiny",
            [1, 2, 3],
        ),
    ];
    let text = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../config/prices/2026-10-22.toml"),
    )
    .expect("the price table exists");
    assert!(
        text.contains("worker = 1.0"),
        "the table names the multiplier key"
    );
    let with_multiplier =
        PriceTable::parse(&text.replace("worker = 1.0", "worker = 2.0")).expect("parses");

    let report = report_with(&events, &with_multiplier, t(50));

    let role = |role: Role| {
        report
            .tokens_by_role
            .iter()
            .find(|row| row.role == role)
            .map(|row| (row.tokens.input, row.tokens.output, row.tokens.cache_read))
    };
    assert_eq!(role(Role::Worker), Some((150, 15, 1500)));
    assert_eq!(role(Role::Orchestrator), Some((10, 20, 30)));
    assert_eq!(role(Role::ScrutinyValidator), Some((1, 2, 3)));
    assert_eq!(role(Role::UserTestingValidator), None, "no turn, no row");

    let template = CycleTemplate::engineering();
    assert_eq!(report.tokens_by_lane.len(), template.lanes.len());
    for (row, lane) in report.tokens_by_lane.iter().zip(&template.lanes) {
        assert_eq!(row.lane, lane.id, "one row per template lane, in order");
    }
    let lane = |id: &str| {
        let row = report
            .tokens_by_lane
            .iter()
            .find(|row| row.lane.as_str() == id)
            .expect("lane row");
        (row.tokens.input, row.tokens.output, row.tokens.cache_read)
    };
    assert_eq!(lane("build"), (150, 15, 1500));
    assert_eq!(lane("plan"), (10, 20, 30));
    assert_eq!(lane("gate"), (0, 0, 0), "a lane with no turn is a zero row");

    // Cost = tokens x the model's rate x the role multiplier, named by its key.
    let worker = report
        .cost_by_role
        .iter()
        .find(|row| row.role == Role::Worker)
        .expect("worker cost");
    assert_eq!(worker.multiplier_key, "roles.worker");
    assert!(close(worker.multiplier, 2.0));
    assert!(close(
        worker.cost.amount,
        usd([150, 15, 1500], [2.0, 10.0, 0.10]) * 2.0
    ));
    assert_eq!(worker.cost.price_table, DATE);
    let orchestrator = report
        .cost_by_role
        .iter()
        .find(|row| row.role == Role::Orchestrator)
        .expect("orchestrator cost");
    assert!(
        close(orchestrator.multiplier, 1.0),
        "1.0 when none is configured"
    );
    assert!(close(
        orchestrator.cost.amount,
        usd([10, 20, 30], [10.0, 50.0, 0.25])
    ));
}

#[test]
fn m0_role_sums_equal_lane_sums_equal_the_golden_turn_payloads() {
    let events = m0_events();
    let report = report(&events, m0_end(&events));

    let mut total = [0_u64; 3];
    for event in &events {
        for (sum, count) in total.iter_mut().zip(counts(event)) {
            *sum += count;
        }
    }
    let sum = |rows: Vec<&observatory_core::metrics::TokenCounts>| {
        rows.iter().fold([0_u64; 3], |acc, c| {
            [acc[0] + c.input, acc[1] + c.output, acc[2] + c.cache_read]
        })
    };
    let by_role = sum(report.tokens_by_role.iter().map(|r| &r.tokens).collect());
    let by_lane = sum(report.tokens_by_lane.iter().map(|r| &r.tokens).collect());
    println!("golden total {total:?} roles {by_role:?} lanes {by_lane:?}");
    assert_eq!(by_role, total);
    assert_eq!(by_lane, total);
    assert_eq!(report.tokens_by_role.len(), 4, "four roles have turns");
    assert_eq!(report.tokens_by_lane.len(), 7);
    println!(
        "{}",
        serde_json::to_string_pretty(&report.cost_by_role).unwrap()
    );
}

#[test]
fn tokens_burnt_in_waste_bands_are_attributed_per_cause() {
    let template = CycleTemplate::engineering();
    let f = "f";
    let events = vec![
        ev("w1", EventKind::FileTouched, 100)
            .feature(f)
            .milestone("m1")
            .lasting(100),
        ev("q1", EventKind::QuestionAsked, 100)
            .feature(f)
            .milestone("m1"),
        ev("q2", EventKind::QuestionAnswered, 300)
            .feature(f)
            .milestone("m1"),
        ev("w2", EventKind::FileTouched, 400)
            .feature(f)
            .milestone("m1")
            .lasting(100),
        ev("p1", EventKind::MissionPaused, 500)
            .feature(f)
            .milestone("m1"),
        ev("r1", EventKind::MissionResumed, 700)
            .feature(f)
            .milestone("m1"),
        ev("w3", EventKind::FileTouched, 800)
            .feature(f)
            .milestone("m1")
            .lasting(100),
        turn("t50", 50, Role::Worker, SONNET, "build", [1, 0, 0]).feature(f),
        turn("t150", 150, Role::Worker, SONNET, "build", [10, 0, 0]).feature(f),
        turn("t250", 250, Role::Worker, SONNET, "build", [100, 0, 0]).feature(f),
        turn("t450", 450, Role::Worker, SONNET, "build", [1000, 0, 0]).feature(f),
        turn("t600", 600, Role::Worker, SONNET, "build", [10_000, 0, 0]).feature(f),
        turn("t750", 750, Role::Worker, SONNET, "build", [100_000, 0, 0]).feature(f),
    ];

    let report = report(&events, t(900));
    let waste = &report.tokens_in_waste_by_cause;
    let tokens = |cause: WasteCause| waste.by_cause[&cause].input;

    assert_eq!(WasteCause::ALL.len(), 8);
    for cause in WasteCause::ALL {
        assert!(
            waste.by_cause.contains_key(&cause),
            "all eight causes are present"
        );
    }
    assert_eq!(
        tokens(WasteCause::WaitingHumanQuestion),
        110,
        "t150 and t250, once each"
    );
    assert_eq!(
        tokens(WasteCause::IdleUnscheduled),
        1000,
        "t450 sits in the gap before the stop"
    );
    assert_eq!(
        waste.throttled.input, 10_000,
        "t600 is inside the stop, reported apart"
    );
    let burnt: u64 = waste.by_cause.values().map(|c| c.input).sum();
    assert_eq!(
        burnt + waste.throttled.input,
        110 + 1000 + 10_000,
        "no turn counted twice"
    );
    let json = serde_json::to_value(&report).expect("serializes");
    assert_eq!(
        json["tokens_in_waste_by_cause"]
            .as_object()
            .map(|map| map.len()),
        Some(9),
        "eight causes plus throttled"
    );
    assert!(template.lanes.len() > 1);
}

#[test]
fn budget_meter_compares_against_optional_budget() {
    let off = budget_meter(0.0, None, 3.125, DATE, "USD");
    assert!(!off.enabled);
    assert_eq!(off.budget, None);
    assert_eq!(off.fraction, None, "no fraction with budget 0");
    assert!(close(off.cost_so_far, 3.125));

    let on = budget_meter(12.5, None, 3.125, DATE, "USD");
    assert!(on.enabled);
    assert_eq!(on.budget, Some(12.5));
    assert!(close(on.fraction.expect("a fraction"), 0.25));
    assert_eq!(on.currency, "USD");
    assert_eq!(on.price_table, DATE);

    let launched = budget_meter(12.5, Some(25.0), 3.125, DATE, "USD");
    assert_eq!(
        launched.budget,
        Some(25.0),
        "the launch budget_usd wins over the config key"
    );
    assert!(close(launched.fraction.expect("a fraction"), 0.125));

    let zero_launch = budget_meter(0.0, Some(0.0), 3.125, DATE, "USD");
    assert!(!zero_launch.enabled);
}

fn comparison_stream() -> Vec<Event> {
    let mut events = vec![
        ev("f1a", EventKind::FileTouched, 100)
            .feature("f1")
            .lasting(100),
        ev("f1b", EventKind::FindingBlocking, 150)
            .feature("f1")
            .by(Role::ScrutinyValidator),
        ev("f1c", EventKind::FileTouched, 250)
            .feature("f1")
            .lasting(100),
        ev("f1d", EventKind::ReworkResolved, 300).feature("f1"),
        ev("f1e", EventKind::TaskDone, 310).feature("f1"),
        ev("f1f", EventKind::ScrutinyPassed, 400)
            .feature("f1")
            .by(Role::ScrutinyValidator),
        ev("f2a", EventKind::FileTouched, 1000)
            .feature("f2")
            .lasting(50),
        ev("f2b", EventKind::TaskDone, 1010).feature("f2"),
        ev("f2c", EventKind::ScrutinyPassed, 1100)
            .feature("f2")
            .by(Role::ScrutinyValidator),
        ev("f3a", EventKind::TaskStarted, 1200).feature("f3"),
    ];
    for event in &mut events {
        if event.role == Role::Worker {
            event.model = Some(ModelId(SONNET.to_owned()));
        } else {
            event.model = Some(ModelId(OPUS.to_owned()));
        }
    }
    events.extend([
        turn(
            "tw",
            20,
            Role::Worker,
            SONNET,
            "build",
            [1_000_000, 100_000, 0],
        ),
        turn(
            "to",
            30,
            Role::Orchestrator,
            FABLE,
            "plan",
            [100_000, 10_000, 0],
        ),
        turn(
            "ts",
            40,
            Role::ScrutinyValidator,
            OPUS,
            "scrutiny",
            [500_000, 0, 0],
        ),
    ]);
    events
}

#[test]
fn model_comparison_groups_by_role_and_model() {
    let events = comparison_stream();
    let report = report(&events, t(1300));

    let row = |role: Role, model: &str| {
        report
            .model_comparison
            .iter()
            .find(|row| row.role == role && row.model.as_ref().is_some_and(|m| m.0 == model))
            .unwrap_or_else(|| panic!("a row for {role:?} on {model}"))
    };
    let worker = row(Role::Worker, SONNET);
    let orchestrator = row(Role::Orchestrator, FABLE);
    let scrutiny = row(Role::ScrutinyValidator, OPUS);

    assert_eq!(
        worker.completed_features, 2,
        "f1 and f2 have task.done and scrutiny.passed; f3 does not"
    );
    let worker_cost = usd([1_000_000, 100_000, 0], [2.0, 10.0, 0.10]);
    assert!(close(worker.cost.amount, worker_cost));
    assert_eq!(worker.cost.price_table, DATE);
    assert!(close(
        worker.cost_per_completed_feature.expect("a figure"),
        worker_cost / 2.0
    ));
    // f1: [0,100] first pass, [150,250] inside the finding window [150,300]; f2: [950,1000].
    assert!(
        close(worker.first_pass_active_s, 150.0),
        "{}",
        worker.first_pass_active_s
    );
    assert!(close(
        worker
            .first_pass_active_s_per_completed_feature
            .expect("a figure"),
        75.0
    ));
    assert!(close(
        orchestrator.cost_per_completed_feature.expect("a figure"),
        usd([100_000, 10_000, 0], [10.0, 50.0, 0.25]) / 2.0
    ));
    assert!(close(
        scrutiny.cost_per_completed_feature.expect("a figure"),
        usd([500_000, 0, 0], [4.0, 20.0, 0.20]) / 2.0
    ));
    assert!(close(scrutiny.first_pass_active_s, 0.0));

    let none = report_with(
        &[ev("x", EventKind::TaskStarted, 0).feature("f")],
        &prices(),
        t(10),
    );
    assert!(none.model_comparison.is_empty());
}

#[test]
fn reconciliation_prints_the_signed_difference_per_session() {
    let mut s1a = turn("a", 10, Role::Worker, SONNET, "build", [10, 20, 30]);
    s1a.session_id = Some(SessionId("s1".into()));
    let mut s1b = turn("b", 20, Role::Worker, SONNET, "build", [1, 2, 3]);
    s1b.session_id = Some(SessionId("s1".into()));
    let mut s2 = turn("c", 30, Role::Worker, SONNET, "build", [5, 5, 5]);
    s2.session_id = Some(SessionId("s2".into()));
    let mut reported = ev("u", EventKind::UsageReported, 40)
        .model(SONNET)
        .reporting(TokenUsage {
            input: 10,
            output: 25,
            cache_read: 30,
            cache_write: 0,
            credits: None,
        });
    reported.session_id = Some(SessionId("s1".into()));

    let report = report(&[s1a, s1b, s2, reported], t(50));

    let rec = &report.reconciliation;
    assert_eq!(rec.first_ts, Some(t(10)));
    assert_eq!(rec.last_ts, Some(t(30)));
    let row = |id: &str| {
        rec.sessions
            .iter()
            .find(|row| row.session.0 == id)
            .expect("row")
    };
    let s1 = row("s1");
    assert!(s1.has_sidecar);
    assert_eq!(
        (s1.turns.input, s1.turns.output, s1.turns.cache_read),
        (11, 22, 33)
    );
    assert_eq!(
        (
            s1.reported.input,
            s1.reported.output,
            s1.reported.cache_read
        ),
        (10, 25, 30)
    );
    assert_eq!(
        (
            s1.difference.input,
            s1.difference.output,
            s1.difference.cache_read
        ),
        (1, -3, 3),
        "turns minus the sidecar"
    );
    let s2 = row("s2");
    assert!(!s2.has_sidecar);
    assert_eq!(
        (
            s2.difference.input,
            s2.difference.output,
            s2.difference.cache_read
        ),
        (5, 5, 5)
    );
}

#[test]
fn overhead_is_separate_unless_apportionment_is_equal() {
    let events = vec![
        ev("a", EventKind::FileTouched, 100)
            .feature("f1")
            .lasting(10),
        ev("b", EventKind::FileTouched, 200)
            .feature("f2")
            .lasting(10),
        ev("c", EventKind::PlanRevised, 300)
            .by(Role::Orchestrator)
            .lasting(60),
        turn(
            "t1",
            150,
            Role::Orchestrator,
            FABLE,
            "plan",
            [1_000_000, 0, 0],
        ),
        turn("t2", 160, Role::Worker, SONNET, "build", [7, 0, 0]).feature("f1"),
    ];
    let mission = |apportionment| {
        cost_report(&CostInput {
            events: &events,
            template: &CycleTemplate::engineering(),
            notes: &Notes::default(),
            prices: &prices(),
            end: t(400),
            budget: 0.0,
            launch_budget_usd: None,
            apportionment,
        })
    };

    let separate = mission(OverheadApportionment::Separate).overhead;
    assert_eq!(separate.label, "separate");
    assert_eq!(separate.tokens.input, 1_000_000, "the turn no feature owns");
    assert!(close(separate.cost.amount, 10.0));
    assert_eq!(separate.cost.price_table, DATE);
    assert!(close(separate.active_s, 60.0));
    assert!(separate.per_feature.is_empty());

    let equal = mission(OverheadApportionment::Equal).overhead;
    assert_eq!(equal.label, "apportioned");
    let shares: Vec<(FeatureId, f64)> = equal
        .per_feature
        .iter()
        .map(|share| (share.feature.clone(), share.cost.amount))
        .collect();
    assert_eq!(shares.len(), 2);
    for (_, amount) in shares {
        assert!(close(amount, 5.0));
    }
}

#[test]
fn a_model_without_a_price_row_is_counted_not_priced() {
    let mut orphan = turn("o", 10, Role::Worker, SONNET, "build", [1, 2, 3]);
    orphan.session_id = None;
    let events = vec![
        turn("a", 10, Role::Worker, "kimi-k3", "build", [1000, 0, 0]),
        turn("b", 20, Role::Worker, SONNET, "elsewhere", [10, 0, 0]),
        orphan,
    ];

    let report = report(&events, t(30));

    assert_eq!(report.unpriced.len(), 1);
    assert_eq!(report.unpriced[0].model, Some(ModelId("kimi-k3".into())));
    assert_eq!(report.unpriced[0].tokens.input, 1000);
    assert_eq!(report.total_tokens.input, 1011, "tokens are all counted");
    assert!(close(
        report.cost_so_far.amount,
        usd([10, 0, 0], [2.0, 10.0, 0.10]) + usd([1, 2, 3], [2.0, 10.0, 0.10])
    ));
    assert_eq!(
        report.tokens_other_lanes.input, 10,
        "a lane the template lacks"
    );
    let lanes: u64 = report.tokens_by_lane.iter().map(|l| l.tokens.input).sum();
    assert_eq!(
        lanes + report.tokens_other_lanes.input,
        report.total_tokens.input
    );
    assert!(
        report
            .reconciliation
            .sessions
            .iter()
            .any(|row| row.session.0 == "unjoined" && row.turns.input == 1),
        "a turn with no session is listed under its own id"
    );
}

#[test]
fn a_price_table_names_its_error() {
    assert!(
        PriceTable::parse("currency = \"USD\"").is_err(),
        "no effective date"
    );
    let missing =
        PriceTable::load(std::path::Path::new("no/such/prices.toml")).expect_err("a missing file");
    assert!(missing.to_string().contains("no/such/prices.toml"));
    let table = PriceTable::parse("effective = \"2026-01-01\"").expect("a table with no rows");
    assert_eq!(table.currency(), "USD");
    assert!(table.rates(SONNET).is_none());
    assert!(
        table
            .session_cost(
                &TokenUsage {
                    input: 1,
                    output: 1,
                    cache_read: 1,
                    cache_write: 1,
                    credits: None
                },
                Some(SONNET)
            )
            .is_none()
    );
}

fn metrics_of(
    events: &[Event],
    prices: &PriceTable,
    end: Timestamp,
) -> observatory_core::metrics::MetricsReport {
    metrics_report(&CostInput {
        events,
        template: &CycleTemplate::engineering(),
        notes: &Notes::default(),
        prices,
        end,
        budget: 0.0,
        launch_budget_usd: None,
        apportionment: OverheadApportionment::Separate,
    })
}

fn session_total() -> TokenUsage {
    TokenUsage {
        input: 1_000_000,
        output: 100_000,
        cache_read: 2_000_000,
        cache_write: 400_000,
        credits: None,
    }
}

#[test]
fn a_row_cost_prices_the_cache_write_tokens_of_its_session_report() {
    let events = vec![
        ev("fd", EventKind::FeatureDefined, 0)
            .feature("f1")
            .milestone("m1")
            .by(Role::Orchestrator),
        turn(
            "t1",
            10,
            Role::Worker,
            SONNET,
            "build",
            [1_000_000, 100_000, 2_000_000],
        )
        .feature("f1")
        .milestone("m1"),
        usage_report("u1", 20, Role::Worker, SONNET, "build", session_total())
            .feature("f1")
            .milestone("m1"),
    ];

    let found = metrics_of(&events, &prices(), t(30));

    // Turns: 1 M input at 2.0, 0.1 M output at 10.0, 2 M cache read at 0.10; the sidecar's
    // 0.4 M cache write at 2.5.
    let by_hand = 2.0 + 1.0 + 0.2 + 1.0;
    for (scope, id) in [(RowScope::Feature, "f1"), (RowScope::Milestone, "m1")] {
        let row = found
            .rows
            .iter()
            .find(|row| row.scope == scope && row.id == id)
            .expect("a row");
        assert!(
            (row.cost_usd - by_hand).abs() < 0.01,
            "{id}: {} against {by_hand}",
            row.cost_usd
        );
        assert_eq!(row.tokens_cache_write, 400_000, "{id}");
        assert_eq!(
            row.tokens_input, 1_000_000,
            "{id}: the turns' tokens stay the turns' tokens, no double count"
        );
    }
}

#[test]
fn the_row_cost_of_m0_ci_rust_job_linux_equals_the_four_class_recompute() {
    let events = m0_events();
    let table = prices();
    let found = metrics_of(&events, &table, m0_end(&events));
    let row = found
        .rows
        .iter()
        .find(|row| row.scope == RowScope::Feature && row.id == "m0-ci-rust-job-linux")
        .expect("the M0 capture holds the feature");

    let mut recompute = 0.0;
    for event in events
        .iter()
        .filter(|e| e.feature_id.as_ref().map(|f| f.0.as_str()) == Some(row.id.as_str()))
    {
        let multiplier = table.multiplier(event.role);
        match &event.kind {
            EventKind::TurnUsage {
                input,
                output,
                cache_read,
                model,
                ..
            } => {
                let model = model.as_ref().or(event.model.as_ref());
                if let Some(rates) = model.and_then(|m| table.rates(&m.0)) {
                    recompute += (*input as f64 * rates.input_per_m
                        + *output as f64 * rates.output_per_m
                        + *cache_read as f64 * rates.cache_read_per_m)
                        / 1e6
                        * multiplier;
                }
            }
            EventKind::UsageReported => {
                let rates = event.model.as_ref().and_then(|m| table.rates(&m.0));
                if let (Some(usage), Some(rates)) = (event.usage, rates) {
                    recompute +=
                        usage.cache_write as f64 * rates.cache_write_per_m / 1e6 * multiplier;
                }
            }
            _ => {}
        }
    }
    println!(
        "m0-ci-rust-job-linux: csv cost {} four-class recompute {recompute}, cache write {}",
        row.cost_usd, row.tokens_cache_write
    );
    assert!(row.tokens_cache_write > 0);
    assert!(
        (row.cost_usd - recompute).abs() < 0.0001 + 1e-9,
        "{} against {recompute}",
        row.cost_usd
    );
}

#[test]
fn an_unpriced_model_keeps_its_cache_write_tokens_in_the_unpriced_list() {
    let events = vec![
        turn("a", 10, Role::Worker, "kimi-k3", "build", [1000, 0, 0]),
        usage_report(
            "u",
            20,
            Role::Worker,
            "kimi-k3",
            "build",
            cache_write(400_000),
        ),
        usage_report("v", 25, Role::Worker, SONNET, "build", cache_write(0)),
    ];

    let report = report(&events, t(30));

    let json = serde_json::to_value(&report).expect("serializes");
    let unpriced = json["unpriced"].as_array().expect("a list");
    assert_eq!(unpriced.len(), 1, "{unpriced:?}");
    assert_eq!(unpriced[0]["model"], "kimi-k3");
    assert_eq!(unpriced[0]["tokens"]["input"], 1000);
    assert_eq!(
        unpriced[0]["cache_write"], 400_000,
        "counted with the model's other tokens, not priced"
    );
    assert!(close(report.cost_so_far.amount, 0.0), "nothing is priced");
}

#[test]
fn role_lane_milestone_mission_and_comparison_costs_include_the_cache_write_term() {
    let text = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../config/prices/2026-10-22.toml"),
    )
    .expect("the price table exists");
    let table = PriceTable::parse(&text.replace("worker = 1.0", "worker = 2.0")).expect("parses");
    let events = vec![
        turn("t1", 10, Role::Worker, SONNET, "build", [1_000_000, 0, 0])
            .feature("f1")
            .milestone("m1"),
        usage_report(
            "u1",
            20,
            Role::Worker,
            SONNET,
            "build",
            cache_write(400_000),
        )
        .feature("f1")
        .milestone("m1"),
        usage_report(
            "u2",
            30,
            Role::Orchestrator,
            FABLE,
            "plan",
            cache_write(100_000),
        ),
    ];

    let report = report_with(&events, &table, t(40));

    // Worker: (2.0 + 1.0) x 2.0; orchestrator: 0.1 M cache write at 12.5.
    let worker = 6.0;
    let orchestrator = 1.25;
    let role = |role: Role| {
        report
            .cost_by_role
            .iter()
            .find(|row| row.role == role)
            .map(|row| row.cost.amount)
    };
    assert!(close(role(Role::Worker).expect("worker"), worker));
    assert!(close(
        role(Role::Orchestrator).expect("orchestrator"),
        orchestrator
    ));
    let lane = |id: &str| {
        report
            .tokens_by_lane
            .iter()
            .find(|row| row.lane.as_str() == id)
            .map(|row| row.cost.amount)
    };
    assert!(close(lane("build").expect("build"), worker));
    assert!(close(lane("plan").expect("plan"), orchestrator));
    let milestone = |id: Option<&str>| {
        report
            .tokens_by_milestone
            .iter()
            .find(|row| row.milestone.as_ref().map(|m| m.0.as_str()) == id)
            .map(|row| row.cost.amount)
    };
    assert!(close(milestone(Some("m1")).expect("m1"), worker));
    assert!(close(milestone(None).expect("no milestone"), orchestrator));
    assert!(close(report.cost_so_far.amount, worker + orchestrator));
    assert!(
        close(report.overhead.cost.amount, orchestrator),
        "the report with no feature is mission level"
    );
    let group = |role: Role| {
        report
            .model_comparison
            .iter()
            .find(|row| row.role == role)
            .map(|row| row.cost.amount)
    };
    assert!(close(group(Role::Worker).expect("worker group"), worker));
    assert!(close(
        group(Role::Orchestrator).expect("orchestrator group"),
        orchestrator
    ));
    assert_eq!(report.total_tokens.input, 1_000_000, "no cache write here");
}
