use serde::{Deserialize, Serialize};

use super::role::Role;

/// The nine board lanes in PRD §7 order.
///
/// `TimeWasteCost` is computed from the other lanes and never carries events.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Lane {
    Intent,
    Orchestrator,
    Workers,
    Scrutiny,
    UserTesting,
    Integration,
    Human,
    Outside,
    TimeWasteCost,
}

impl Lane {
    /// Board order, left to right.
    pub const ALL: [Lane; 9] = [
        Lane::Intent,
        Lane::Orchestrator,
        Lane::Workers,
        Lane::Scrutiny,
        Lane::UserTesting,
        Lane::Integration,
        Lane::Human,
        Lane::Outside,
        Lane::TimeWasteCost,
    ];

    /// 1-based lane number as in the PRD table.
    pub fn number(self) -> u8 {
        match self {
            Lane::Intent => 1,
            Lane::Orchestrator => 2,
            Lane::Workers => 3,
            Lane::Scrutiny => 4,
            Lane::UserTesting => 5,
            Lane::Integration => 6,
            Lane::Human => 7,
            Lane::Outside => 8,
            Lane::TimeWasteCost => 9,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Lane::Intent => "Intent",
            Lane::Orchestrator => "Orchestrator",
            Lane::Workers => "Workers",
            Lane::Scrutiny => "Validators, scrutiny",
            Lane::UserTesting => "Validators, user testing",
            Lane::Integration => "Integration",
            Lane::Human => "Human",
            Lane::Outside => "Outside",
            Lane::TimeWasteCost => "Time, Waste, Cost and models",
        }
    }

    /// Lane of an event whose kind does not fix one (an `Unknown` event): the role decides.
    /// An unresolved role lands in Workers, where child sessions live.
    pub fn for_role(role: Role) -> Lane {
        match role {
            Role::Orchestrator => Lane::Orchestrator,
            Role::Worker | Role::Unresolved => Lane::Workers,
            Role::ScrutinyValidator => Lane::Scrutiny,
            Role::UserTestingValidator => Lane::UserTesting,
            Role::Human => Lane::Human,
            Role::External => Lane::Outside,
        }
    }
}
