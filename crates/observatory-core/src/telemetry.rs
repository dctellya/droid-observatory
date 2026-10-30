//! Telemetry series plan (architecture §10, SPEC §5.7, D-010): the eleven exported instruments,
//! the label allow-list, the bounds on label values and the worst-case series count the exporter
//! checks before it starts. Pure: the exporter that sends the series lives in the app.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::config::TelemetryConfig;
use crate::metrics::{
    Notes, PriceTable, ReworkKind, WasteCause, card_times, completed_features, rework_windows,
    throttled, turns,
};
use crate::model::{CycleTemplate, Event, EventKind, FeatureId, Role, Timestamp};

/// Values the `role` label takes: the orchestrator, the worker and one `validator` value for
/// both validators. People, outside parties and unresolved sessions burn no tokens and are not
/// exported.
pub const ROLE_LABEL_VALUES: u32 = 3;

/// Distinct `model` values a mission keeps; the fifth and later ones export as [`MODEL_OTHER`].
pub const MODEL_LABEL_VALUES: u32 = 4;

/// The `milestone` value of the milestones beyond the bound.
pub const MILESTONE_OVERFLOW: &str = "overflow";

/// The `model` value of the models beyond the bound.
pub const MODEL_OTHER: &str = "other";

/// Series a histogram adds besides its buckets: `+Inf`, `_sum` and `_count`.
const HISTOGRAM_EXTRA_SERIES: u32 = 3;

/// A metric attribute the exporter may send (D-010). Nothing else is ever a label: never a
/// session id, an event id, a feature id or free text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Label {
    Mission,
    Milestone,
    Role,
    Model,
    Cause,
}

impl Label {
    pub const ALL: [Label; 5] = [
        Self::Mission,
        Self::Milestone,
        Self::Role,
        Self::Model,
        Self::Cause,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mission => "mission",
            Self::Milestone => "milestone",
            Self::Role => "role",
            Self::Model => "model",
            Self::Cause => "cause",
        }
    }

    pub fn parse(name: &str) -> Result<Label, LabelError> {
        Self::ALL
            .into_iter()
            .find(|label| label.as_str() == name)
            .ok_or_else(|| LabelError {
                name: name.to_owned(),
            })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("label {name:?} is not one of mission, milestone, role, model, cause")]
pub struct LabelError {
    pub name: String,
}

/// The exported `role` value of a role, or `None` for a role that is not exported.
pub fn role_label(role: Role) -> Option<&'static str> {
    match role {
        Role::Orchestrator => Some("orchestrator"),
        Role::Worker => Some("worker"),
        Role::ScrutinyValidator | Role::UserTestingValidator => Some("validator"),
        Role::Human | Role::External | Role::Unresolved => None,
    }
}

/// A label whose values are bounded: the first `named` distinct ids of a mission keep their own
/// value, every later one reads as `fallback`. One tracker per mission and label.
#[derive(Clone, Debug)]
pub struct BoundedLabel {
    named: usize,
    fallback: &'static str,
    seen: Vec<String>,
}

impl BoundedLabel {
    /// `milestone`: `max_milestone_labels - 1` named values, then `overflow`.
    pub fn milestone(cfg: &TelemetryConfig) -> Self {
        let named = (cfg.max_milestone_labels as usize).saturating_sub(1);
        Self::new(named, MILESTONE_OVERFLOW)
    }

    /// `model`: four named values, then `other`.
    pub fn model() -> Self {
        Self::new(MODEL_LABEL_VALUES as usize, MODEL_OTHER)
    }

    fn new(named: usize, fallback: &'static str) -> Self {
        Self {
            named,
            fallback,
            seen: Vec::new(),
        }
    }

    /// The label value to export for `id`.
    pub fn value(&mut self, id: &str) -> &str {
        if let Some(at) = self.seen.iter().position(|seen| seen == id) {
            return &self.seen[at];
        }
        if self.seen.len() < self.named {
            self.seen.push(id.to_owned());
            return &self.seen[self.seen.len() - 1];
        }
        self.fallback
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InstrumentKind {
    Counter,
    /// Explicit bucket boundaries in seconds.
    Histogram {
        buckets: Vec<u64>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstrumentPlan {
    pub name: String,
    pub kind: InstrumentKind,
    pub labels: Vec<Label>,
    /// Worst-case series this instrument adds.
    pub series: u32,
}

impl InstrumentPlan {
    /// No instrument carries an OTLP unit: Mimir would append a unit suffix to the name.
    pub fn unit(&self) -> Option<&'static str> {
        None
    }
}

/// The instruments the exporter registers and the series they add in the worst case.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SeriesPlan {
    /// The names of `[telemetry].mission_metrics` that are in the contract, in config order.
    pub instruments: Vec<InstrumentPlan>,
    /// Configured names outside the contract or listed twice; the exporter does not start with any.
    pub rejected: Vec<String>,
    pub worst_case: u32,
}

impl From<&TelemetryConfig> for SeriesPlan {
    fn from(cfg: &TelemetryConfig) -> Self {
        let mut instruments: Vec<InstrumentPlan> = Vec::new();
        let mut rejected = Vec::new();
        for name in &cfg.mission_metrics {
            let listed = instruments.iter().any(|planned| &planned.name == name);
            match instrument(name, cfg) {
                Some(planned) if !listed => instruments.push(planned),
                _ => rejected.push(name.clone()),
            }
        }
        let worst_case = instruments.iter().map(|planned| planned.series).sum();
        Self {
            instruments,
            rejected,
            worst_case,
        }
    }
}

/// The shape of a contract instrument under the configured bounds, `None` for any other name.
fn instrument(name: &str, cfg: &TelemetryConfig) -> Option<InstrumentPlan> {
    let milestones = cfg.max_milestone_labels;
    let role_model_pairs = ROLE_LABEL_VALUES * MODEL_LABEL_VALUES;
    let histogram = |buckets: &[u64]| {
        let series = buckets.len() as u32 + HISTOGRAM_EXTRA_SERIES;
        (
            InstrumentKind::Histogram {
                buckets: buckets.to_vec(),
            },
            vec![Label::Mission],
            series,
        )
    };
    let (kind, labels, series) = match name {
        "observatory.mission.cycle_seconds" => histogram(&cfg.buckets.mission_cycle_seconds),
        "observatory.feature.cycle_seconds" => histogram(&cfg.buckets.feature_cycle_seconds),
        "observatory.waste_seconds" => (
            InstrumentKind::Counter,
            vec![Label::Mission, Label::Milestone, Label::Cause],
            milestones * WasteCause::ALL.len() as u32,
        ),
        "observatory.rework_windows" => (
            InstrumentKind::Counter,
            vec![Label::Mission, Label::Milestone, Label::Cause],
            milestones * ReworkKind::ALL.len() as u32,
        ),
        "observatory.fix_loops" => (
            InstrumentKind::Counter,
            vec![Label::Mission, Label::Milestone],
            milestones,
        ),
        "observatory.tokens.input"
        | "observatory.tokens.output"
        | "observatory.tokens.cache_read"
        | "observatory.tokens.cache_write"
        | "observatory.cost_usd" => (
            InstrumentKind::Counter,
            vec![Label::Mission, Label::Role, Label::Model],
            role_model_pairs,
        ),
        "observatory.unknown_events" => (InstrumentKind::Counter, vec![Label::Mission], 1),
        _ => return None,
    };
    Some(InstrumentPlan {
        name: name.to_owned(),
        kind,
        labels,
        series,
    })
}

/// What the UI and the log say about export (`telemetry_status`, architecture §7, §10).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TelemetryStatus {
    pub enabled: bool,
    pub reason: String,
}

impl TelemetryStatus {
    pub fn off(reason: impl Into<String>) -> Self {
        Self {
            enabled: false,
            reason: reason.into(),
        }
    }
}

/// The plan the exporter may start with, or the off status that says why it must not: a
/// configured label outside the allow-list, an instrument outside the contract or a missing one,
/// or a worst case that is not under `[telemetry].max_series`.
pub fn start_check(cfg: &TelemetryConfig) -> Result<SeriesPlan, TelemetryStatus> {
    let refuse = |why: String| TelemetryStatus::off(format!("series plan refused: {why}"));

    let mut allowed = BTreeSet::new();
    for name in &cfg.metric_labels_allowed {
        let label = Label::parse(name).map_err(|error| refuse(error.to_string()))?;
        allowed.insert(label);
    }
    let plan = SeriesPlan::from(cfg);
    if !plan.rejected.is_empty() {
        return Err(refuse(format!(
            "metrics outside the contract: {}",
            plan.rejected.join(", ")
        )));
    }
    let missing = contract_names()
        .into_iter()
        .filter(|name| !plan.instruments.iter().any(|planned| planned.name == *name))
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(refuse(format!(
            "contract metrics missing from the config: {}",
            missing.join(", ")
        )));
    }
    for planned in &plan.instruments {
        if let Some(label) = planned.labels.iter().find(|label| !allowed.contains(label)) {
            return Err(refuse(format!(
                "{} carries label {} that metric_labels_allowed does not list",
                planned.name,
                label.as_str()
            )));
        }
    }
    if plan.worst_case >= cfg.max_series {
        return Err(refuse(format!(
            "worst case {} series is not under max_series {}",
            plan.worst_case, cfg.max_series
        )));
    }
    Ok(plan)
}

/// The eleven names of the contract (SPEC §5.7).
fn contract_names() -> [&'static str; 11] {
    [
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
    ]
}

/// One header of the collector request, read from the keychain item `headers_keychain_item`. The
/// value has no `Display` and prints as `<redacted>` under `Debug`; the HTTP client reads it once
/// per request through `expose_value`.
#[derive(Clone, PartialEq, Eq)]
pub struct OtlpHeader {
    name: String,
    value: String,
}

impl OtlpHeader {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn expose_value(&self) -> &str {
        &self.value
    }
}

impl fmt::Debug for OtlpHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "OtlpHeader({}: <redacted>)", self.name)
    }
}

/// Splits a keychain string `name=value` on the first `=` only: a value may hold more of them
/// (`Authorization=Basic YWJj=`). The collector of the readiness report answered 401 to the whole
/// string and 200 to the split. A string with no `=` or an empty name is `None`.
pub fn parse_header(text: &str) -> Option<(&str, &str)> {
    let (name, value) = text.trim().split_once('=')?;
    let name = name.trim();
    (!name.is_empty()).then_some((name, value))
}

/// What the status bar and the `telemetry{state, endpoint}` log line say (architecture §10).
/// `status` keeps the `{enabled, reason}` of `telemetry_status`; `state` is `off (config)`,
/// `off, no keychain item <name>` or `exporting since <timestamp>` (D-046).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TelemetryReport {
    #[serde(flatten)]
    pub status: TelemetryStatus,
    pub state: String,
    pub endpoint: String,
}

impl TelemetryReport {
    fn off(reason: &str, state: String, endpoint: &str) -> Self {
        Self {
            status: TelemetryStatus::off(reason),
            state,
            endpoint: endpoint.to_owned(),
        }
    }

    pub fn exporting(endpoint: &str, since: &str) -> Self {
        Self {
            status: TelemetryStatus {
                enabled: true,
                reason: "exporting".into(),
            },
            state: format!("exporting since {since}"),
            endpoint: endpoint.to_owned(),
        }
    }

    /// The status bar text.
    pub fn bar_text(&self) -> String {
        format!("telemetry: {}", self.state)
    }
}

/// Everything the exporter needs to start: the plan it registers, the collector and the optional
/// header.
#[derive(Clone, Debug)]
pub struct ExporterStart {
    pub plan: SeriesPlan,
    pub endpoint: String,
    pub header: Option<OtlpHeader>,
}

impl ExporterStart {
    /// The report once the exporter is running since `since`.
    pub fn report(&self, since: &str) -> TelemetryReport {
        TelemetryReport::exporting(&self.endpoint, since)
    }
}

/// Decides whether the exporter starts. `lookup` reads the keychain item of a service name and is
/// called only when `[telemetry].headers_keychain_item` is non-empty and the plan is accepted: with
/// no item the exporter starts without a header and the keychain is never touched (D-046). A
/// configured item that is absent, refused or not `name=value` keeps export off with a state that
/// says so; the environment is never consulted. Nothing here carries a header value into a status.
pub fn resolve_start(
    cfg: &TelemetryConfig,
    lookup: impl FnOnce(&str) -> Result<Option<String>, String>,
) -> Result<ExporterStart, TelemetryReport> {
    let endpoint = cfg.endpoint.as_str();
    if !cfg.enabled {
        return Err(TelemetryReport::off(
            "off in config",
            "off (config)".into(),
            endpoint,
        ));
    }
    let plan = start_check(cfg).map_err(|status| TelemetryReport {
        state: format!("off, {}", status.reason),
        status,
        endpoint: endpoint.to_owned(),
    })?;
    let item = cfg.headers_keychain_item.trim();
    let header = if item.is_empty() {
        None
    } else {
        let off =
            |reason: &str| TelemetryReport::off(reason, format!("off, {reason} {item}"), endpoint);
        match lookup(item) {
            Ok(Some(text)) => match parse_header(&text) {
                Some((name, value)) => Some(OtlpHeader {
                    name: name.to_owned(),
                    value: value.to_owned(),
                }),
                None => return Err(off("header item is not name=value")),
            },
            Ok(None) => return Err(off("no keychain item")),
            Err(_) => return Err(off("keychain refused")),
        }
    };
    Ok(ExporterStart {
        plan,
        endpoint: cfg.endpoint.clone(),
        header,
    })
}

/// The labels of a point: a label and its bounded value, in the order the instrument plans them.
pub type Labels = Vec<(Label, String)>;

/// One value of one series: a counter's running total, or one histogram observation.
#[derive(Clone, Debug, PartialEq)]
pub struct Point {
    pub name: &'static str,
    /// In the order the instrument plans them.
    pub labels: Labels,
    pub value: f64,
    /// What makes an observation the same one on the next sample (the feature of a feature cycle,
    /// the mission of a mission cycle); empty for a counter.
    pub key: String,
}

/// What one mission contributes to the eleven instruments at `end`. Counters are running totals
/// since the mission began, so the exporter adds the difference to its last sample; observations
/// are recorded once each.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MissionSample {
    pub counters: Vec<Point>,
    pub observations: Vec<Point>,
}

pub struct SampleInput<'a> {
    /// The `mission` label value.
    pub mission: &'a str,
    pub events: &'a [Event],
    pub template: &'a CycleTemplate,
    pub notes: &'a Notes,
    pub prices: &'a PriceTable,
    pub end: Timestamp,
    pub cfg: &'a TelemetryConfig,
}

/// The `milestone` value of work no milestone owns; it takes one of the named slots like any id.
const NO_MILESTONE: &str = "none";

/// The `model` value of a turn whose model is unknown.
const UNKNOWN_MODEL: &str = "unknown";

fn waste_label(cause: WasteCause) -> &'static str {
    match cause {
        WasteCause::WaitingHumanQuestion => "waiting_human_question",
        WasteCause::WaitingHumanPermission => "waiting_human_permission",
        WasteCause::WaitingMerge => "waiting_merge",
        WasteCause::WaitingValidation => "waiting_validation",
        WasteCause::Rework => "rework",
        WasteCause::RetryFailure => "retry_failure",
        WasteCause::IdleQueued => "idle_queued",
        WasteCause::IdleUnscheduled => "idle_unscheduled",
    }
}

fn rework_label(kind: ReworkKind) -> &'static str {
    match kind {
        ReworkKind::Finding => "finding",
        ReworkKind::Scenario => "scenario",
        ReworkKind::Conflict => "conflict",
        ReworkKind::Check => "check",
    }
}

/// The values of the eleven instruments for one mission, every label value bounded (`overflow`,
/// `other`) and every label one of the five. Pure over the mapped events.
pub fn mission_sample(input: &SampleInput<'_>) -> MissionSample {
    let SampleInput {
        mission,
        events,
        template,
        notes,
        prices,
        end,
        cfg,
    } = *input;
    let mut milestones = BoundedLabel::milestone(cfg);
    // Ids are assigned in event order so a value keeps its label as the mission grows.
    for event in events.iter().filter(|event| !event.kind.is_panel_only()) {
        if let Some(id) = &event.milestone_id {
            milestones.value(&id.0);
        }
    }
    let mut models = BoundedLabel::model();
    let mission_label = || (Label::Mission, mission.to_owned());
    let mut totals: BTreeMap<(&'static str, Labels), f64> = BTreeMap::new();
    let mut add = |name: &'static str, labels: Labels, value: f64| {
        *totals.entry((name, labels)).or_default() += value;
    };

    let stops = throttled(events, notes, end);
    let stop_spans = stops.spans();
    let cards = card_times(events, template, notes, end, &stop_spans);

    let milestone_of = |feature: &Option<FeatureId>| -> Option<String> {
        events
            .iter()
            .filter(|event| event.feature_id == *feature)
            .find_map(|event| event.milestone_id.as_ref())
            .map(|id| id.0.clone())
    };
    let mut milestone_labels: BTreeMap<Option<FeatureId>, String> = BTreeMap::new();
    for key in cards.keys() {
        let id = key
            .as_ref()
            .and_then(|_| milestone_of(key))
            .unwrap_or_else(|| NO_MILESTONE.to_owned());
        milestone_labels.insert(key.clone(), milestones.value(&id).to_owned());
    }

    for (key, card) in &cards {
        let milestone = milestone_labels[key].clone();
        for cause in WasteCause::ALL {
            let seconds = card.waste.seconds(cause);
            if seconds > 0 {
                add(
                    "observatory.waste_seconds",
                    vec![
                        mission_label(),
                        (Label::Milestone, milestone.clone()),
                        (Label::Cause, waste_label(cause).to_owned()),
                    ],
                    seconds as f64,
                );
            }
        }
    }

    let windows = rework_windows(events, template, notes, end);
    for window in &windows {
        let milestone = match (&window.feature, &window.milestone) {
            (Some(_), _) => milestone_labels.get(&window.feature).cloned(),
            (None, Some(id)) => Some(milestones.value(&id.0).to_owned()),
            (None, None) => milestone_labels.get(&None).cloned(),
        }
        .unwrap_or_else(|| milestones.value(NO_MILESTONE).to_owned());
        add(
            "observatory.rework_windows",
            vec![
                mission_label(),
                (Label::Milestone, milestone.clone()),
                (Label::Cause, rework_label(window.kind).to_owned()),
            ],
            1.0,
        );
        if window.kind == ReworkKind::Check {
            add(
                "observatory.fix_loops",
                vec![mission_label(), (Label::Milestone, milestone)],
                1.0,
            );
        }
    }

    for turn in turns(events) {
        let Some(role) = role_label(turn.event.role) else {
            continue;
        };
        let model_id = turn.model.map_or(UNKNOWN_MODEL, |model| model.0.as_str());
        let model = models.value(model_id).to_owned();
        let labels = vec![
            mission_label(),
            (Label::Role, role.to_owned()),
            (Label::Model, model),
        ];
        let usd = prices
            .turn_usd(
                turn.event.role,
                turn.model.map(|m| m.0.as_str()),
                &turn.tokens,
            )
            .unwrap_or(0.0);
        add(
            "observatory.tokens.input",
            labels.clone(),
            turn.tokens.input as f64,
        );
        add(
            "observatory.tokens.output",
            labels.clone(),
            turn.tokens.output as f64,
        );
        add(
            "observatory.tokens.cache_read",
            labels.clone(),
            turn.tokens.cache_read as f64,
        );
        add("observatory.cost_usd", labels, usd);
    }

    let unknown = events
        .iter()
        .filter(|event| matches!(event.kind, EventKind::Unknown { .. }))
        .count();
    if unknown > 0 {
        add(
            "observatory.unknown_events",
            vec![mission_label()],
            unknown as f64,
        );
    }

    let counters = totals
        .into_iter()
        .map(|((name, labels), value)| Point {
            name,
            labels,
            value,
            key: String::new(),
        })
        .collect();

    let mut observations = Vec::new();
    let real = || events.iter().filter(|event| !event.kind.is_panel_only());
    let seconds = |from: Timestamp, to: Timestamp| (to.0 - from.0).as_seconds_f64().max(0.0);
    if let (Some(first), Some(done)) = (
        real().map(|event| event.at).min(),
        real()
            .filter(|event| matches!(event.kind, EventKind::HandedToCd { .. }))
            .map(|event| event.at)
            .max(),
    ) {
        observations.push(Point {
            name: "observatory.mission.cycle_seconds",
            labels: vec![mission_label()],
            value: seconds(first, done),
            key: mission.to_owned(),
        });
    }
    for feature in completed_features(events, &cards) {
        let own = || real().filter(|event| event.feature_id.as_ref() == Some(&feature));
        if let (Some(first), Some(last)) = (
            own().map(|event| event.at).min(),
            own().map(|event| event.at).max(),
        ) {
            observations.push(Point {
                name: "observatory.feature.cycle_seconds",
                labels: vec![mission_label()],
                value: seconds(first, last),
                key: feature.0.clone(),
            });
        }
    }
    MissionSample {
        counters,
        observations,
    }
}
