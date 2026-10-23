use serde::{Deserialize, Serialize};

use super::role::Role;

/// The seven mission phases, in the order work moves through them (D-024).
///
/// Roles are colour and attribution, not lanes. Human and commander actions are
/// [`Overlay`](super::Overlay)s on a phase. The Panel (time, waste, cost, model drift)
/// is computed from the lanes and is not one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Lane {
    Plan,
    Contract,
    Build,
    Scrutiny,
    UserTesting,
    Integrate,
    Gate,
}

impl Lane {
    /// Board order, left to right.
    pub const ALL: [Lane; 7] = [
        Lane::Plan,
        Lane::Contract,
        Lane::Build,
        Lane::Scrutiny,
        Lane::UserTesting,
        Lane::Integrate,
        Lane::Gate,
    ];

    /// 1-based lane number: 1 Plan .. 7 Gate.
    pub fn number(self) -> u8 {
        match self {
            Lane::Plan => 1,
            Lane::Contract => 2,
            Lane::Build => 3,
            Lane::Scrutiny => 4,
            Lane::UserTesting => 5,
            Lane::Integrate => 6,
            Lane::Gate => 7,
        }
    }

    /// The lane with this number; `None` outside 1..=7.
    pub fn from_number(number: u8) -> Option<Lane> {
        Lane::ALL.into_iter().find(|lane| lane.number() == number)
    }

    pub fn name(self) -> &'static str {
        match self {
            Lane::Plan => "Plan",
            Lane::Contract => "Contract",
            Lane::Build => "Build",
            Lane::Scrutiny => "Scrutiny",
            Lane::UserTesting => "User testing",
            Lane::Integrate => "Integrate",
            Lane::Gate => "Gate",
        }
    }

    /// The phase a role works in when nothing else fixes the event's phase (also the lane of
    /// an `Unknown` event). An unresolved role lands in Build, where child sessions live.
    pub fn home(role: Role) -> Lane {
        match role {
            Role::Orchestrator | Role::Human => Lane::Plan,
            Role::Worker | Role::Unresolved => Lane::Build,
            Role::ScrutinyValidator => Lane::Scrutiny,
            Role::UserTestingValidator => Lane::UserTesting,
            Role::External => Lane::Gate,
        }
    }
}
