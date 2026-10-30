//! Telemetry series plan (architecture §10, SPEC §5.7, D-010): the eleven exported instruments,
//! the label allow-list, the bounds on label values and the worst-case series count the exporter
//! checks before it starts. Pure: the exporter that sends the series lives in the app.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::config::TelemetryConfig;
use crate::metrics::{ReworkKind, WasteCause};
use crate::model::Role;

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
