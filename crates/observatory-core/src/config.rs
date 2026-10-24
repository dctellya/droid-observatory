//! Configuration contract: `config/observatory.example.toml` is the source of every default.
//!
//! Compiled to wasm for the UI (architecture I9), so nothing here depends on the platform
//! beyond `std::fs` and `std::env`, which only run when the shell asks for a file.

use std::ffi::{OsStr, OsString};
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Environment variable that overrides the config file location.
pub const CONFIG_ENV: &str = "OBSERVATORY_CONFIG";

/// Location below `$HOME` of the user's config file.
const DEFAULT_RELATIVE_PATH: &str = ".droid-observatory/config.toml";

#[derive(Debug)]
pub enum ConfigError {
    Io {
        path: PathBuf,
        source: io::Error,
    },
    Parse {
        path: Option<PathBuf>,
        message: String,
    },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => {
                write!(f, "cannot read config {}: {source}", path.display())
            }
            Self::Parse {
                path: Some(path),
                message,
            } => {
                write!(f, "invalid config {}: {message}", path.display())
            }
            Self::Parse {
                path: None,
                message,
            } => write!(f, "invalid config: {message}"),
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Parse { .. } => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MergePolicy {
    /// Read the policy from GitHub rulesets and branch protection.
    Auto,
    /// Always show the Merge button.
    Human,
    /// Never show the Merge button.
    Policy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverheadApportionment {
    Separate,
    Equal,
}

/// Every table accepts partial files: a missing key keeps its default.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub mission: MissionConfig,
    pub ingest: IngestConfig,
    pub ui: UiConfig,
    pub store: StoreConfig,
    pub github: GithubConfig,
    pub intervention: InterventionConfig,
    pub metrics: MetricsConfig,
    pub telemetry: TelemetryConfig,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MissionConfig {
    pub orchestrator_model: String,
    pub worker_model: String,
    pub validator_model: String,
    pub branch_pattern: String,
}

impl Default for MissionConfig {
    fn default() -> Self {
        Self {
            orchestrator_model: "claude-fable-5.1".into(),
            worker_model: "claude-sonnet-5-5".into(),
            validator_model: "claude-opus-5-5".into(),
            branch_pattern: "mission/<name>".into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct IngestConfig {
    pub factory_sessions_dir: String,
    pub factory_logs_dir: String,
    pub watch_sessions_dir: bool,
    pub jsonl_poll_ms: u64,
    pub max_watched_sessions: u32,
    pub max_session_file_mb: u64,
    pub tail_chunk_kb: u64,
    pub startup_scan_hours: u64,
    pub runner_tags: Vec<String>,
}

impl IngestConfig {
    /// `factory_logs_dir` with a leading `~/` replaced by `$HOME`.
    pub fn resolved_logs_dir(&self) -> PathBuf {
        expand_home(&self.factory_logs_dir, std::env::var_os("HOME").as_deref())
    }

    /// `factory_sessions_dir` with a leading `~/` replaced by `$HOME`.
    pub fn resolved_sessions_dir(&self) -> PathBuf {
        expand_home(
            &self.factory_sessions_dir,
            std::env::var_os("HOME").as_deref(),
        )
    }
}

impl Default for IngestConfig {
    fn default() -> Self {
        Self {
            factory_sessions_dir: "~/.factory/sessions".into(),
            factory_logs_dir: "~/.factory/logs".into(),
            watch_sessions_dir: true,
            jsonl_poll_ms: 500,
            max_watched_sessions: 64,
            max_session_file_mb: 512,
            tail_chunk_kb: 1024,
            startup_scan_hours: 24,
            runner_tags: [
                "exec",
                "mission-worker",
                "mission-session",
                "subagent",
                "read-only",
            ]
            .map(String::from)
            .to_vec(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct UiConfig {
    pub event_batch_ms: u64,
    pub dev_port: u16,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            event_batch_ms: 100,
            dev_port: 3141,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct StoreConfig {
    pub path: String,
}

impl Default for StoreConfig {
    fn default() -> Self {
        Self {
            path: "~/.droid-observatory/observatory.sqlite".into(),
        }
    }
}

impl StoreConfig {
    /// `path` with a leading `~/` replaced by `$HOME`.
    pub fn resolved_path(&self) -> PathBuf {
        expand_home(&self.path, std::env::var_os("HOME").as_deref())
    }
}

/// Replaces a leading `~` or `~/` with `home`. Any other path, and `~` without a known home,
/// is returned as written.
pub fn expand_home(path: &str, home: Option<&OsStr>) -> PathBuf {
    match (path.strip_prefix('~'), home) {
        (Some(""), Some(home)) => PathBuf::from(home),
        (Some(rest), Some(home)) if rest.starts_with('/') => {
            Path::new(home).join(rest.trim_start_matches('/'))
        }
        _ => PathBuf::from(path),
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GithubConfig {
    pub poll_interval_s: u64,
    pub merge_policy: MergePolicy,
    pub target_branch: String,
    pub repo: String,
}

impl Default for GithubConfig {
    fn default() -> Self {
        Self {
            poll_interval_s: 30,
            merge_policy: MergePolicy::Auto,
            target_branch: "main".into(),
            repo: "dctellya/droid-observatory".into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct InterventionConfig {
    pub stall_threshold_min: u64,
}

impl Default for InterventionConfig {
    fn default() -> Self {
        Self {
            stall_threshold_min: 20,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MetricsConfig {
    pub price_table: String,
    pub overhead_apportionment: OverheadApportionment,
    pub budget: f64,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            price_table: "config/prices/2026-10-22.toml".into(),
            overhead_apportionment: OverheadApportionment::Separate,
            budget: 0.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TelemetryConfig {
    pub enabled: bool,
    pub protocol: String,
    pub endpoint: String,
    pub keychain_service: String,
    pub service_name: String,
    pub metric_export_interval_s: u64,
    pub metric_labels_allowed: Vec<String>,
    pub max_series: u32,
    pub mission_metrics: Vec<String>,
    pub max_milestone_labels: u32,
    pub mission_switch_min_interval_min: u64,
    pub buckets: BucketsConfig,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        let own = |names: &[&str]| names.iter().map(|name| (*name).to_owned()).collect();
        Self {
            enabled: false,
            protocol: "http/protobuf".into(),
            endpoint: "https://otlp-gateway-prod-eu-west-2.grafana.net/otlp".into(),
            keychain_service: "droid-observatory/otlp-headers".into(),
            service_name: "droid-observatory".into(),
            metric_export_interval_s: 15,
            metric_labels_allowed: own(&["mission", "milestone", "role", "model", "cause"]),
            max_series: 200,
            mission_metrics: own(&[
                "observatory.mission.cycle_seconds",
                "observatory.feature.cycle_seconds",
                "observatory.waste_seconds",
                "observatory.rework_windows",
                "observatory.fix_loops",
                "observatory.tokens.input",
                "observatory.tokens.output",
                "observatory.tokens.cache_read",
                "observatory.tokens.cache_write",
                "observatory.cost_usd",
                "observatory.unknown_events",
            ]),
            max_milestone_labels: 8,
            mission_switch_min_interval_min: 30,
            buckets: BucketsConfig::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct BucketsConfig {
    pub mission_cycle_seconds: Vec<u64>,
    pub feature_cycle_seconds: Vec<u64>,
}

impl Default for BucketsConfig {
    fn default() -> Self {
        Self {
            mission_cycle_seconds: vec![3600, 7200, 14400, 28800, 86400, 172800],
            feature_cycle_seconds: vec![600, 1800, 3600, 7200, 14400],
        }
    }
}

impl Config {
    pub fn from_toml_str(text: &str) -> Result<Self, ConfigError> {
        toml::from_str(text).map_err(|err| ConfigError::Parse {
            path: None,
            message: err.to_string(),
        })
    }

    /// Loads `path` when given (a missing file is an error). Without a path, reads
    /// `$OBSERVATORY_CONFIG`, else `~/.droid-observatory/config.toml`; a missing default file
    /// means defaults.
    pub fn load(path: Option<&Path>) -> Result<Self, ConfigError> {
        Self::load_with_origin(path).map(|(config, _)| config)
    }

    /// Like `load`, and the file the settings were read from: `None` when no config file
    /// was in effect and the defaults were used.
    pub fn load_with_origin(path: Option<&Path>) -> Result<(Self, Option<PathBuf>), ConfigError> {
        Self::locate(path, std::env::var_os(CONFIG_ENV), std::env::var_os("HOME"))
    }

    #[cfg(test)]
    fn load_with(
        explicit: Option<&Path>,
        env_override: Option<OsString>,
        home: Option<OsString>,
    ) -> Result<Self, ConfigError> {
        Self::locate(explicit, env_override, home).map(|(config, _)| config)
    }

    fn locate(
        explicit: Option<&Path>,
        env_override: Option<OsString>,
        home: Option<OsString>,
    ) -> Result<(Self, Option<PathBuf>), ConfigError> {
        if let Some(path) = explicit {
            return Self::read_file(path).map(|config| (config, Some(path.to_owned())));
        }
        let resolved = match (env_override, home) {
            (Some(path), _) if !path.is_empty() => PathBuf::from(path),
            (_, Some(home)) => Path::new(&home).join(DEFAULT_RELATIVE_PATH),
            _ => return Ok((Self::default(), None)),
        };
        match Self::read_file(&resolved) {
            Ok(config) => Ok((config, Some(resolved))),
            Err(ConfigError::Io { source, .. }) if source.kind() == io::ErrorKind::NotFound => {
                tracing::debug!(path = %resolved.display(), "no config file, using defaults");
                Ok((Self::default(), None))
            }
            Err(err) => Err(err),
        }
    }

    fn read_file(path: &Path) -> Result<Self, ConfigError> {
        let text = std::fs::read_to_string(path).map_err(|source| ConfigError::Io {
            path: path.to_owned(),
            source,
        })?;
        toml::from_str(&text).map_err(|err| ConfigError::Parse {
            path: Some(path.to_owned()),
            message: err.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE: &str = include_str!("../../../config/observatory.example.toml");

    /// Every key SPEC §5.5 and architecture §11 name, as (section path, key).
    const DOCUMENTED_KEYS: &[(&str, &str)] = &[
        ("mission", "orchestrator_model"),
        ("mission", "worker_model"),
        ("mission", "validator_model"),
        ("mission", "branch_pattern"),
        ("ingest", "factory_sessions_dir"),
        ("ingest", "factory_logs_dir"),
        ("ingest", "watch_sessions_dir"),
        ("ingest", "jsonl_poll_ms"),
        ("ingest", "max_watched_sessions"),
        ("ingest", "max_session_file_mb"),
        ("ingest", "tail_chunk_kb"),
        ("ingest", "startup_scan_hours"),
        ("ingest", "runner_tags"),
        ("ui", "event_batch_ms"),
        ("ui", "dev_port"),
        ("store", "path"),
        ("github", "poll_interval_s"),
        ("github", "merge_policy"),
        ("github", "target_branch"),
        ("github", "repo"),
        ("intervention", "stall_threshold_min"),
        ("metrics", "price_table"),
        ("metrics", "overhead_apportionment"),
        ("metrics", "budget"),
        ("telemetry", "enabled"),
        ("telemetry", "protocol"),
        ("telemetry", "endpoint"),
        ("telemetry", "keychain_service"),
        ("telemetry", "service_name"),
        ("telemetry", "metric_export_interval_s"),
        ("telemetry", "metric_labels_allowed"),
        ("telemetry", "max_series"),
        ("telemetry", "mission_metrics"),
        ("telemetry", "max_milestone_labels"),
        ("telemetry", "mission_switch_min_interval_min"),
        ("telemetry.buckets", "mission_cycle_seconds"),
        ("telemetry.buckets", "feature_cycle_seconds"),
    ];

    fn section<'a>(root: &'a toml::Table, path: &str) -> &'a toml::Table {
        path.split('.').fold(root, |table, part| {
            table
                .get(part)
                .and_then(toml::Value::as_table)
                .unwrap_or_else(|| panic!("section [{path}] missing from the example"))
        })
    }

    #[test]
    fn config_example_parses_with_every_documented_key() {
        let root: toml::Table = EXAMPLE.parse().expect("example is valid TOML");
        for (path, key) in DOCUMENTED_KEYS {
            assert!(
                section(&root, path).contains_key(*key),
                "[{path}] {key} missing"
            );
        }

        let config = Config::from_toml_str(EXAMPLE).expect("example loads through core::config");
        assert_eq!(config.ingest.max_watched_sessions, 64);
        assert_eq!(config.ingest.max_session_file_mb, 512);
        assert_eq!(config.ingest.tail_chunk_kb, 1024);
        assert_eq!(config.ingest.startup_scan_hours, 24);
        assert_eq!(config.ui.event_batch_ms, 100);
        assert_eq!(config.ui.dev_port, 3141);
        assert_eq!(config.store.path, "~/.droid-observatory/observatory.sqlite");
        assert_eq!(config.telemetry.max_milestone_labels, 8);
        assert_eq!(config.telemetry.mission_switch_min_interval_min, 30);
        assert_eq!(config.telemetry.buckets.mission_cycle_seconds.len(), 6);
        assert_eq!(config.telemetry.buckets.feature_cycle_seconds.len(), 5);
        assert_eq!(config.github.target_branch, "main");
        assert_eq!(config.github.repo, "dctellya/droid-observatory");
        assert_eq!(config.intervention.stall_threshold_min, 20);
    }

    #[test]
    fn example_config_equals_defaults() {
        let config = Config::from_toml_str(EXAMPLE).expect("example parses");
        assert_eq!(config, Config::default());
    }

    #[test]
    fn every_key_in_the_example_has_a_reason_comment() {
        // A key directly under another key shares that key's comment (the three role models).
        let mut covered = false;
        for line in EXAMPLE.lines() {
            let is_key = line.split_once(" = ").is_some_and(|(name, _)| {
                !name.is_empty() && name.chars().all(|c| c.is_ascii_lowercase() || c == '_')
            });
            if is_key {
                assert!(covered, "no reason comment above: {line}");
            }
            covered = line.starts_with('#') || (is_key && covered);
        }
    }

    #[test]
    fn home_is_expanded_only_at_the_start_of_a_store_path() {
        let home = OsStr::new("/home/someone");
        let expand = |path| expand_home(path, Some(home));
        assert_eq!(
            expand("~/.droid-observatory/observatory.sqlite"),
            Path::new("/home/someone/.droid-observatory/observatory.sqlite")
        );
        assert_eq!(expand("~"), Path::new("/home/someone"));
        assert_eq!(expand("/tmp/~/x.sqlite"), Path::new("/tmp/~/x.sqlite"));
        assert_eq!(expand("~other/x"), Path::new("~other/x"));
        assert_eq!(expand_home("~/x", None), Path::new("~/x"));
    }

    #[test]
    fn sessions_dir_resolves_like_the_store_path() {
        let cfg = IngestConfig {
            factory_sessions_dir: "/var/sessions".into(),
            ..IngestConfig::default()
        };
        assert_eq!(cfg.resolved_sessions_dir(), Path::new("/var/sessions"));
    }

    #[test]
    fn load_reports_the_file_the_settings_came_from() {
        let dir = scratch_dir("origin");
        let file = dir.join("config.toml");
        std::fs::write(&file, "[store]\npath = \"/tmp/x.sqlite\"\n").unwrap();

        let (config, origin) = Config::locate(Some(&file), None, None).unwrap();
        assert_eq!(config.store.path, "/tmp/x.sqlite");
        assert_eq!(origin.as_deref(), Some(file.as_path()));

        let empty_home = Some(dir.join("nobody").into_os_string());
        let (config, origin) = Config::locate(None, None, empty_home).unwrap();
        assert_eq!(config, Config::default());
        assert_eq!(origin, None);
    }

    #[test]
    fn partial_file_keeps_defaults_for_missing_keys() {
        let config = Config::from_toml_str("[ui]\nevent_batch_ms = 250\n").expect("parses");
        assert_eq!(config.ui.event_batch_ms, 250);
        assert_eq!(config.ui.dev_port, UiConfig::default().dev_port);
        assert_eq!(config.ingest, IngestConfig::default());
    }

    #[test]
    fn unknown_key_is_rejected() {
        let err = Config::from_toml_str("[ui]\nevent_batch = 1\n").unwrap_err();
        assert!(
            matches!(err, ConfigError::Parse { path: None, .. }),
            "{err}"
        );
    }

    #[test]
    fn load_reads_explicit_path_and_errors_when_it_is_missing() {
        let dir = scratch_dir("explicit");
        let file = dir.join("config.toml");
        std::fs::write(&file, "[github]\nmerge_policy = \"human\"\n").unwrap();

        let config = Config::load(Some(&file)).expect("explicit file loads");
        assert_eq!(config.github.merge_policy, MergePolicy::Human);

        let missing = Config::load(Some(&dir.join("absent.toml")));
        assert!(matches!(missing, Err(ConfigError::Io { .. })));
    }

    #[test]
    fn load_uses_env_override_then_home_then_defaults() {
        let dir = scratch_dir("resolve");
        let override_file = dir.join("override.toml");
        std::fs::write(&override_file, "[ui]\ndev_port = 3142\n").unwrap();
        let from_env =
            Config::load_with(None, Some(override_file.into_os_string()), None).expect("env file");
        assert_eq!(from_env.ui.dev_port, 3142);

        let home = dir.join("home");
        std::fs::create_dir_all(home.join(".droid-observatory")).unwrap();
        std::fs::write(home.join(DEFAULT_RELATIVE_PATH), "[ui]\ndev_port = 3143\n").unwrap();
        let from_home =
            Config::load_with(None, None, Some(home.clone().into_os_string())).expect("home file");
        assert_eq!(from_home.ui.dev_port, 3143);

        let empty_home = Some(dir.join("nobody").into_os_string());
        assert_eq!(
            Config::load_with(None, None, empty_home).unwrap(),
            Config::default()
        );
        assert_eq!(
            Config::load_with(None, None, None).unwrap(),
            Config::default()
        );
    }

    #[test]
    fn invalid_file_error_names_the_path() {
        let dir = scratch_dir("invalid");
        let file = dir.join("config.toml");
        std::fs::write(&file, "[ui\n").unwrap();
        let err = Config::load(Some(&file)).unwrap_err();
        assert!(err.to_string().contains("config.toml"), "{err}");
    }

    fn scratch_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "observatory-config-test-{}-{label}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }
}
