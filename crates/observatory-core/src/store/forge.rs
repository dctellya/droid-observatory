//! What a poll of the forge records besides its events: the pull request and its checks.

use rusqlite::params;

use super::{Store, StoreError, insert_events_in};
use crate::ingest::forge::Poll;
use crate::model::MissionId;

impl Store {
    /// Records one poll: the pull request row (its `observed_at` stays the first observation,
    /// the poll that detected it), one row per check, and the poll's new events. Safe to repeat.
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
        tx.commit()?;
        Ok(())
    }
}
