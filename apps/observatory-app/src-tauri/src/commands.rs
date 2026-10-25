//! The Tauri commands of architecture §7. Each answers with a core value; the UI renders it.

use std::path::PathBuf;

use observatory_core::config::{Config, MissionConfig};
use observatory_core::control::HumanAction;
use observatory_core::ingest::rpc::HostControl;
use observatory_core::ingest::{AttachStatus, OpenedMission, SessionInfo};
use observatory_core::model::{CycleTemplate, EventId, MissionId, SessionId, Timestamp};
use observatory_core::view::{
    ActionRecorded, BoardDigest, BoardView, ControlSent, DeclaredModels, Filter, HostedStatus,
    LaunchConfig, Launched, MissionRow, RawLineView, ViewConfig, WorkerGrouping, branch_of,
};
use tauri::{AppHandle, State};

use crate::Shell;
use crate::actions;
use crate::host::{self, Hosts};
use crate::logging;

fn text(err: impl std::fmt::Display) -> String {
    err.to_string()
}

fn lock<T>(mutex: &std::sync::Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Replays the capture at `path` (the root `.jsonl` or its `.capture.json`) through the store
/// and answers with the mission as the store now holds it.
#[tauri::command(rename_all = "snake_case")]
pub async fn open_replay(shell: State<'_, Shell>, path: String) -> Result<OpenedMission, String> {
    let mut opened = lock(&shell.store)
        .replay_file_with(
            &PathBuf::from(path),
            &shell.cfg.ingest,
            shell.template.clone(),
        )
        .map_err(text)?;
    opened.live = is_live(&shell, &opened.summary.mission_id);
    Ok(opened)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn list_missions(shell: State<'_, Shell>) -> Result<Vec<MissionRow>, String> {
    lock(&shell.store).missions().map_err(text)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn open_mission(
    shell: State<'_, Shell>,
    mission_id: MissionId,
) -> Result<OpenedMission, String> {
    let mut opened = lock(&shell.store).open_mission(&mission_id).map_err(text)?;
    opened.live = is_live(&shell, &mission_id);
    Ok(opened)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn mission_sessions(
    shell: State<'_, Shell>,
    mission_id: MissionId,
) -> Result<Vec<SessionInfo>, String> {
    lock(&shell.store).sessions_of(&mission_id).map_err(text)
}

/// The sessions being watched, how many were found and not watched, and the warnings raised.
#[tauri::command(rename_all = "snake_case")]
pub async fn list_sessions(shell: State<'_, Shell>) -> Result<AttachStatus, String> {
    Ok(lock(&shell.status).clone())
}

/// Opens the mission a watched session belongs to. The ingest thread tails every growing
/// session file already; later events of the mission arrive as `events_appended` batches.
#[tauri::command(rename_all = "snake_case")]
pub async fn attach(
    shell: State<'_, Shell>,
    session_id: SessionId,
) -> Result<OpenedMission, String> {
    let store = lock(&shell.store);
    let mission = store
        .mission_of_session(&session_id)
        .map_err(text)?
        .ok_or_else(|| format!("session {session_id} is not known to the store"))?;
    let mut opened = store.open_mission(&mission).map_err(text)?;
    drop(store);
    opened.live = is_live(&shell, &mission);
    Ok(opened)
}

/// The counts of the board of a mission under a filter, a grouping and a scrubber position.
#[tauri::command(rename_all = "snake_case")]
pub async fn board(
    shell: State<'_, Shell>,
    mission_id: MissionId,
    filter: Option<Filter>,
    grouping: Option<WorkerGrouping>,
    at: Option<Timestamp>,
) -> Result<BoardDigest, String> {
    let opened = lock(&shell.store).open_mission(&mission_id).map_err(text)?;
    let mission = MissionConfig::default();
    let cfg = ViewConfig {
        filter: filter.unwrap_or_default(),
        grouping: grouping.unwrap_or_default(),
        at,
    };
    let board = BoardView::build(
        &opened.events,
        &cfg,
        &DeclaredModels::from_config(&mission),
        &branch_of(&mission, &opened.summary.name),
        &shell.template,
    );
    Ok(board.digest())
}

/// The raw line of an event, masked for display.
#[tauri::command(rename_all = "snake_case")]
pub async fn event_raw(shell: State<'_, Shell>, event_id: EventId) -> Result<RawLineView, String> {
    lock(&shell.store).event_raw(&event_id).map_err(text)
}

/// Starts the hosted mission of a validated launch form and returns at once; events,
/// live output and status follow as `events_appended`, `live_output` and `hosted_status`.
#[tauri::command(rename_all = "snake_case")]
pub async fn host_launch(
    app: AppHandle,
    shell: State<'_, Shell>,
    hosts: State<'_, Hosts>,
    config: LaunchConfig,
) -> Result<Launched, String> {
    host::launch(&app, &hosts, &shell.cfg, &shell.template, config).await
}

/// Pause, resume, message, answer, approve or deny, written to the hosted process.
#[tauri::command(rename_all = "snake_case")]
pub async fn host_control(
    hosts: State<'_, Hosts>,
    launch_id: String,
    control: HostControl,
) -> Result<ControlSent, String> {
    hosts.control(&launch_id, &control)
}

/// One of the six controls: the `human_actions` row and its human-overlay event, and for a
/// hosted mission (`launch_id` set) the line written to the process. An attached mission has no
/// control channel (D-008): the event is recorded and the window shows the text to paste.
#[tauri::command(rename_all = "snake_case")]
pub async fn human_action(
    app: AppHandle,
    shell: State<'_, Shell>,
    hosts: State<'_, Hosts>,
    mission_id: MissionId,
    launch_id: Option<String>,
    action: HumanAction,
    text: Option<String>,
) -> Result<ActionRecorded, String> {
    match launch_id {
        Some(launch_id) => hosts.human_action(&app, &launch_id, action, text),
        None => {
            let about = actions::about_event(&shell, &action)?;
            actions::record(&app, &shell, &mission_id, &action, about.as_ref())
        }
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn host_status(
    hosts: State<'_, Hosts>,
    launch_id: String,
) -> Result<HostedStatus, String> {
    hosts.status(&launch_id)
}

/// The panel has drawn these events: the render half of the 1 s latency measurement (the
/// emit half is the `events_appended` line of the host). `open` says whether the board of
/// their mission was on screen; `ms` is the window's own `event:first_on_board` measure.
#[tauri::command(rename_all = "snake_case")]
pub async fn panel_rendered(
    mission_id: String,
    event_ids: Vec<String>,
    open: bool,
    ms: Option<f64>,
) {
    if let Some(ms) = ms {
        logging::interval("panel_rendered", ms, event_ids.len());
    }
    tracing::info!(
        mission_id,
        count = event_ids.len(),
        open,
        event_ids = %event_ids.join(","),
        "panel_rendered"
    );
}

/// The configuration in effect: the launch form pre-fills from `[mission]` and the live
/// output keeps `[ui].live_output_lines`.
#[tauri::command(rename_all = "snake_case")]
pub async fn app_config(shell: State<'_, Shell>) -> Result<Config, String> {
    Ok(shell.cfg.clone())
}

/// The cycle template in effect: the lanes the board draws, their names and the clocks.
#[tauri::command(rename_all = "snake_case")]
pub async fn cycle_template(shell: State<'_, Shell>) -> Result<CycleTemplate, String> {
    Ok(shell.template.as_ref().clone())
}

fn is_live(shell: &Shell, mission: &MissionId) -> bool {
    let watched = lock(&shell.status).watched.clone();
    let store = lock(&shell.store);
    watched.iter().any(|row| {
        store
            .mission_of_session(&row.session_id)
            .ok()
            .flatten()
            .is_some_and(|of| of == *mission)
    })
}
