//! The stall flag (FR-H5, FR-H6): every `[intervention].stall_poll_s` the events of each live
//! mission go through core's `stall::evaluate`; a change in the flags is logged as `stall_flag`
//! and emitted to the window. Nothing under the threshold is ever reported.

use std::collections::HashMap;
use std::time::Duration;

use observatory_core::control::stall;
use observatory_core::model::{MissionId, Timestamp};
use observatory_core::view::{StallBatch, StallFlagView};
use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::Shell;
use crate::host::Hosts;

pub fn spawn<R: Runtime>(app: AppHandle<R>) {
    let poll = Duration::from_secs(app.state::<Shell>().cfg.intervention.stall_poll_s.max(1));
    tauri::async_runtime::spawn(async move {
        let mut raised: HashMap<MissionId, Vec<StallFlagView>> = HashMap::new();
        let mut ticker = tokio::time::interval(poll);
        loop {
            ticker.tick().await;
            check(&app, &mut raised);
        }
    });
}

fn live_missions<R: Runtime>(app: &AppHandle<R>) -> Vec<MissionId> {
    let shell = app.state::<Shell>();
    let mut missions: Vec<MissionId> = app
        .state::<Hosts>()
        .launch_ids()
        .into_iter()
        .map(MissionId)
        .collect();
    let watched = shell
        .status
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .watched
        .clone();
    let store = shell
        .store
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    for row in watched {
        if let Ok(Some(mission)) = store.mission_of_session(&row.session_id) {
            if !missions.contains(&mission) {
                missions.push(mission);
            }
        }
    }
    missions
}

fn check<R: Runtime>(app: &AppHandle<R>, raised: &mut HashMap<MissionId, Vec<StallFlagView>>) {
    let shell = app.state::<Shell>();
    let threshold = stall::threshold(&shell.cfg.intervention);
    for mission in live_missions(app) {
        let opened = shell
            .store
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .open_mission(&mission);
        let Ok(opened) = opened else {
            continue;
        };
        let flags: Vec<StallFlagView> = stall::evaluate(
            &stall::activity_of(&opened.events),
            Timestamp::now(),
            threshold,
        )
        .iter()
        .map(StallFlagView::from)
        .collect();
        let before = raised.get(&mission).cloned().unwrap_or_default();
        let same = |a: &StallFlagView, b: &StallFlagView| {
            a.feature_id == b.feature_id && a.anchor == b.anchor
        };
        let changed = flags.len() != before.len()
            || flags
                .iter()
                .any(|flag| !before.iter().any(|known| same(flag, known)));
        if !changed {
            continue;
        }
        for flag in flags
            .iter()
            .filter(|flag| !before.iter().any(|known| same(flag, known)))
        {
            tracing::info!(
                mission_id = %mission,
                feature_id = %flag.feature_id,
                idle_for = flag.idle_secs,
                prompt = %flag.prompt,
                anchor = %flag.anchor,
                "stall_flag"
            );
        }
        let _ = app.emit(
            "stall_flag",
            &StallBatch {
                mission_id: mission.to_string(),
                flags: flags.clone(),
            },
        );
        raised.insert(mission, flags);
    }
}
