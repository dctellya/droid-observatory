//! Forward-only numbered migrations; `schema_version` holds the highest one applied.

use rusqlite::{Connection, params};

use super::now_text;
use crate::mapping::MappingVersion;

const MIGRATIONS: &[(i64, &str)] = &[
    (1, include_str!("migrations/0001_init.sql")),
    (2, include_str!("migrations/0002_lane_text.sql")),
    (3, include_str!("migrations/0003_gate_windows.sql")),
];

pub(super) fn migrate(conn: &mut Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL)",
    )?;
    let current: i64 = conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_version",
        [],
        |row| row.get(0),
    )?;
    // A table rebuild (0002) drops a table other tables point at; SQLite only honours this
    // pragma outside a transaction, and `foreign_key_check` proves the result before it is back on.
    conn.pragma_update(None, "foreign_keys", false)?;
    let applied = apply(conn, current);
    conn.pragma_update(None, "foreign_keys", true)?;
    applied
}

fn apply(conn: &mut Connection, current: i64) -> rusqlite::Result<()> {
    for (version, sql) in MIGRATIONS.iter().filter(|(version, _)| *version > current) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        let violations: i64 =
            tx.query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
                row.get(0)
            })?;
        if violations > 0 {
            return Err(rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CONSTRAINT_FOREIGNKEY),
                Some(format!(
                    "migration {version} leaves {violations} foreign key violations"
                )),
            ));
        }
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
