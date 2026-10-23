//! Tauri shell: IO plumbing only. The commands in `commands` and the ingest thread in `ingest`
//! hand `observatory-core` values to the UI; nothing is decided here.

mod commands;
mod ingest;
mod logging;

use std::sync::{Arc, Mutex};

use observatory_core::config::Config;
use observatory_core::ingest::AttachStatus;
use observatory_core::store::Store;

/// What the commands share: the configuration in effect, a store connection of their own (the
/// ingest thread holds another) and the attach status the thread publishes.
pub struct Shell {
    pub cfg: Config,
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
    let status = Arc::new(Mutex::new(AttachStatus::default()));
    let shell = Shell {
        cfg: cfg.clone(),
        store: Mutex::new(store),
        status: status.clone(),
    };

    tauri::Builder::default()
        .manage(shell)
        .setup(move |app| {
            ingest::spawn(app.handle().clone(), cfg, store_path, status);
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
