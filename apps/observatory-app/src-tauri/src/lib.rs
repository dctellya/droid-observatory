//! Tauri shell: IO plumbing only. The commands in `commands` and the ingest thread in `ingest`
//! hand `observatory-core` values to the UI; nothing is decided here.

mod actions;
mod commands;
mod host;
mod ingest;
#[cfg(test)]
mod live_check;
mod logging;
mod process;
mod stall;

use std::sync::{Arc, Mutex};

use observatory_core::config::Config;
use observatory_core::ingest::AttachStatus;
use observatory_core::model::CycleTemplate;
use observatory_core::store::Store;
use tauri::Manager;

/// What the commands share: the configuration in effect, a store connection of their own (the
/// ingest thread holds another) and the attach status the thread publishes.
pub struct Shell {
    pub cfg: Config,
    /// The cycle template read once at start-up (F-041); every board and mapper iterates it.
    pub template: Arc<CycleTemplate>,
    pub store: Mutex<Store>,
    pub status: Arc<Mutex<AttachStatus>>,
}

pub fn run() {
    logging::init();
    let (cfg, origin) = match Config::load_with_origin(None) {
        Ok(loaded) => loaded,
        Err(err) => {
            tracing::error!(%err, "config");
            std::process::exit(1);
        }
    };
    let store_path = cfg.store.resolved_path();
    let store = match Store::open(&store_path) {
        Ok(store) => store,
        Err(err) => {
            tracing::error!(%err, "store");
            std::process::exit(1);
        }
    };
    tracing::info!(
        config = %origin.as_ref().map_or_else(|| "defaults".to_owned(), |p| p.display().to_string()),
        store = %store_path.display(),
        sessions_dir = %cfg.ingest.factory_sessions_dir,
        "startup"
    );
    let loaded = cfg.cycle.load_template();
    if let Some(warning) = &loaded.warning {
        tracing::warn!(%warning, "cycle template");
    }
    let status = Arc::new(Mutex::new(AttachStatus::default()));
    let shell = Shell {
        cfg: cfg.clone(),
        template: Arc::new(loaded.template),
        store: Mutex::new(store),
        status: status.clone(),
    };

    let (exclude, excluded) = std::sync::mpsc::channel();
    let grace = std::time::Duration::from_millis(cfg.mission.reap_grace_ms);
    let builder = tauri::Builder::default();
    #[cfg(feature = "e2e")]
    let builder = builder.plugin(tauri_plugin_wdio_webdriver::init());
    builder
        .manage(shell)
        .manage(host::Hosts::new(exclude))
        .setup(move |app| {
            app.manage(ingest::spawn(
                app.handle().clone(),
                cfg,
                store_path,
                status,
                excluded,
            ));
            stall::spawn(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::open_replay,
            commands::list_missions,
            commands::open_mission,
            commands::mission_sessions,
            commands::list_sessions,
            commands::attach,
            commands::board,
            commands::event_raw,
            commands::host_launch,
            commands::host_control,
            commands::host_status,
            commands::human_action,
            commands::app_config,
            commands::cycle_template,
            commands::panel_rendered,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(move |app, event| match event {
            tauri::RunEvent::WindowEvent {
                event: tauri::WindowEvent::CloseRequested { .. },
                ..
            }
            | tauri::RunEvent::ExitRequested { .. } => reap_hosted(app, grace),
            tauri::RunEvent::Exit => {
                reap_hosted(app, grace);
                app.state::<ingest::Ingest>().stop();
            }
            _ => {}
        });
}

/// Closing the window ends the hosted missions: their process groups get SIGTERM, then SIGKILL
/// after the grace. Repeating it finds the groups gone.
fn reap_hosted(app: &tauri::AppHandle, grace: std::time::Duration) {
    tauri::async_runtime::block_on(app.state::<host::Hosts>().reap_all(grace));
}
