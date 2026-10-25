//! The cycle template (F-041, D-035): the lanes, their order, the role that owns each, the final
//! event and the clocks are data read at start-up, never names in code. The engineering cycle of
//! D-024 is the default, compiled in from `config/cycles/engineering.toml`; `[cycle].template`
//! selects another file. Views, metrics, the CLI and the UI iterate [`CycleTemplate::lanes`].

use std::collections::BTreeSet;
use std::fmt;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use super::kind::EventKind;
use super::role::Role;

/// The engineering cycle: seven lanes from the plan to the gate.
const ENGINEERING: &str = include_str!("../../../../config/cycles/engineering.toml");

/// The wire id of a lane (`plan`, `user_testing`): the text events carry and the store keeps.
/// An id the loaded template does not list is still a valid id and is shown as itself.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LaneId(pub String);

impl LaneId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for LaneId {
    fn from(id: &str) -> Self {
        Self(id.to_owned())
    }
}

impl From<String> for LaneId {
    fn from(id: String) -> Self {
        Self(id)
    }
}

impl fmt::Display for LaneId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// How the rows of a lane split under headings.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LaneGrouping {
    #[default]
    None,
    /// A sub-lane per worker session or per feature, as the filter says.
    Sessions,
    /// One section per milestone.
    Milestones,
}

/// One lane of the cycle.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LaneSpec {
    pub id: LaneId,
    /// The display name on the board header.
    pub name: String,
    /// The role that owns the lane (attribution and colour); none for a lane no role owns.
    #[serde(default)]
    pub owner_role: Option<Role>,
    /// Roles whose role-neutral events land here.
    #[serde(default)]
    pub home_for: Vec<Role>,
    /// PRD names of the kinds fixed to this lane whoever emits them.
    #[serde(default)]
    pub kinds: Vec<String>,
    #[serde(default)]
    pub grouping: LaneGrouping,
    /// Ids of the template clocks shown in the lane header.
    #[serde(default)]
    pub clocks: Vec<String>,
}

/// A duration the cycle names. A clock with neither `start` nor `end` is label only.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Clock {
    pub id: String,
    pub label: String,
    /// PRD name of the kind that starts the clock.
    #[serde(default)]
    pub start: Option<String>,
    /// PRD name of the kind that starts it for a feature with no `start` event.
    #[serde(default)]
    pub start_fallback: Option<String>,
    /// PRD name of the kind that stops the clock.
    #[serde(default)]
    pub end: Option<String>,
}

/// The session facts that move an event off its kind's lane (D-024 rules 2 to 4, D-032).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionRules {
    /// The lane of the role-neutral events of an integration session.
    pub integration_session: LaneId,
    /// The lane of the role-neutral events of a session that writes the validation contract.
    pub contract_session: LaneId,
    /// The lane of the orchestrator's `commit.created`, which lands on the mission branch.
    pub orchestrator_commit: LaneId,
    /// Kinds that take the lane of the role that emits them instead of their own.
    #[serde(default)]
    pub kind_follows_emitter: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CycleTemplate {
    pub name: String,
    /// Board order, left to right; the number of a lane is its position plus one.
    pub lanes: Vec<LaneSpec>,
    /// PRD name of the cycle's last event; it sits in exactly one lane, the final lane.
    pub final_event: String,
    #[serde(default)]
    pub clocks: Vec<Clock>,
    pub rules: SessionRules,
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum CycleError {
    #[error("cycle template does not parse: {0}")]
    Parse(String),
    #[error("cycle template has no lanes")]
    NoLanes,
    #[error("lane id {0:?} is used twice")]
    DuplicateLane(String),
    #[error("clock id {0:?} is used twice")]
    DuplicateClock(String),
    #[error("kind {0:?} is not an event kind")]
    UnknownKind(String),
    #[error("kind {kind:?} is listed in lanes {first:?} and {second:?}")]
    KindInTwoLanes {
        kind: String,
        first: String,
        second: String,
    },
    #[error("role {0:?} is the home of no lane")]
    RoleWithoutHome(String),
    #[error("role {0:?} is the home of two lanes")]
    RoleWithTwoHomes(String),
    #[error("final event {0:?} is in no lane")]
    FinalEventInNoLane(String),
    #[error("{what} names lane {id:?}, which the template does not have")]
    UnknownLane { what: &'static str, id: String },
    #[error("lane {lane:?} shows clock {clock:?}, which the template does not have")]
    UnknownClock { lane: String, clock: String },
}

impl CycleTemplate {
    /// The engineering cycle of D-024, compiled in. Infallible: a test parses the same text.
    pub fn engineering() -> Self {
        static PARSED: OnceLock<CycleTemplate> = OnceLock::new();
        PARSED
            .get_or_init(|| {
                Self::from_toml(ENGINEERING).expect("the compiled-in engineering template is valid")
            })
            .clone()
    }

    /// Parses and validates a template.
    pub fn from_toml(text: &str) -> Result<Self, CycleError> {
        let template: Self =
            toml::from_str(text).map_err(|error| CycleError::Parse(error.to_string()))?;
        template.validate()?;
        Ok(template)
    }

    fn validate(&self) -> Result<(), CycleError> {
        if self.lanes.is_empty() {
            return Err(CycleError::NoLanes);
        }
        let known: BTreeSet<String> = EventKind::KNOWN
            .iter()
            .map(|kind| kind.prd_name().into_owned())
            .collect();
        let check_kind = |name: &str| {
            known
                .contains(name)
                .then_some(())
                .ok_or_else(|| CycleError::UnknownKind(name.to_owned()))
        };

        let mut ids = BTreeSet::new();
        let mut placed: Vec<(&str, &str)> = Vec::new();
        for lane in &self.lanes {
            if !ids.insert(lane.id.as_str()) {
                return Err(CycleError::DuplicateLane(lane.id.to_string()));
            }
            for kind in &lane.kinds {
                check_kind(kind)?;
                if let Some((_, first)) = placed.iter().find(|(seen, _)| seen == kind) {
                    return Err(CycleError::KindInTwoLanes {
                        kind: kind.clone(),
                        first: (*first).to_owned(),
                        second: lane.id.to_string(),
                    });
                }
                placed.push((kind, lane.id.as_str()));
            }
        }

        let mut clock_ids = BTreeSet::new();
        for clock in &self.clocks {
            if !clock_ids.insert(clock.id.as_str()) {
                return Err(CycleError::DuplicateClock(clock.id.clone()));
            }
            for kind in [&clock.start, &clock.start_fallback, &clock.end]
                .into_iter()
                .flatten()
            {
                check_kind(kind)?;
            }
        }
        for lane in &self.lanes {
            if let Some(clock) = lane.clocks.iter().find(|c| !clock_ids.contains(c.as_str())) {
                return Err(CycleError::UnknownClock {
                    lane: lane.id.to_string(),
                    clock: clock.clone(),
                });
            }
        }

        for role in Role::ALL {
            let homes = self
                .lanes
                .iter()
                .filter(|lane| lane.home_for.contains(&role))
                .count();
            match homes {
                0 => return Err(CycleError::RoleWithoutHome(role.as_str().to_owned())),
                1 => {}
                _ => return Err(CycleError::RoleWithTwoHomes(role.as_str().to_owned())),
            }
        }

        check_kind(&self.final_event)?;
        if !placed.iter().any(|(kind, _)| *kind == self.final_event) {
            return Err(CycleError::FinalEventInNoLane(self.final_event.clone()));
        }
        for (what, id) in [
            ("rules.integration_session", &self.rules.integration_session),
            ("rules.contract_session", &self.rules.contract_session),
            ("rules.orchestrator_commit", &self.rules.orchestrator_commit),
        ] {
            if !ids.contains(id.as_str()) {
                return Err(CycleError::UnknownLane {
                    what,
                    id: id.to_string(),
                });
            }
        }
        self.rules
            .kind_follows_emitter
            .iter()
            .try_for_each(|kind| check_kind(kind))
    }

    pub fn lane(&self, id: &LaneId) -> Option<&LaneSpec> {
        self.lanes.iter().find(|lane| &lane.id == id)
    }

    /// 0-based position of a lane; its board number is this plus one.
    pub fn index(&self, id: &LaneId) -> Option<usize> {
        self.lanes.iter().position(|lane| &lane.id == id)
    }

    /// The lane an empty milestone waits in: the first one.
    pub fn first(&self) -> &LaneId {
        &self.lanes[0].id
    }

    /// The lane that holds the final event (the header and placeholder owner).
    pub fn final_lane(&self) -> &LaneId {
        self.lanes
            .iter()
            .find(|lane| lane.kinds.contains(&self.final_event))
            .map_or_else(|| &self.lanes[self.lanes.len() - 1].id, |lane| &lane.id)
    }

    /// The lane a kind always has, whoever emits it; none for the role-neutral kinds.
    pub fn fixed_lane(&self, kind: &EventKind) -> Option<&LaneId> {
        let name = kind.prd_name();
        self.lanes
            .iter()
            .find(|lane| lane.kinds.iter().any(|listed| *listed == name))
            .map(|lane| &lane.id)
    }

    /// The lane a role works in when nothing else fixes the event's lane.
    pub fn home(&self, role: Role) -> &LaneId {
        self.lanes
            .iter()
            .find(|lane| lane.home_for.contains(&role))
            .map_or_else(|| self.first(), |lane| &lane.id)
    }

    /// The lane of an event of this kind by `role` (D-024 rules 1, 2 and 5): the kind's fixed
    /// lane; the orchestrator's commits, which land on the mission branch; otherwise the
    /// role's home. Session facts are applied by the mapping on top of this.
    pub fn lane_of(&self, kind: &EventKind, role: Role) -> LaneId {
        if matches!((kind, role), (EventKind::CommitCreated, Role::Orchestrator)) {
            return self.rules.orchestrator_commit.clone();
        }
        self.fixed_lane(kind)
            .unwrap_or_else(|| self.home(role))
            .clone()
    }

    /// Whether `kind` takes the lane of the role that emits it (D-032).
    pub fn follows_emitter(&self, kind: &EventKind) -> bool {
        let name = kind.prd_name();
        self.rules
            .kind_follows_emitter
            .iter()
            .any(|listed| *listed == name)
    }

    /// `<position> <name>` as the CLI prints a lane; an id the template does not have is
    /// printed as itself.
    pub fn lane_text(&self, id: &LaneId) -> String {
        match self.index(id) {
            Some(index) => format!("{} {}", index + 1, self.lanes[index].name),
            None => id.to_string(),
        }
    }

    /// The display name of a lane; an id the template does not have is its own name.
    pub fn name_of(&self, id: &LaneId) -> String {
        self.lane(id)
            .map_or_else(|| id.to_string(), |lane| lane.name.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_compiled_in_template_is_valid_and_has_the_wire_ids() {
        let template = CycleTemplate::engineering();
        let ids: Vec<&str> = template.lanes.iter().map(|l| l.id.as_str()).collect();
        assert_eq!(ids.len(), 7);
        assert_eq!(template.index(&LaneId::from(ids[6])), Some(6));
    }

    #[test]
    fn an_unknown_lane_id_prints_as_itself() {
        let template = CycleTemplate::engineering();
        let first = template.first().clone();
        assert_eq!(template.lane_text(&first), "1 Plan");
        assert_eq!(template.lane_text(&LaneId::from("retired")), "retired");
        assert_eq!(template.name_of(&LaneId::from("retired")), "retired");
    }

    #[test]
    fn every_known_kind_has_at_most_one_lane() {
        let template = CycleTemplate::engineering();
        for kind in EventKind::KNOWN {
            let lanes = template
                .lanes
                .iter()
                .filter(|lane| lane.kinds.iter().any(|k| *k == kind.prd_name()))
                .count();
            assert!(lanes <= 1, "{}", kind.prd_name());
        }
    }
}
