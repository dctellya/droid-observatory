//! The embedded WebDriver server answers anything on a local port: it must exist in the `e2e`
//! build only (VAL-E2E-005). These tests read the manifest, the cargo graph and the sources.

use std::path::{Path, PathBuf};
use std::process::Command;

const PLUGIN: &str = "tauri-plugin-wdio-webdriver";

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn manifest() -> toml::Table {
    let text = std::fs::read_to_string(crate_dir().join("Cargo.toml")).unwrap();
    text.parse().unwrap()
}

fn tree_of_plugin(features: &[&str]) -> std::process::Output {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let mut command = Command::new(cargo);
    command
        .args([
            "tree",
            "--offline",
            "-p",
            "observatory-app",
            "-e",
            "features",
        ])
        .args(["--prefix", "none", "-i", PLUGIN])
        .current_dir(crate_dir());
    if !features.is_empty() {
        command.args(["--features", &features.join(",")]);
    }
    command.output().expect("cargo tree runs")
}

#[test]
fn the_manifest_makes_the_plugin_optional_and_the_feature_not_default() {
    let manifest = manifest();
    let features = manifest["features"].as_table().expect("[features]");
    assert_eq!(
        features["e2e"].as_array().unwrap(),
        &vec![toml::Value::String(format!("dep:{PLUGIN}"))]
    );
    let in_default = features
        .get("default")
        .and_then(toml::Value::as_array)
        .is_some_and(|all| all.iter().any(|name| name.as_str() == Some("e2e")));
    assert!(!in_default, "`e2e` must not be a default feature");
    assert_eq!(
        manifest["dependencies"][PLUGIN]["optional"].as_bool(),
        Some(true)
    );
}

#[test]
fn the_plugin_is_not_in_the_graph_without_the_e2e_feature() {
    let output = tree_of_plugin(&[]);
    assert!(
        !output.status.success(),
        "the plugin is reachable without --features e2e:\n{}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn the_plugin_is_in_the_graph_with_the_e2e_feature() {
    let output = tree_of_plugin(&["e2e"]);
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "{text}{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(text.contains(PLUGIN), "{text}");
}

fn rust_files(dir: &Path, found: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, found);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            found.push(path);
        }
    }
}

#[test]
fn every_use_of_the_plugin_in_the_sources_is_behind_the_e2e_cfg() {
    let mut files = Vec::new();
    rust_files(&crate_dir().join("src"), &mut files);
    let mut uses = 0;
    for file in files {
        let text = std::fs::read_to_string(&file).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        for (index, line) in lines.iter().enumerate() {
            if !line.contains("tauri_plugin_wdio_webdriver") || line.trim_start().starts_with("//")
            {
                continue;
            }
            uses += 1;
            let gated = line.contains("feature = \"e2e\"")
                || index
                    .checked_sub(1)
                    .is_some_and(|before| lines[before].contains("#[cfg(feature = \"e2e\")]"));
            assert!(
                gated,
                "{}:{} is not behind cfg(feature = \"e2e\")",
                file.display(),
                index + 1
            );
        }
    }
    assert!(uses > 0, "the e2e build never registers the plugin");
}

#[test]
fn the_default_capability_carries_no_webdriver_permission() {
    let capabilities = crate_dir().join("capabilities");
    for entry in std::fs::read_dir(capabilities).unwrap() {
        let text = std::fs::read_to_string(entry.unwrap().path()).unwrap();
        assert!(
            !text.contains("wdio"),
            "a capability names the WebDriver plugin"
        );
    }
}
