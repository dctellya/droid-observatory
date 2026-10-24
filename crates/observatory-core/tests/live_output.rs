//! The live output of a hosted agent (FR-A3): the latest lines of its spool, masked at render
//! time, with a count of every line the spool holds.

use observatory_core::view::{LiveOutput, OutputLines};

fn raw(lines: &[&str]) -> Vec<Vec<u8>> {
    lines.iter().map(|line| line.as_bytes().to_vec()).collect()
}

#[test]
fn live_output_counts_every_line_and_keeps_the_latest() {
    let mut output = LiveOutput::new(3);

    output.push(OutputLines::of("l1", 0, &raw(&["a", "b"])));
    output.push(OutputLines::of("l1", 2, &raw(&["c", "d", "e"])));

    assert_eq!(output.total(), 5);
    let shown: Vec<&str> = output
        .lines()
        .iter()
        .map(|line| line.text.as_str())
        .collect();
    assert_eq!(shown, ["c", "d", "e"]);
}

#[test]
fn live_output_masks_token_shapes_and_leaves_the_raw_line_alone() {
    let secret = format!("token ghp_{}", "a".repeat(36));
    let lines = vec![secret.clone().into_bytes()];

    let batch = OutputLines::of("l1", 0, &lines);

    assert_eq!(batch.lines[0].masked, 1);
    assert!(!batch.lines[0].text.contains("ghp_aaaa"));
    assert!(String::from_utf8_lossy(&lines[0]).contains("ghp_aaaa"));
}

#[test]
fn live_output_of_another_launch_replaces_the_held_lines() {
    let mut output = LiveOutput::new(3);
    output.push(OutputLines::of("l1", 0, &raw(&["a"])));

    output.push(OutputLines::of("l2", 0, &raw(&["x", "y"])));

    assert_eq!(output.launch_id(), Some("l2"));
    assert_eq!(output.total(), 2);
    assert_eq!(output.lines().len(), 2);
}
