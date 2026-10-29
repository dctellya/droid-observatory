//! `observatory-cli metrics <capture> --json`: tokens, cost, waste tokens, reconciliation, budget
//! and model comparison of a capture, with the usage log its header names.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

const M0: &str = "fixtures/streams/2026-10-22-observatory-v1-m0";

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn metrics(args: &[&str]) -> Output {
    let home = tempfile::tempdir().unwrap();
    Command::new(env!("CARGO_BIN_EXE_observatory-cli"))
        .env("HOME", home.path())
        .env_remove("OBSERVATORY_CONFIG")
        .current_dir(repo())
        .arg("metrics")
        .args(args)
        .output()
        .expect("the binary starts")
}

fn report(args: &[&str]) -> Value {
    let output = metrics(args);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("--json prints one JSON document")
}

fn golden_turn_sum() -> [u64; 3] {
    let text = std::fs::read_to_string(repo().join(format!("{M0}.events.json"))).unwrap();
    let events: Vec<Value> = serde_json::from_str(&text).unwrap();
    let mut sum = [0_u64; 3];
    for event in &events {
        if let Some(turn) = event["kind"].get("turn_usage") {
            for (slot, field) in sum.iter_mut().zip(["input", "output", "cache_read"]) {
                *slot += turn[field].as_u64().unwrap();
            }
        }
    }
    sum
}

fn sum_of(rows: &Value) -> [u64; 3] {
    let mut sum = [0_u64; 3];
    for row in rows.as_array().unwrap() {
        for (slot, field) in sum.iter_mut().zip(["input", "output", "cache_read"]) {
            *slot += row["tokens"][field].as_u64().unwrap();
        }
    }
    sum
}

#[test]
fn metrics_json_prints_tokens_per_role_and_lane_equal_to_the_golden_turns() {
    let json = report(&[&format!("{M0}.jsonl"), "--json"]);

    let golden = golden_turn_sum();
    assert_eq!(json["tokens_by_role"].as_array().unwrap().len(), 4);
    assert_eq!(json["tokens_by_lane"].as_array().unwrap().len(), 7);
    assert_eq!(sum_of(&json["tokens_by_role"]), golden);
    assert_eq!(sum_of(&json["tokens_by_lane"]), golden);
    for row in json["cost_by_role"].as_array().unwrap() {
        assert_eq!(row["cost"]["price_table"], "2026-10-22");
        assert!(
            row["multiplier_key"]
                .as_str()
                .unwrap()
                .starts_with("roles.")
        );
        assert_eq!(row["multiplier"], 1.0);
    }
    let waste = json["tokens_in_waste_by_cause"].as_object().unwrap();
    assert_eq!(waste.len(), 9, "eight causes plus throttled");
    assert!(waste.contains_key("throttled"));
    assert_eq!(json["price_table"], "2026-10-22");
}

#[test]
fn metrics_json_reconciles_every_session_against_the_sidecar() {
    let json = report(&[&format!("{M0}.jsonl"), "--json"]);

    let reconciliation = &json["reconciliation"];
    assert_eq!(reconciliation["first_ts"], "2026-10-22T10:18:19.201Z");
    assert!(reconciliation["last_ts"].is_string());
    let sessions = reconciliation["sessions"].as_array().unwrap();
    assert!(sessions.len() > 10, "{}", sessions.len());
    for row in sessions {
        for field in ["input", "output", "cache_read"] {
            let turns = row["turns"][field].as_i64().unwrap();
            let reported = row["reported"][field].as_i64().unwrap();
            assert_eq!(row["difference"][field].as_i64().unwrap(), turns - reported);
        }
    }
}

#[test]
fn metrics_json_compares_the_three_role_models_per_completed_feature() {
    let json = report(&[&format!("{M0}.jsonl"), "--json"]);

    let rows = json["model_comparison"].as_array().unwrap();
    for (role, model) in [
        ("orchestrator", "claude-fable-5.1"),
        ("worker", "claude-sonnet-5-5"),
        ("scrutiny_validator", "claude-opus-5-5"),
    ] {
        let row = rows
            .iter()
            .find(|row| row["role"] == role && row["model"] == model)
            .unwrap_or_else(|| panic!("no row for {role} on {model}"));
        let completed = row["completed_features"].as_f64().unwrap();
        assert!(completed > 0.0);
        let cost = row["cost"]["amount"].as_f64().unwrap();
        let per = row["cost_per_completed_feature"].as_f64().unwrap();
        assert!((per - cost / completed).abs() < 1e-9);
        let active = row["first_pass_active_s"].as_f64().unwrap();
        let per = row["first_pass_active_s_per_completed_feature"]
            .as_f64()
            .unwrap();
        assert!((per - active / completed).abs() < 1e-9);
        assert_eq!(row["cost"]["price_table"], "2026-10-22");
    }
}

#[test]
fn metrics_budget_meter_is_off_by_default_and_a_fraction_with_a_budget() {
    let file = format!("{M0}.jsonl");
    let off = report(&[&file, "--json"]);
    assert_eq!(off["budget"]["enabled"], false);
    assert_eq!(off["budget"]["fraction"], Value::Null);

    let on = report(&[&file, "--json", "--budget-usd", "200"]);
    assert_eq!(on["budget"]["enabled"], true);
    assert_eq!(on["budget"]["budget"], 200.0);
    let cost = on["cost_so_far"]["amount"].as_f64().unwrap();
    let fraction = on["budget"]["fraction"].as_f64().unwrap();
    assert!((fraction - cost / 200.0).abs() < 1e-12);
}

#[test]
fn metrics_of_a_missing_file_exits_nonzero_and_names_it() {
    let output = metrics(&["no-such-capture.jsonl", "--json"]);

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("no-such-capture.jsonl"));
}
