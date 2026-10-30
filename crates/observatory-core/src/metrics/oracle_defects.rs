
#[test]
fn a_turn_still_open_at_the_end_counts_by_its_subagent_span() {
    let events = vec![ev("f:0", EventKind::SubagentFinished, 238)
        .feature("f")
        .lasting(238)];

    let report = report(&events, 238);

    assert_eq!(row(&report, "f").first_pass_active_s, 238);
}
