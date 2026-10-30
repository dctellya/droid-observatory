//! Tauri shell: IO plumbing only. The commands in `commands` and the ingest thread in `ingest`
//! hand `observatory-core` values to the UI; nothing is decided here.

mod actions;
mod commands;
#[cfg(feature = "e2e")]
mod e2e_gate;
mod git;
mod github;
mod host;
mod ingest;
#[cfg(test)]
mod live_check;
mod logging;
mod merge;
mod process;
mod stall;
pub mod telemetry;

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

/// The config and the store the window opens on; either failing ends the process, with the
/// reason in the log.
fn open_or_exit() -> (Config, std::path::PathBuf, Store) {
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
    (cfg, store_path, store)
}

pub fn run() {
    logging::init();
    let (cfg, store_path, store) = open_or_exit();
    let loaded = cfg.cycle.load_template();
    if let Some(warning) = &loaded.warning {
        tracing::warn!(%warning, "cycle template");
    }
    let status = Arc::new(Mutex::new(AttachStatus::default()));
    let github_cfg = cfg.clone();
    let github_store_path = store_path.clone();
    let git_cfg = cfg.clone();
    let git_store_path = store_path.clone();
    let shell = Shell {
        cfg: cfg.clone(),
        template: Arc::new(loaded.template),
        store: Mutex::new(store),
        status: status.clone(),
    };

    let telemetry = Arc::new(telemetry::Telemetry::start(&cfg, shell.template.clone()));
    let ingest_telemetry = telemetry.clone();
    let template = shell.template.clone();
    let git_template = template.clone();
    let (exclude, excluded) = std::sync::mpsc::channel();
    let grace = std::time::Duration::from_millis(cfg.mission.reap_grace_ms);
    let builder = tauri::Builder::default();
    #[cfg(feature = "e2e")]
    let builder = builder.plugin(tauri_plugin_wdio_webdriver::init());
    builder
        .manage(shell)
        .manage(host::Hosts::new(exclude))
        .manage(github::GateCache::default())
        .manage(git::GitCache::default())
        .manage(merge::MergeDesk::default())
        .manage(telemetry)
        .setup(move |app| {
            app.manage(ingest::spawn(
                app.handle().clone(),
                cfg,
                store_path,
                status,
                excluded,
                ingest_telemetry,
            ));
            app.manage(github::spawn(
                app.handle().clone(),
                github_cfg,
                template,
                github_store_path,
            ));
            app.manage(git::spawn(
                app.handle().clone(),
                git_cfg,
                git_template,
                git_store_path,
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
            commands::gate_group,
            commands::milestone_worktrees,
            merge::merge,
            #[cfg(feature = "e2e")]
            e2e_gate::e2e_gate_poll,
            commands::panel_rendered,
            commands::telemetry_status,
            commands::metrics,
            commands::price_table,
            commands::export,
            commands::export_default_path,
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
                app.state::<github::Github>().stop();
                app.state::<git::Git>().stop();
                app.state::<ingest::Ingest>().stop();
                app.state::<Arc<telemetry::Telemetry>>().stop();
            }
            _ => {}
        });
}

/// Closing the window ends the hosted missions: their process groups get SIGTERM, then SIGKILL
/// after the grace. Repeating it finds the groups gone.
fn reap_hosted(app: &tauri::AppHandle, grace: std::time::Duration) {
    tauri::async_runtime::block_on(app.state::<host::Hosts>().reap_all(grace));
}
