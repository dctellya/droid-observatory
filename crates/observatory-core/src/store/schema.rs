//! Forward-only numbered migrations; `schema_version` holds the highest one applied.

use rusqlite::{Connection, params};

use super::now_text;
use crate::mapping::MappingVersion;

const MIGRATIONS: &[(i64, &str)] = &[(1, include_str!("migrations/0001_init.sql"))];

pub(super) fn migrate(conn: &mut Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL)",
    )?;
    let current: i64 = conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_version",
        [],
        |row| row.get(0),
    )?;
    for (version, sql) in MIGRATIONS.iter().filter(|(version, _)| *version > current) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.execute(
            "INSERT INTO schema_version (version, applied_at) VALUES (?1, ?2)",
            params![version, now_text()],
        )?;
        tx.commit()?;
    }
    Ok(())
}

/// Every mapping version the code knows has a row, so an event's `mapping_version` always
/// has something to point at.
pub(super) fn seed_mapping_versions(conn: &Connection) -> rusqlite::Result<()> {
    for version in MappingVersion::ALL {
        conn.execute(
            "INSERT OR IGNORE INTO mapping_versions (version, droid_version_range, frozen_at) VALUES (?1, ?2, ?3)",
            params![
                version.to_string(),
                version.droid_version_range(),
                version.frozen_at()
            ],
        )?;
    }
    Ok(())
}
