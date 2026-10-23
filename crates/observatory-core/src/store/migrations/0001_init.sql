-- Pointer index of events (architecture §6). Raw line text, tool output, prompts and the
-- systemPrompt have no column here: the store keeps path, byte_offset and byte_len only.
CREATE TABLE captures (id INTEGER PRIMARY KEY, source_dir TEXT NOT NULL, droid_version TEXT,
  header_path TEXT, started_at TEXT NOT NULL);
CREATE TABLE mapping_versions (version TEXT PRIMARY KEY, droid_version_range TEXT NOT NULL, frozen_at TEXT NOT NULL);
CREATE TABLE missions (id TEXT PRIMARY KEY, name TEXT NOT NULL, capture_id INTEGER REFERENCES captures(id),
  branch TEXT, state TEXT NOT NULL CHECK (state IN ('planning','building','validating','at_gate','fixing','complete')),
  started_at TEXT NOT NULL, completed_at TEXT, price_table TEXT);
CREATE TABLE sessions (id TEXT PRIMARY KEY, path TEXT NOT NULL UNIQUE, mission_id TEXT NOT NULL REFERENCES missions(id),
  parent_id TEXT REFERENCES sessions(id), role TEXT NOT NULL, tag TEXT, model TEXT, droid_version TEXT,
  started_at TEXT, ended_at TEXT, size_bytes INTEGER NOT NULL DEFAULT 0, tailed_to INTEGER NOT NULL DEFAULT 0);
CREATE TABLE milestones (id TEXT PRIMARY KEY, mission_id TEXT NOT NULL REFERENCES missions(id), name TEXT NOT NULL,
  defined_at TEXT, completed_at TEXT, state TEXT NOT NULL);
CREATE TABLE features (id TEXT PRIMARY KEY, milestone_id TEXT NOT NULL REFERENCES milestones(id), name TEXT NOT NULL,
  defined_at TEXT, completed_at TEXT, state TEXT NOT NULL);
CREATE TABLE events (id TEXT PRIMARY KEY, mission_id TEXT NOT NULL REFERENCES missions(id), session_id TEXT REFERENCES sessions(id),
  milestone_id TEXT, feature_id TEXT, lane INTEGER NOT NULL CHECK (lane BETWEEN 1 AND 7),
  overlay TEXT CHECK (overlay IN ('human','commander')), kind TEXT NOT NULL,
  raw_type TEXT, mapping_version TEXT NOT NULL REFERENCES mapping_versions(version), role TEXT NOT NULL, model TEXT,
  at TEXT NOT NULL, duration_ms INTEGER, source TEXT NOT NULL CHECK (source IN ('jsonl','rpc','usage_log','git','github','app')),
  path TEXT, byte_offset INTEGER, byte_len INTEGER, sub INTEGER NOT NULL DEFAULT 0, repo TEXT, sha TEXT, url TEXT, action_id INTEGER);
CREATE UNIQUE INDEX events_ptr ON events(path, byte_offset, sub) WHERE path IS NOT NULL;
CREATE INDEX events_mission_at ON events(mission_id, at);
CREATE INDEX events_feature ON events(feature_id, at);
CREATE TABLE usage (event_id TEXT PRIMARY KEY REFERENCES events(id), input INTEGER NOT NULL, output INTEGER NOT NULL,
  cache_read INTEGER NOT NULL, cache_write INTEGER NOT NULL, credits REAL);
CREATE TABLE rework_windows (id INTEGER PRIMARY KEY, feature_id TEXT NOT NULL, opened_by TEXT NOT NULL REFERENCES events(id),
  closed_by TEXT REFERENCES events(id), cause TEXT NOT NULL CHECK (cause IN ('scrutiny','user_testing','integration','gate')));
CREATE TABLE prs (number INTEGER PRIMARY KEY, mission_id TEXT NOT NULL, head TEXT, base TEXT, head_sha TEXT, state TEXT NOT NULL,
  merged_by TEXT, url TEXT, observed_at TEXT NOT NULL);
CREATE TABLE checks (id INTEGER PRIMARY KEY, pr_number INTEGER REFERENCES prs(number), name TEXT NOT NULL, source TEXT NOT NULL,
  conclusion TEXT NOT NULL, url TEXT, observed_at TEXT NOT NULL);
CREATE TABLE reviews (id INTEGER PRIMARY KEY, pr_number INTEGER REFERENCES prs(number), state TEXT NOT NULL, url TEXT, observed_at TEXT NOT NULL);
CREATE TABLE attestations (id INTEGER PRIMARY KEY, pr_number INTEGER REFERENCES prs(number), subject TEXT, verified INTEGER NOT NULL,
  url TEXT, observed_at TEXT NOT NULL);
CREATE TABLE human_actions (id INTEGER PRIMARY KEY, mission_id TEXT NOT NULL, kind TEXT NOT NULL, target TEXT, at TEXT NOT NULL,
  state TEXT NOT NULL, payload_ptr TEXT);
CREATE TABLE prices (effective TEXT NOT NULL, model TEXT NOT NULL, class TEXT NOT NULL, per_m REAL NOT NULL, source TEXT,
  PRIMARY KEY (effective, model, class));
