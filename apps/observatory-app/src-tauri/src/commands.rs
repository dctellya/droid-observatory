//! The Tauri commands of architecture §7. Each answers with a core value; the UI renders it.

use std::path::PathBuf;

use observatory_core::config::{Config, MissionConfig};
use observatory_core::control::HumanAction;
use observatory_core::ingest::rpc::HostControl;
use observatory_core::ingest::{AttachStatus, OpenedMission, SessionInfo};
use observatory_core::model::{CycleTemplate, EventId, MissionId, SessionId, Timestamp};
use observatory_core::store::Store;
use observatory_core::telemetry::TelemetryReport;
use observatory_core::view::{
    ActionRecorded, BoardDigest, BoardView, ControlSent, DeclaredModels, Filter, HostedStatus,
    LaunchConfig, Launched, MissionRow, RawLineView, ViewConfig, WorkerGrouping, branch_of,
};
use tauri::{AppHandle, State};

use crate::Shell;
use crate::actions;
use crate::host::{self, Hosts};
use crate::logging;
use crate::telemetry::Telemetry;

fn text(err: impl std::fmt::Display) -> String {
    err.to_string()
}

fn lock<T>(mutex: &std::sync::Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The branch the Gate placeholder and the gate log name for a mission: the one the git poll
/// recorded in `missions.branch`, else the configured pattern resolved for the mission's name.
pub(crate) fn resolve_branch(
    store: &Store,
    config: &MissionConfig,
    mission: &MissionId,
    name: &str,
) -> String {
    store
        .mission_branch(mission)
        .ok()
        .flatten()
        .unwrap_or_else(|| branch_of(config, name))
}

/// The branch of a mission just opened, see `resolve_branch`.
fn branch_in(store: &Store, shell: &Shell, opened: &OpenedMission) -> String {
    resolve_branch(
        store,
        &shell.cfg.mission,
        &opened.summary.mission_id,
        &opened.summary.name,
    )
}

/// Replays the capture at `path` (the root `.jsonl` or its `.capture.json`) through the store
/// and answers with the mission as the store now holds it.
#[tauri::command(rename_all = "snake_case")]
pub async fn open_replay(
    shell: State<'_, Shell>,
    telemetry: State<'_, std::sync::Arc<Telemetry>>,
    path: String,
) -> Result<OpenedMission, String> {
    let mut opened = {
        let mut store = lock(&shell.store);
        let mut opened = store
            .replay_file_with(
                &PathBuf::from(path),
                &shell.cfg.ingest,
                shell.template.clone(),
            )
            .map_err(text)?;
        opened.branch = Some(branch_in(&store, &shell, &opened));
        opened
    };
    opened.live = is_live(&shell, &opened.summary.mission_id);
    telemetry.observe(&opened);
    Ok(opened)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn list_missions(shell: State<'_, Shell>) -> Result<Vec<MissionRow>, String> {
    lock(&shell.store).missions().map_err(text)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn open_mission(
    shell: State<'_, Shell>,
    telemetry: State<'_, std::sync::Arc<Telemetry>>,
    mission_id: MissionId,
) -> Result<OpenedMission, String> {
    let mut opened = {
        let store = lock(&shell.store);
        let mut opened = store.open_mission(&mission_id).map_err(text)?;
        opened.branch = Some(branch_in(&store, &shell, &opened));
        opened
    };
    opened.live = is_live(&shell, &mission_id);
    telemetry.observe(&opened);
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
    telemetry: State<'_, std::sync::Arc<Telemetry>>,
    session_id: SessionId,
) -> Result<OpenedMission, String> {
    let store = lock(&shell.store);
    let mission = store
        .mission_of_session(&session_id)
        .map_err(text)?
        .ok_or_else(|| format!("session {session_id} is not known to the store"))?;
    let mut opened = store.open_mission(&mission).map_err(text)?;
    opened.branch = Some(branch_in(&store, &shell, &opened));
    drop(store);
    opened.live = is_live(&shell, &mission);
    telemetry.observe(&opened);
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
    let (opened, branch) = {
        let store = lock(&shell.store);
        let opened = store.open_mission(&mission_id).map_err(text)?;
        let branch = branch_in(&store, &shell, &opened);
        (opened, branch)
    };
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
        &branch,
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

/// The Gate header of a mission as the last poll read it, for a board opened after the
/// `gate_changed` that announced it.
#[tauri::command(rename_all = "snake_case")]
pub async fn gate_group(
    cache: State<'_, crate::github::GateCache>,
    mission_id: MissionId,
) -> Result<Option<observatory_core::view::GateGroup>, String> {
    Ok(cache.get(&mission_id))
}

/// The worktrees git listed for the milestones of a mission at the last poll, for the cards.
#[tauri::command(rename_all = "snake_case")]
pub async fn milestone_worktrees(
    cache: State<'_, crate::git::GitCache>,
    mission_id: MissionId,
) -> Result<Vec<observatory_core::view::MilestoneWorktree>, String> {
    Ok(cache.get(&mission_id))
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

/// What the status bar says about export: `{enabled, reason}` plus the `state` and `endpoint` of
/// the `telemetry{state, endpoint}` line (architecture §10).
#[tauri::command(rename_all = "snake_case")]
pub async fn telemetry_status(
    telemetry: State<'_, std::sync::Arc<Telemetry>>,
) -> Result<TelemetryReport, String> {
    Ok(telemetry.report())
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use observatory_core::ingest::AttachStatus;
    use observatory_core::store::Store;
    use tauri::Manager;

    use super::*;

    #[test]
    fn the_cycle_template_command_returns_the_loaded_lanes_in_order() {
        let cfg = Config::default();
        let path =
            std::env::temp_dir().join(format!("observatory-cmd-{}.sqlite", std::process::id()));
        let app = tauri::test::mock_app();
        let template = Arc::new(cfg.cycle.load_template().template);
        app.manage(Arc::new(Telemetry::start(&cfg, template.clone())));
        app.manage(Shell {
            template,
            store: Mutex::new(Store::open(&path).expect("the store opens")),
            status: Arc::new(Mutex::new(AttachStatus::default())),
            cfg,
        });
        let shell = app.state::<Shell>();
        let loaded =
            tauri::async_runtime::block_on(cycle_template(shell)).expect("the command answers");
        let names: Vec<&str> = loaded.lanes.iter().map(|lane| lane.name.as_str()).collect();
        println!("cycle_template lanes: {names:?}");
        assert_eq!(loaded, CycleTemplate::engineering());
        assert_eq!(names.len(), 7, "the default template has seven lanes");
        let _ = std::fs::remove_file(path);
    }

    const LITERAL_BRANCH: &str = "mission/observatory-v1";

    fn app_with_branch_pattern(
        pattern: &str,
        tag: &str,
    ) -> (tauri::App<tauri::test::MockRuntime>, PathBuf) {
        let mut cfg = Config::default();
        cfg.mission.branch_pattern = pattern.to_owned();
        let path = std::env::temp_dir().join(format!(
            "observatory-branch-{tag}-{}.sqlite",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let app = tauri::test::mock_app();
        let template = Arc::new(cfg.cycle.load_template().template);
        app.manage(Arc::new(Telemetry::start(&cfg, template.clone())));
        app.manage(Shell {
            template,
            store: Mutex::new(Store::open(&path).expect("the store opens")),
            status: Arc::new(Mutex::new(AttachStatus::default())),
            cfg,
        });
        (app, path)
    }

    fn s_fixture() -> String {
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../fixtures/streams/2026-10-22-observatory-v1-s.jsonl"
        )
        .to_owned()
    }

    #[test]
    fn an_opened_mission_names_the_branch_of_the_configured_pattern() {
        let (app, path) = app_with_branch_pattern(LITERAL_BRANCH, "pattern");
        let opened = tauri::async_runtime::block_on(open_replay(
            app.state::<Shell>(),
            app.state::<Arc<Telemetry>>(),
            s_fixture(),
        ))
        .expect("the capture replays");
        println!("opened branch: {:?}", opened.branch);
        assert_eq!(opened.branch.as_deref(), Some(LITERAL_BRANCH));
        let again = tauri::async_runtime::block_on(open_mission(
            app.state::<Shell>(),
            app.state::<Arc<Telemetry>>(),
            opened.summary.mission_id.clone(),
        ))
        .expect("the mission opens from the store");
        assert_eq!(again.branch.as_deref(), Some(LITERAL_BRANCH));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn an_opened_mission_prefers_the_branch_the_store_recorded() {
        let (app, path) = app_with_branch_pattern("mission/<name>", "recorded");
        let opened = tauri::async_runtime::block_on(open_replay(
            app.state::<Shell>(),
            app.state::<Arc<Telemetry>>(),
            s_fixture(),
        ))
        .expect("the capture replays");
        let mission = opened.summary.mission_id.clone();
        assert!(
            lock(&app.state::<Shell>().store)
                .set_mission_branch(&mission, "mission/recorded")
                .expect("the branch is recorded")
        );
        let again = tauri::async_runtime::block_on(open_mission(
            app.state::<Shell>(),
            app.state::<Arc<Telemetry>>(),
            mission,
        ))
        .expect("the mission opens from the store");
        assert_eq!(again.branch.as_deref(), Some("mission/recorded"));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn telemetry_status_is_off_in_config_with_the_collector_endpoint_by_default() {
        let (app, path) = app_with_branch_pattern(LITERAL_BRANCH, "telemetry");
        let report =
            tauri::async_runtime::block_on(telemetry_status(app.state::<Arc<Telemetry>>()))
                .expect("the status answers");
        assert!(!report.status.enabled);
        assert_eq!(report.status.reason, "off in config");
        assert_eq!(report.state, "off (config)");
        assert_eq!(report.endpoint, "http://localhost:4318");
        assert_eq!(report.bar_text(), "telemetry: off (config)");
        let json = serde_json::to_value(&report).expect("serialises");
        assert_eq!(json["enabled"], false);
        assert_eq!(json["reason"], "off in config");
        let _ = std::fs::remove_file(path);
    }
}
