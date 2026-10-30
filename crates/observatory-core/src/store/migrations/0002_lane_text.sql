-- F-041, D-035: the cycle is data. The lane of an event is the id of a lane of the cycle template
-- (`plan`, `user_testing`), not a position 1..7, and the run-level overlay is spelled `officer`.
-- The ids are those of the engineering template of D-024, whose positions the old numbers were,
-- so every row of the dogfooding store converts in place. A template with other lane ids stores
-- those ids; there is no CHECK on the text because the lanes are not known to the schema.
-- SQLite cannot alter a CHECK, so `events` and `rework_windows` are rebuilt; the runner turns
-- foreign key enforcement off around the migration (the pragma has no effect inside a
-- transaction) and re-enables it after.
CREATE TABLE events_new (id TEXT PRIMARY KEY, mission_id TEXT NOT NULL REFERENCES missions(id), session_id TEXT REFERENCES sessions(id),
  milestone_id TEXT, feature_id TEXT, lane TEXT NOT NULL CHECK (lane <> ''),
  overlay TEXT CHECK (overlay IN ('human','officer')), kind TEXT NOT NULL,
  raw_type TEXT, mapping_version TEXT NOT NULL REFERENCES mapping_versions(version), role TEXT NOT NULL, model TEXT,
  at TEXT NOT NULL, duration_ms INTEGER, source TEXT NOT NULL CHECK (source IN ('jsonl','rpc','usage_log','git','github','app')),
  path TEXT, byte_offset INTEGER, byte_len INTEGER, sub INTEGER NOT NULL DEFAULT 0, repo TEXT, sha TEXT, url TEXT, action_id INTEGER);
INSERT INTO events_new (id, mission_id, session_id, milestone_id, feature_id, lane, overlay, kind, raw_type, mapping_version,
  role, model, at, duration_ms, source, path, byte_offset, byte_len, sub, repo, sha, url, action_id)
SELECT id, mission_id, session_id, milestone_id, feature_id,
  CASE lane WHEN 1 THEN 'plan' WHEN 2 THEN 'contract' WHEN 3 THEN 'build' WHEN 4 THEN 'scrutiny'
    WHEN 5 THEN 'user_testing' WHEN 6 THEN 'integrate' WHEN 7 THEN 'gate' END,
  CASE overlay WHEN 'commander' THEN 'officer' ELSE overlay END,
  kind, raw_type, mapping_version, role, model, at, duration_ms, source, path, byte_offset, byte_len, sub, repo, sha, url,
  action_id
FROM events;
CREATE TABLE rework_windows_new (id INTEGER PRIMARY KEY, feature_id TEXT NOT NULL, opened_by TEXT NOT NULL REFERENCES events(id),
  closed_by TEXT REFERENCES events(id), cause TEXT NOT NULL);
INSERT INTO rework_windows_new (id, feature_id, opened_by, closed_by, cause)
SELECT id, feature_id, opened_by, closed_by, CASE cause WHEN 'integration' THEN 'integrate' ELSE cause END
FROM rework_windows;
DROP TABLE rework_windows;
DROP TABLE events;
ALTER TABLE events_new RENAME TO events;
ALTER TABLE rework_windows_new RENAME TO rework_windows;
CREATE UNIQUE INDEX events_ptr ON events(path, byte_offset, sub) WHERE path IS NOT NULL;
CREATE INDEX events_mission_at ON events(mission_id, at);
CREATE INDEX events_feature ON events(feature_id, at);
