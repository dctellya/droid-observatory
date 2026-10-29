//! What a poll of the forge records besides its events: the pull request and its checks.

use rusqlite::{OptionalExtension, params};

use super::{Store, StoreError, insert_events_in};
use crate::ingest::forge::{Poll, WindowChange};
use crate::model::{MilestoneId, MissionId};

/// The cause text of a window the forge opens (`rework_windows.cause`).
const GATE: &str = "gate";

impl Store {
    /// Records one poll: the pull request row (its `observed_at` stays the first observation,
    /// the poll that detected it), one row per check, the poll's new events and the fix-cycle
    /// windows they open and close. Safe to repeat.
    pub fn record_poll(&mut self, mission: &MissionId, poll: &Poll) -> Result<(), StoreError> {
        let observed_at = poll.observed_at.to_rfc3339_millis();
        let tx = self.conn.transaction()?;
        if let Some(pr) = &poll.pr {
            let number = i64::try_from(pr.number).unwrap_or(i64::MAX);
            tx.execute(
                "INSERT INTO prs (number, mission_id, head, base, head_sha, state, url, observed_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8) \
                 ON CONFLICT (number) DO UPDATE SET head_sha = excluded.head_sha, \
                 state = excluded.state, url = excluded.url",
                params![
                    number,
                    mission.to_string(),
                    pr.head,
                    pr.base,
                    pr.head_sha,
                    pr.state.wire(),
                    pr.url,
                    observed_at
                ],
            )?;
            for check in &poll.checks {
                let changed = tx.execute(
                    "UPDATE checks SET conclusion = ?4, url = ?5, observed_at = ?6 \
                     WHERE pr_number = ?1 AND name = ?2 AND source = ?3",
                    params![
                        number,
                        check.name,
                        check.source.wire(),
                        check.conclusion.wire(),
                        check.url,
                        observed_at
                    ],
                )?;
                if changed == 0 {
                    tx.execute(
                        "INSERT INTO checks (pr_number, name, source, conclusion, url, observed_at) \
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                        params![
                            number,
                            check.name,
                            check.source.wire(),
                            check.conclusion.wire(),
                            check.url,
                            observed_at
                        ],
                    )?;
                }
            }
        }
        insert_events_in(&tx, &poll.events)?;
        for change in &poll.windows {
            apply_window(&tx, mission, change)?;
        }
        tx.commit()?;
        Ok(())
    }

    /// The card a fix cycle returns to: the milestone integrated last, else the milestone of the
    /// latest event that names one.
    pub fn affected_milestone(
        &self,
        mission: &MissionId,
    ) -> Result<Option<MilestoneId>, StoreError> {
        let latest = |filter: &str| -> Result<Option<String>, StoreError> {
            Ok(self
                .conn
                .query_row(
                    &format!(
                        "SELECT milestone_id FROM events WHERE mission_id = ?1 \
                         AND milestone_id IS NOT NULL {filter} ORDER BY at DESC, id DESC LIMIT 1"
                    ),
                    params![mission.to_string()],
                    |row| row.get(0),
                )
                .optional()?)
        };
        let found = match latest("AND kind = 'integration.merged'")? {
            Some(found) => Some(found),
            None => latest("")?,
        };
        Ok(found.map(MilestoneId))
    }
}

fn apply_window(
    tx: &rusqlite::Transaction<'_>,
    mission: &MissionId,
    change: &WindowChange,
) -> Result<(), StoreError> {
    match change {
        WindowChange::Open {
            event,
            pr_number,
            check,
            card,
        } => {
            let number = i64::try_from(*pr_number).unwrap_or(i64::MAX);
            // A window is open for the check already (a second red before the fix) or this very
            // event opened one that is closed since (the source restarted): neither opens another.
            let known: bool = tx.query_row(
                "SELECT EXISTS (SELECT 1 FROM rework_windows WHERE cause = ?1 AND opened_by = ?2) \
                 OR EXISTS (SELECT 1 FROM rework_windows WHERE cause = ?1 AND pr_number = ?3 \
                 AND check_name = ?4 AND closed_by IS NULL)",
                params![GATE, event.to_string(), number, check],
                |row| row.get(0),
            )?;
            if !known {
                tx.execute(
                    "INSERT INTO rework_windows (feature_id, opened_by, cause, pr_number, check_name) \
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![card.to_string(), event.to_string(), GATE, number, check],
                )?;
                tx.execute(
                    "UPDATE missions SET state = 'fixing' WHERE id = ?1 AND state <> 'complete'",
                    params![mission.to_string()],
                )?;
            }
        }
        WindowChange::Close {
            event,
            pr_number,
            check,
        } => {
            let closed = tx.execute(
                "UPDATE rework_windows SET closed_by = ?1 WHERE cause = ?2 AND pr_number = ?3 \
                 AND check_name = ?4 AND closed_by IS NULL",
                params![
                    event.to_string(),
                    GATE,
                    i64::try_from(*pr_number).unwrap_or(i64::MAX),
                    check
                ],
            )?;
            if closed > 0 {
                tx.execute(
                    "UPDATE missions SET state = 'at_gate' WHERE id = ?1 AND state = 'fixing' \
                     AND NOT EXISTS (SELECT 1 FROM rework_windows w JOIN prs p ON p.number = w.pr_number \
                     WHERE p.mission_id = ?1 AND w.cause = ?2 AND w.closed_by IS NULL)",
                    params![mission.to_string(), GATE],
                )?;
            }
        }
    }
    Ok(())
}
