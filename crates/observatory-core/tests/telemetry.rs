//! `core::telemetry` (architecture §10, SPEC §5.7, D-010): the series plan, the label allow-list and the
//! overflow bounds. Pure: no exporter, no keychain, no network.

use observatory_core::config::Config;
use observatory_core::model::Role;
use observatory_core::telemetry::{
    BoundedLabel, InstrumentKind, Label, LabelError, SeriesPlan, TelemetryReport, TelemetryStatus,
    parse_header, resolve_start, start_check,
};

const ELEVEN: [&str; 11] = [
    "observatory.mission.cycle_seconds",
    "observatory.feature.cycle_seconds",
    "observatory.waste_seconds",
    "observatory.rework_windows",
    "observatory.fix_loops",
    "observatory.tokens.input",
    "observatory.tokens.output",
    "observatory.tokens.cache_read",
    "observatory.tokens.cache_write",
    "observatory.cost_usd",
    "observatory.unknown_events",
];

fn series_of(plan: &SeriesPlan, name: &str) -> u32 {
    plan.instruments
        .iter()
        .find(|instrument| instrument.name == name)
        .unwrap_or_else(|| panic!("{name} is not in the plan"))
        .series
}

#[test]
fn exported_series_count_is_under_budget_on_worst_case_labels() {
    let cfg = Config::default().telemetry;
    let plan = SeriesPlan::from(&cfg);

    // 9 + 8 + 64 + 32 + 8 + 48 + 12 + 1, the terms of SPEC §5.7.
    assert_eq!(series_of(&plan, "observatory.mission.cycle_seconds"), 9);
    assert_eq!(series_of(&plan, "observatory.feature.cycle_seconds"), 8);
    assert_eq!(series_of(&plan, "observatory.waste_seconds"), 64);
    assert_eq!(series_of(&plan, "observatory.rework_windows"), 32);
    assert_eq!(series_of(&plan, "observatory.fix_loops"), 8);
    let tokens: u32 = [
        "observatory.tokens.input",
        "observatory.tokens.output",
        "observatory.tokens.cache_read",
        "observatory.tokens.cache_write",
    ]
    .iter()
    .map(|name| series_of(&plan, name))
    .sum();
    assert_eq!(tokens, 48);
    assert_eq!(series_of(&plan, "observatory.cost_usd"), 12);
    assert_eq!(series_of(&plan, "observatory.unknown_events"), 1);

    assert_eq!(plan.worst_case, 182);
    assert!(plan.worst_case < cfg.max_series);
    assert_eq!(cfg.max_series, 200);
}

#[test]
fn exporter_refuses_to_start_when_worst_case_reaches_max_series() {
    let mut cfg = Config::default().telemetry;
    cfg.enabled = true;

    cfg.max_series = 182;
    let refused = start_check(&cfg).expect_err("182 is not under 182");
    assert!(!refused.enabled);
    assert!(
        refused.reason.contains("182") && refused.reason.contains("max_series"),
        "the reason names the computed count and the limit: {}",
        refused.reason
    );
    println!("TelemetryStatus.reason = {}", refused.reason);

    cfg.max_series = 100;
    assert!(start_check(&cfg).is_err());

    cfg.max_series = 183;
    assert_eq!(start_check(&cfg).expect("183 is above 182").worst_case, 182);

    cfg.max_series = 200;
    assert_eq!(start_check(&cfg).expect("200 is accepted").worst_case, 182);
}

#[test]
fn a_plan_with_a_wider_histogram_or_more_milestones_can_cross_the_budget() {
    let mut cfg = Config::default().telemetry;
    cfg.max_milestone_labels = 12;
    let plan = SeriesPlan::from(&cfg);
    assert!(plan.worst_case > 182);
    assert!(start_check(&cfg).is_err());

    let mut cfg = Config::default().telemetry;
    cfg.buckets.mission_cycle_seconds.push(345_600);
    assert_eq!(SeriesPlan::from(&cfg).worst_case, 183);
}

#[test]
fn labels_outside_allowed_set_are_rejected() {
    let accepted = ["mission", "milestone", "role", "model", "cause"];
    for name in accepted {
        let label = Label::parse(name).unwrap_or_else(|_| panic!("{name} must be accepted"));
        assert_eq!(label.as_str(), name);
    }
    println!("accepted: {accepted:?}");

    let rejected = [
        "session_id",
        "event_id",
        "feature",
        "feature_id",
        "status",
        "class",
        "unit",
        "price_table",
        "",
        "Mission",
        "some free text",
    ];
    for name in rejected {
        assert_eq!(
            Label::parse(name),
            Err(LabelError {
                name: name.to_owned()
            }),
            "{name} must be rejected"
        );
    }
    println!("rejected: {rejected:?}");

    let plan = SeriesPlan::from(&Config::default().telemetry);
    for instrument in &plan.instruments {
        for label in &instrument.labels {
            assert!(accepted.contains(&label.as_str()));
        }
    }
}

#[test]
fn a_configured_label_outside_the_allow_list_stops_the_start() {
    let mut cfg = Config::default().telemetry;
    cfg.metric_labels_allowed.push("session_id".into());
    let refused = start_check(&cfg).expect_err("session_id is not an allowed label");
    assert!(refused.reason.contains("session_id"), "{}", refused.reason);

    let mut cfg = Config::default().telemetry;
    cfg.metric_labels_allowed.retain(|name| name != "model");
    let refused = start_check(&cfg).expect_err("an instrument carries a label the config dropped");
    assert!(refused.reason.contains("model"), "{}", refused.reason);
}

#[test]
fn label_values_are_bounded_by_overflow_and_other() {
    let cfg = Config::default().telemetry;

    let mut milestones = BoundedLabel::milestone(&cfg);
    let seen: Vec<String> = (1..=9)
        .map(|n| milestones.value(&format!("m{n}")).to_owned())
        .collect();
    assert_eq!(seen[..7], ["m1", "m2", "m3", "m4", "m5", "m6", "m7"]);
    assert_eq!(seen[7], "overflow");
    assert_eq!(seen[8], "overflow");
    println!("milestone values for m1..m9: {seen:?}");
    assert_eq!(milestones.value("m3"), "m3", "a named value stays named");
    assert_eq!(
        milestones.value("m8"),
        "overflow",
        "an overflowed id stays overflow"
    );

    let mut models = BoundedLabel::model();
    let seen: Vec<String> = ["a", "b", "c", "d", "e"]
        .iter()
        .map(|id| models.value(id).to_owned())
        .collect();
    assert_eq!(seen, ["a", "b", "c", "d", "other"]);
    println!("model values for a..e: {seen:?}");
    assert_eq!(models.value("f"), "other");
    assert_eq!(models.value("a"), "a");
}

#[test]
fn the_role_label_has_three_values_and_skips_non_agents() {
    let labels: Vec<Option<&str>> = Role::ALL
        .into_iter()
        .map(observatory_core::telemetry::role_label)
        .collect();
    assert_eq!(
        labels,
        [
            Some("orchestrator"),
            Some("worker"),
            Some("validator"),
            Some("validator"),
            None,
            None,
            None
        ]
    );
}

#[test]
fn exactly_eleven_instruments_are_exported() {
    let cfg = Config::default().telemetry;
    let plan = SeriesPlan::from(&cfg);

    let names: Vec<&str> = plan.instruments.iter().map(|i| i.name.as_str()).collect();
    assert_eq!(names, ELEVEN);
    assert_eq!(
        names,
        cfg.mission_metrics
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
    );
    assert!(plan.rejected.is_empty());

    let histograms: Vec<(&str, &Vec<u64>)> = plan
        .instruments
        .iter()
        .filter_map(|i| match &i.kind {
            InstrumentKind::Histogram { buckets } => Some((i.name.as_str(), buckets)),
            InstrumentKind::Counter => None,
        })
        .collect();
    assert_eq!(
        histograms,
        [
            (
                "observatory.mission.cycle_seconds",
                &vec![3600, 7200, 14400, 28800, 86400, 172_800]
            ),
            (
                "observatory.feature.cycle_seconds",
                &vec![600, 1800, 3600, 7200, 14400]
            ),
        ]
    );
    let counters = plan
        .instruments
        .iter()
        .filter(|i| i.kind == InstrumentKind::Counter)
        .count();
    assert_eq!(counters, 9);

    for instrument in &plan.instruments {
        assert_eq!(
            instrument.unit(),
            None,
            "{} carries an OTLP unit",
            instrument.name
        );
    }
    assert!(
        !names
            .iter()
            .any(|n| n.contains("active_agents") || n.contains("overhead"))
    );
}

#[test]
fn an_instrument_outside_the_contract_stops_the_start() {
    let mut cfg = Config::default().telemetry;
    cfg.mission_metrics.push("observatory.active_agents".into());
    let plan = SeriesPlan::from(&cfg);
    assert_eq!(plan.rejected, ["observatory.active_agents"]);
    assert_eq!(plan.instruments.len(), 11);
    let refused = start_check(&cfg).expect_err("an extra instrument is refused");
    assert!(
        refused.reason.contains("observatory.active_agents"),
        "{}",
        refused.reason
    );

    let mut cfg = Config::default().telemetry;
    cfg.mission_metrics.push("observatory.cost_usd".into());
    assert_eq!(SeriesPlan::from(&cfg).rejected, ["observatory.cost_usd"]);

    let mut cfg = Config::default().telemetry;
    cfg.mission_metrics.pop();
    let refused = start_check(&cfg).expect_err("a missing instrument is refused");
    assert!(
        refused.reason.contains("observatory.unknown_events"),
        "{}",
        refused.reason
    );
}

fn enabled() -> observatory_core::config::TelemetryConfig {
    let mut cfg = Config::default().telemetry;
    cfg.enabled = true;
    cfg
}

#[test]
fn telemetry_defaults_point_at_the_local_collector() {
    let cfg = Config::default().telemetry;
    assert!(!cfg.enabled);
    assert_eq!(cfg.endpoint, "http://localhost:4318");
    assert_eq!(cfg.headers_keychain_item, "");

    let off = resolve_start(&cfg, |_| panic!("an off config never reads the keychain"))
        .expect_err("export is off by default");
    assert_eq!(
        off.status,
        TelemetryStatus {
            enabled: false,
            reason: "off in config".into()
        }
    );
    assert_eq!(off.state, "off (config)");
    assert_eq!(off.endpoint, "http://localhost:4318");
}

#[test]
fn header_is_split_on_first_equals_and_never_logged() {
    assert_eq!(
        parse_header("Authorization=Basic YWJj="),
        Some(("Authorization", "Basic YWJj="))
    );
    assert_eq!(parse_header("nodelimiter"), None);
    assert_eq!(parse_header("=value"), None, "a header needs a name");
    assert_eq!(
        parse_header("Authorization=Basic YWJj=\n").map(|h| h.1),
        Some("Basic YWJj=")
    );

    let mut cfg = enabled();
    cfg.headers_keychain_item = "droid-observatory/otlp-headers-absent".into();
    let secret_text = "Authorization=Basic YWJj=";
    let start = resolve_start(&cfg, |_| Ok(Some(secret_text.to_owned())))
        .expect("an item with a header starts the exporter");
    let header = start.header.as_ref().expect("the header was read");
    assert_eq!(header.name(), "Authorization");
    assert_eq!(header.expose_value(), "Basic YWJj=");

    // Nothing the app can print about the start carries the value or its first 8 characters.
    let printed = format!(
        "{start:?} {header:?} {:?}",
        start.report("2026-10-30T00:00:00Z")
    );
    for leak in ["Basic ", "YWJj", "Authorization="] {
        assert!(!printed.contains(leak), "{leak:?} leaked into: {printed}");
    }
    assert!(printed.contains("<redacted>"), "{printed}");
}

#[test]
fn telemetry_stays_off_without_keychain_item_and_ui_says_so() {
    // (a) the default empty item: the keychain is never read, the exporter starts without a header.
    let cfg = enabled();
    assert_eq!(cfg.headers_keychain_item, "");
    let start = resolve_start(&cfg, |service| {
        panic!("an empty headers_keychain_item never reads the keychain, asked for {service}")
    })
    .expect("enabled with no item starts");
    assert!(start.header.is_none());
    assert_eq!(start.endpoint, "http://localhost:4318");
    let report: TelemetryReport = start.report("2026-10-30T00:00:00Z");
    assert!(report.status.enabled);
    assert_eq!(report.state, "exporting since 2026-10-30T00:00:00Z");
    assert_eq!(report.endpoint, "http://localhost:4318");
    println!("(a) {} / {}", report.state, report.endpoint);

    // (b) a configured item that is absent keeps export off and says so.
    let mut cfg = enabled();
    cfg.headers_keychain_item = "droid-observatory/otlp-headers-absent".into();
    let mut asked = Vec::new();
    let off = resolve_start(&cfg, |service| {
        asked.push(service.to_owned());
        Ok(None)
    })
    .expect_err("an absent item keeps export off");
    assert_eq!(asked, ["droid-observatory/otlp-headers-absent"]);
    assert_eq!(
        off.status,
        TelemetryStatus {
            enabled: false,
            reason: "no keychain item".into()
        }
    );
    assert_eq!(
        off.state,
        "off, no keychain item droid-observatory/otlp-headers-absent"
    );
    assert_eq!(off.endpoint, "http://localhost:4318");
    println!("(b) {} / {}", off.state, off.endpoint);

    // A refused lookup and an item without `name=value` are off states too, never a silent start.
    let refused = resolve_start(&cfg, |_| Err("locked".into())).expect_err("refused lookup");
    assert!(!refused.status.enabled);
    assert!(refused.state.starts_with("off, "), "{}", refused.state);
    let malformed = resolve_start(&cfg, |_| Ok(Some("nodelimiter".into()))).expect_err("malformed");
    assert!(!malformed.status.enabled);
    assert!(!malformed.state.contains("nodelimiter"));

    // The series plan is checked before the keychain: a refused plan reads nothing.
    let mut cfg = enabled();
    cfg.headers_keychain_item = "droid-observatory/otlp-headers-absent".into();
    cfg.max_series = 182;
    let refused = resolve_start(&cfg, |_| panic!("a refused plan never reads the keychain"))
        .expect_err("182 is not under 182");
    assert!(refused.status.reason.contains("max_series"));
}
