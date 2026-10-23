//! Checks on the committed session-stream fixtures under `fixtures/streams/`.

use std::path::{Path, PathBuf};
use std::process::Command;

use observatory_core::config::IngestConfig;
use observatory_core::ingest::{JsonlSource, RawRecord, Source, mapping_version_of};
use observatory_core::mapping::{Mapper, SessionMeta};
use observatory_core::model::{Event, EventKind, SourcePtr};
use serde_json::Value;

const S_SLUG: &str = "2026-10-22-observatory-v1-s";
const M0_SLUG: &str = "2026-10-22-observatory-v1-m0";
const EMAIL_REPLACEMENT_DOMAIN: &[u8] = b"@example.invalid";
const HOME_PATH: &[u8] = b"/Users/dct";
const HYPHENATED_HOME_PATH: &[u8] = b"-Users-dct";

fn streams_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/streams")
}

fn read_capture(slug: &str) -> Value {
    let path = streams_dir().join(format!("{slug}.capture.json"));
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("capture header {} must exist: {e}", path.display()));
    serde_json::from_str(&text).expect("capture header is valid JSON")
}

#[test]
fn s_fixture_has_capture_header_with_droid_version() {
    let header = read_capture("2026-10-22-observatory-v1-s");

    assert_eq!(header["schema"], 1);
    let version = header["droid_version"]
        .as_str()
        .expect("droid_version is a string");
    assert!(!version.is_empty(), "droid_version must not be empty");
    assert_eq!(header["mapping_version"], "v1");
    assert_eq!(header["mission_name"], "observatory-v1");
    assert!(
        header["captured_at"]
            .as_str()
            .is_some_and(|t| t.ends_with('Z'))
    );
    assert!(
        header["source_dir"]
            .as_str()
            .is_some_and(|d| d.starts_with("~/.factory/sessions/"))
    );
    assert!(
        header["gitleaks"]
            .as_str()
            .is_some_and(|g| g.starts_with("clean"))
    );
    assert!(
        header["reviewed_by"]
            .as_str()
            .is_some_and(|r| !r.is_empty())
    );

    let sessions = header["sessions"].as_array().expect("sessions is an array");
    assert_eq!(sessions[0]["role"], "orchestrator");
    for session in sessions {
        let file = session["file"].as_str().expect("session file is a string");
        let jsonl = streams_dir().join(file);
        assert!(jsonl.is_file(), "{} must exist", jsonl.display());
        assert!(
            jsonl.with_extension("settings.json").is_file(),
            "sidecar beside {} must exist",
            jsonl.display()
        );
    }
}

fn first_line_json(path: &Path) -> Value {
    let bytes = std::fs::read(path).unwrap_or_else(|e| panic!("{} reads: {e}", path.display()));
    let end = bytes
        .iter()
        .position(|b| *b == b'\n')
        .unwrap_or(bytes.len());
    serde_json::from_slice(&bytes[..end]).expect("first line is JSON")
}

#[test]
fn m0_capture_header_lists_all_four_roles() {
    let header = read_capture(M0_SLUG);

    assert_eq!(header["schema"], 1);
    assert_eq!(header["droid_version"], "0.247.0");
    assert_eq!(header["mapping_version"], "v1");
    assert_eq!(header["mission_name"], "observatory-v1");
    assert_eq!(header["captured_with"], "droid --version");
    assert_eq!(header["reviewed_by"], "factory-droid[bot]");
    assert!(
        header["gitleaks"]
            .as_str()
            .is_some_and(|g| g.starts_with("clean"))
    );

    let sessions = header["sessions"].as_array().expect("sessions is an array");
    let mut roles: Vec<&str> = sessions
        .iter()
        .map(|s| s["role"].as_str().expect("role is a string"))
        .collect();
    roles.sort_unstable();
    roles.dedup();
    assert_eq!(
        roles,
        [
            "orchestrator",
            "scrutiny_validator",
            "user_testing_validator",
            "worker"
        ]
    );

    let mut roots = Vec::new();
    for session in sessions {
        let file = session["file"].as_str().expect("session file is a string");
        let jsonl = streams_dir().join(file);
        assert!(jsonl.is_file(), "{} must exist", jsonl.display());
        assert!(
            jsonl.with_extension("settings.json").is_file(),
            "sidecar beside {} must exist",
            jsonl.display()
        );
        let first = first_line_json(&jsonl);
        assert_eq!(first["type"], "session_start");
        assert_eq!(first["id"], session["id"]);
        if first.get("callingSessionId").is_none() {
            roots.push(session["role"].as_str().expect("role is a string"));
        }
    }
    assert_eq!(
        roots,
        ["orchestrator"],
        "exactly one root, the orchestrator"
    );
}

fn files_under(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(dir).expect("directory reads") {
        let path = entry.expect("entry reads").path();
        if path.is_dir() {
            found.extend(files_under(&path));
        } else {
            found.push(path);
        }
    }
    found.sort();
    found
}

fn is_local_part(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || b"._%+-".contains(&byte)
}

fn is_domain_part(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || b".-".contains(&byte)
}

/// End (exclusive) of the longest `[A-Za-z0-9.-]+\.[A-Za-z]{2,}` prefix of `domain`.
fn domain_end(domain: &[u8]) -> Option<usize> {
    (3..=domain.len()).rev().find(|&end| {
        let head = &domain[..end];
        let letters = head
            .iter()
            .rev()
            .take_while(|b| b.is_ascii_alphabetic())
            .count();
        letters >= 2 && end > letters + 1 && head[end - letters - 1] == b'.'
    })
}

/// Every match of `[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}` in `text`.
fn email_matches(text: &[u8]) -> Vec<&[u8]> {
    let mut matches = Vec::new();
    let mut cursor = 0;
    while let Some(at) = text[cursor..]
        .iter()
        .position(|b| *b == b'@')
        .map(|i| i + cursor)
    {
        let start = text[..at]
            .iter()
            .rposition(|b| !is_local_part(*b))
            .map_or(0, |i| i + 1)
            .max(cursor);
        let run = text[at + 1..]
            .iter()
            .take_while(|b| is_domain_part(**b))
            .count();
        match domain_end(&text[at + 1..at + 1 + run]) {
            Some(end) if start < at => {
                matches.push(&text[start..at + 1 + end]);
                cursor = at + 1 + end;
            }
            _ => cursor = at + 1,
        }
    }
    matches
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

#[test]
fn fixtures_contain_no_email_or_home_path() {
    let mut offences = Vec::new();
    let mut scanned = 0;
    for path in files_under(&streams_dir()) {
        let bytes = std::fs::read(&path).expect("fixture file reads");
        scanned += 1;
        for email in email_matches(&bytes) {
            if !email.ends_with(EMAIL_REPLACEMENT_DOMAIN) {
                offences.push(format!(
                    "{}: email {}",
                    path.display(),
                    String::from_utf8_lossy(email)
                ));
            }
        }
        if contains(&bytes, HOME_PATH) {
            offences.push(format!("{}: contains /Users/dct", path.display()));
        }
        if contains(&bytes, HYPHENATED_HOME_PATH) {
            offences.push(format!("{}: contains -Users-dct", path.display()));
        }
    }

    assert!(scanned > 0, "fixtures/streams must hold files to scan");
    assert!(
        offences.is_empty(),
        "{} unredacted occurrences, first ones:\n{}",
        offences.len(),
        offences
            .iter()
            .take(10)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

fn replay(path: &Path) -> (Vec<RawRecord>, Vec<Event>) {
    let mut source = JsonlSource::open(path, &IngestConfig::default()).expect("fixture opens");
    let version = mapping_version_of(path).expect("capture header is readable");
    let mut mapper = Mapper::new(SessionMeta::root(source.root_session_id().clone()), version);
    let mut records = Vec::new();
    for _ in 0..100_000 {
        records.extend(source.next_batch().expect("batch reads"));
        if source.at_eof() {
            records.extend(source.drain_partial());
            break;
        }
    }
    let events = records.iter().flat_map(|r| mapper.map(r)).collect();
    (records, events)
}

/// Same number as `wc -l`, the one recorded in the capture header.
fn line_count(bytes: &[u8]) -> usize {
    bytes.iter().filter(|b| **b == b'\n').count()
}

#[test]
fn redaction_keeps_line_count_and_event_types() {
    let root = streams_dir().join(format!("{S_SLUG}.jsonl"));
    let bytes = std::fs::read(&root).expect("S root reads");

    let (records, events) = replay(&root);

    assert_eq!(line_count(&bytes), 2027);
    assert_eq!(records.len(), 2027);
    assert_eq!(events.len(), 2027);
    for event in &events {
        let SourcePtr::Jsonl {
            byte_offset,
            byte_len,
            ..
        } = &event.source
        else {
            panic!("replayed events point into a Jsonl file");
        };
        let start = *byte_offset as usize;
        let line: Value = serde_json::from_slice(&bytes[start..start + *byte_len as usize])
            .expect("redacted line is JSON");
        let EventKind::Unknown { raw_type } = &event.kind else {
            panic!("the S mapper yields Unknown only");
        };
        assert_eq!(line["type"].as_str(), Some(raw_type.as_str()));
    }

    let header = read_capture(S_SLUG);
    let rows = header["redaction"]["files"]
        .as_array()
        .expect("capture header records the redaction rows");
    assert!(!rows.is_empty());
    for row in rows {
        let rel = row["path"].as_str().expect("row has a path");
        let file = std::fs::read(streams_dir().join(rel)).expect("row path exists");
        assert_eq!(
            row["lines"].as_u64(),
            Some(line_count(&file) as u64),
            "line count of {rel} must equal the original's"
        );
        for key in ["original_sha256", "redacted_sha256"] {
            assert!(
                row[key]
                    .as_str()
                    .is_some_and(|h| h.len() == 64 && h.bytes().all(|b| b.is_ascii_hexdigit())),
                "{rel}: {key} must be 64 hex characters"
            );
        }
    }
}

fn redact_script() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/redact-fixture.sh")
}

fn redact(root: &Path) {
    let status = Command::new("sh")
        .arg(redact_script())
        .arg(root)
        .status()
        .expect("sh runs");
    assert!(status.success(), "redact-fixture.sh must exit 0");
}

#[test]
fn redact_script_rewrites_emails_and_home_paths_in_every_fixture_file_and_nothing_else() {
    let dir = tempfile::tempdir().unwrap();
    let write = |name: &str, text: &str| {
        let path = dir.path().join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, text).unwrap();
        path
    };
    let root = write(
        "fx.jsonl",
        concat!(
            "{\"cwd\":\"/Users/dct/code/x\",\"by\":\"david.carbajosa@dalimaray.io\"}\n",
            "{\"remote\":\"git@github.com:o/r.git\",\"img\":\"icons/128x128@2x.png\"}\n",
            "{\"keep\":\"user@example.invalid\",\"home\":\"/home/user\"}\n",
            "{\"slug\":\"~/.factory/sessions/-Users-dct-code-x\",\"keep\":\"Users-dct\"}\n",
            "no trailing newline /Users/dct",
        ),
    );
    let settings = write(
        "fx.settings.json",
        "{\"dir\":\"/Users/dct\",\"slug\":\"-Users-dct\"}\n",
    );
    let capture = write("fx.capture.json", "{\"who\":\"a.b@c.io\"}\n");
    let child = write(
        "fx.children/c1.jsonl",
        "{\"m\":\"105358724+dctellya@users.noreply.github.com\"}\n",
    );
    let child_settings = write("fx.children/c1.settings.json", "{}\n");
    let other = write(
        "other.jsonl",
        "{\"cwd\":\"/Users/dct\",\"e\":\"x@y.com\"}\n",
    );
    let before_unchanged = (
        std::fs::read(&other).unwrap(),
        std::fs::read(&child_settings).unwrap(),
    );

    redact(&root);

    let read = |p: &Path| std::fs::read_to_string(p).unwrap();
    assert_eq!(
        read(&root),
        concat!(
            "{\"cwd\":\"/home/user/code/x\",\"by\":\"user________@example.invalid\"}\n",
            "{\"remote\":\"user@example.invalid:o/r.git\",\"img\":\"icons/user@example.invalid\"}\n",
            "{\"keep\":\"user@example.invalid\",\"home\":\"/home/user\"}\n",
            "{\"slug\":\"~/.factory/sessions/-home-user-code-x\",\"keep\":\"Users-dct\"}\n",
            "no trailing newline /home/user",
        )
    );
    assert_eq!(
        read(&settings),
        "{\"dir\":\"/home/user\",\"slug\":\"-home-user\"}\n"
    );
    assert_eq!(read(&capture), "{\"who\":\"user@example.invalid\"}\n");
    assert_eq!(
        read(&child),
        format!(
            "{{\"m\":\"user{}@example.invalid\"}}\n",
            "_".repeat(43 - 20)
        )
    );
    assert_eq!(std::fs::read(&other).unwrap(), before_unchanged.0);
    assert_eq!(std::fs::read(&child_settings).unwrap(), before_unchanged.1);

    let once: Vec<_> = [&root, &settings, &capture, &child]
        .iter()
        .map(|p| std::fs::read(p).unwrap())
        .collect();
    redact(&root);
    let twice: Vec<_> = [&root, &settings, &capture, &child]
        .iter()
        .map(|p| std::fs::read(p).unwrap())
        .collect();
    assert_eq!(once, twice, "a second run is a no-op");
}
