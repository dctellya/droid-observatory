//! Checks on the committed session-stream fixtures under `fixtures/streams/`.

use std::path::{Path, PathBuf};

use serde_json::Value;

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
