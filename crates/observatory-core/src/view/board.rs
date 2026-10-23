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

/// The Panel (time, waste, cost, model drift): drawn beside the lanes, never one of them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PanelView {
    /// Stable accessible name, `panel`.
    pub aria_label: String,
}

/// The board: the seven phase lanes in order, and the Panel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoardView {
    pub lanes: Vec<LaneView>,
    pub panel: PanelView,
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
        Self {
            lanes,
            panel: PanelView {
                aria_label: "panel".to_owned(),
            },
        }
    }

    pub fn card_count(&self) -> usize {
        self.lanes.iter().map(|lane| lane.card_count).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_board_has_seven_lanes_a_panel_and_no_cards() {
        let board = BoardView::empty();

        assert_eq!(board.lanes.len(), 7);
        assert_eq!(board.card_count(), 0);
        let names: Vec<&str> = board.lanes.iter().map(|l| l.name).collect();
        assert_eq!(
            names,
            [
                "Plan",
                "Contract",
                "Build",
                "Scrutiny",
                "User testing",
                "Integrate",
                "Gate",
            ]
        );
        let labels: Vec<&str> = board.lanes.iter().map(|l| l.aria_label.as_str()).collect();
        assert_eq!(
            labels,
            [
                "lane-1", "lane-2", "lane-3", "lane-4", "lane-5", "lane-6", "lane-7"
            ]
        );
        assert_eq!(board.panel.aria_label, "panel");
    }
}
