//! The `board` command answers with a digest of the board the store holds: the figures a
//! replay of the same capture must reproduce (lane events, cards per lane, the tray).

use std::path::Path;

use observatory_core::config::MissionConfig;
use observatory_core::model::{CycleTemplate, Event};
use observatory_core::view::{
    BoardDigest, BoardView, DeclaredModels, ViewConfig, branch_of, lane_counts, tray_count,
};

fn engineering() -> CycleTemplate {
    CycleTemplate::engineering()
}

fn golden() -> Vec<Event> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/streams/2026-10-22-observatory-v1-m0.events.json");
    let text = std::fs::read_to_string(path).expect("the golden file is committed");
    serde_json::from_str(&text).expect("the golden file is a list of events")
}

fn board_of(events: &[Event]) -> BoardView {
    let mission = MissionConfig::default();
    BoardView::build(
        events,
        &ViewConfig::default(),
        &DeclaredModels::from_config(&mission),
        &branch_of(&mission, "m0"),
        &engineering(),
    )
}

#[test]
fn board_digest_reports_lane_events_card_lanes_and_the_tray() {
    let events = golden();
    let board = board_of(&events);

    let digest = board.digest();

    let on_board = events
        .iter()
        .filter(|event| !event.kind.is_panel_only())
        .count() as u64;
    assert_eq!(digest.event_count, on_board, "turn.usage is Panel data");
    assert_eq!(digest.event_count, 1279);
    assert_eq!(digest.tray_unknown, tray_count(&events));
    assert_eq!(
        digest.lane_events.to_vec(),
        lane_counts(&events, &engineering()).to_vec()
    );
    assert_eq!(digest.lane_events.len(), engineering().lanes.len());
    assert_eq!(
        digest.lane_cards.iter().sum::<usize>(),
        board.card_count(),
        "every card is counted in one lane"
    );
    assert_eq!(digest.cards.len(), board.card_count());
    for card in &digest.cards {
        let lane = engineering()
            .index(&card.lane)
            .expect("a card sits in one of the seven lanes");
        assert!(digest.lane_cards[lane] > 0);
    }
}

#[test]
fn board_digest_survives_a_json_round_trip() {
    let digest = board_of(&golden()).digest();
    let text = serde_json::to_string(&digest).expect("serialises");
    let back: BoardDigest = serde_json::from_str(&text).expect("parses");
    assert_eq!(back, digest);
}
