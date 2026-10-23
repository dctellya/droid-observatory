use crate::model::Lane;

/// One lane column as the UI draws it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LaneView {
    pub lane: Lane,
    pub name: &'static str,
    /// Stable accessible name, `lane-<n>`, which validators address the column by.
    pub aria_label: String,
    pub card_count: usize,
}

/// The board: the nine lanes in PRD §7 order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoardView {
    pub lanes: Vec<LaneView>,
}

impl BoardView {
    pub fn empty() -> Self {
        let lanes = Lane::ALL
            .iter()
            .map(|&lane| LaneView {
                lane,
                name: lane.name(),
                aria_label: format!("lane-{}", lane.number()),
                card_count: 0,
            })
            .collect();
        Self { lanes }
    }

    pub fn card_count(&self) -> usize {
        self.lanes.iter().map(|lane| lane.card_count).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_board_has_nine_lanes_and_no_cards() {
        let board = BoardView::empty();

        assert_eq!(board.lanes.len(), 9);
        assert_eq!(board.card_count(), 0);
        let names: Vec<&str> = board.lanes.iter().map(|l| l.name).collect();
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
        let labels: Vec<&str> = board.lanes.iter().map(|l| l.aria_label.as_str()).collect();
        assert_eq!(
            labels,
            [
                "lane-1", "lane-2", "lane-3", "lane-4", "lane-5", "lane-6", "lane-7", "lane-8",
                "lane-9"
            ]
        );
    }
}
