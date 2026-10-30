use serde::{Deserialize, Serialize};

use crate::model::MissionId;

/// The mission's headline metrics (FR-L4): the figures the Panel shows, kept on its library line.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Headline {
    /// First event to last, in whole seconds.
    pub cycle_s: u64,
    /// The five groups of waste added up, in seconds.
    pub waste_s: u64,
    pub tokens_total: u64,
    pub cost: f64,
    pub currency: String,
    /// The date of the price table the cost was made with.
    pub price_table: String,
}

/// One row of the store's `missions` table with the number of events indexed for it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MissionRow {
    pub id: MissionId,
    pub name: String,
    pub state: String,
    /// RFC 3339 time the mission started (the capture header's `captured_at` for a replay).
    pub started_at: String,
    pub completed_at: Option<String>,
    pub event_count: u64,
    /// Present once the mission's events have been measured; a row written before M5 has none.
    #[serde(default)]
    pub headline: Option<Headline>,
}

/// One line of the mission library (FR-L4): name, date, event count and the headline metrics.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LibraryEntry {
    pub mission_id: MissionId,
    pub name: String,
    /// `YYYY-MM-DD` of the start.
    pub date: String,
    pub event_count: u64,
    pub headline: Option<Headline>,
}

/// The library, newest mission first.
pub fn library(rows: &[MissionRow]) -> Vec<LibraryEntry> {
    let mut entries: Vec<(&str, LibraryEntry)> = rows
        .iter()
        .map(|row| {
            let date = row.started_at.get(..10).unwrap_or(&row.started_at);
            (
                row.started_at.as_str(),
                LibraryEntry {
                    mission_id: row.id.clone(),
                    name: row.name.clone(),
                    date: date.to_owned(),
                    event_count: row.event_count,
                    headline: row.headline.clone(),
                },
            )
        })
        .collect();
    entries.sort_by(|a, b| {
        b.0.cmp(a.0)
            .then_with(|| a.1.mission_id.cmp(&b.1.mission_id))
    });
    entries.into_iter().map(|(_, entry)| entry).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(id: &str, name: &str, started_at: &str, event_count: u64) -> MissionRow {
        MissionRow {
            id: MissionId(id.to_owned()),
            name: name.to_owned(),
            state: "building".to_owned(),
            started_at: started_at.to_owned(),
            completed_at: None,
            event_count,
            headline: None,
        }
    }

    #[test]
    fn the_library_lists_name_date_and_event_count_newest_first() {
        let rows = [
            row("a", "older", "2026-10-22T14:54:47.240Z", 12),
            row("b", "newer", "2026-10-23T08:00:00.000Z", 1279),
        ];

        let entries = library(&rows);

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name, "newer");
        assert_eq!(entries[0].date, "2026-10-23");
        assert_eq!(entries[0].event_count, 1279);
        assert_eq!(entries[1].date, "2026-10-22");
        assert!(library(&[]).is_empty());
    }

    #[test]
    fn a_short_start_time_is_shown_as_it_is() {
        let entries = library(&[row("a", "x", "today", 0)]);
        assert_eq!(entries[0].date, "today");
    }
}
