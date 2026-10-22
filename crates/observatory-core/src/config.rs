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
        ("ingest", "watch_sessions_dir"),
        ("ingest", "jsonl_poll_ms"),
        ("ingest", "max_watched_sessions"),
        ("ingest", "max_session_file_mb"),
        ("ingest", "tail_chunk_kb"),
        ("ingest", "startup_scan_hours"),
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
}
