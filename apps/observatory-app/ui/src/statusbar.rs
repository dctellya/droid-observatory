use leptos::prelude::*;
use leptos::task::spawn_local;
use observatory_core::config::Config;
use observatory_core::telemetry::TelemetryReport;
use serde_json::json;

use crate::shell;

/// The window's status bar: what export says (`telemetry{state, endpoint}`, architecture §10).
/// The browser host has no shell and no exporter, so it reads the default, off in config.
#[component]
pub fn StatusBar() -> impl IntoView {
    let state = RwSignal::new("off (config)".to_owned());
    let endpoint = RwSignal::new(Config::default().telemetry.endpoint);
    if shell::is_tauri() {
        spawn_local(async move {
            if let Ok(report) =
                shell::invoke::<_, TelemetryReport>("telemetry_status", &json!({})).await
            {
                state.set(report.state);
                endpoint.set(report.endpoint);
            }
        });
    }
    view! {
        <footer class="statusbar" aria-label="status-bar">
            <span aria-label="telemetry-status">
                "telemetry: " <span class="mono" aria-label="telemetry-state">{move || state.get()}</span>
            </span>
            <span class="mono" aria-label="telemetry-endpoint" title="OTLP endpoint">
                {move || endpoint.get()}
            </span>
        </footer>
    }
}
