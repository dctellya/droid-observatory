//! Canonical event model: lanes, roles, events and their wire form (architecture §3).

use observatory_core::model::{Lane, Role};

#[test]
fn lanes_are_nine_in_prd_order() {
    let names: Vec<&str> = Lane::ALL.iter().map(|lane| lane.name()).collect();
    assert_eq!(
        names,
        [
            "Intent",
            "Orchestrator",
            "Workers",
            "Validators, scrutiny",
            "Validators, user testing",
            "Integration",
            "Human",
            "Outside",
            "Time, Waste, Cost and models",
        ]
    );

    let numbers: Vec<u8> = Lane::ALL.iter().map(|lane| lane.number()).collect();
    assert_eq!(numbers, [1, 2, 3, 4, 5, 6, 7, 8, 9]);

    assert_eq!(Lane::for_role(Role::Orchestrator), Lane::Orchestrator);
    assert_eq!(Lane::for_role(Role::Worker), Lane::Workers);
    assert_eq!(Lane::for_role(Role::ScrutinyValidator), Lane::Scrutiny);
    assert_eq!(
        Lane::for_role(Role::UserTestingValidator),
        Lane::UserTesting
    );
    assert_eq!(Lane::for_role(Role::Human), Lane::Human);
    assert_eq!(Lane::for_role(Role::External), Lane::Outside);
    assert_eq!(Lane::for_role(Role::Unresolved), Lane::Workers);
}
