//! A control pressed in the window (architecture §7 `human_action`, §8): the `human_actions`
//! row and its human-overlay event, and for a hosted mission the line written to the process.
//! Core decides what each control means; this module moves them to the store and the pipe.

use std::sync::{Mutex, MutexGuard};

use observatory_core::control::{self, HumanAction};
use observatory_core::ingest::EventBatch;
use observatory_core::ingest::rpc::HostControl;
use observatory_core::model::{Event, MissionId, Timestamp};
use observatory_core::view::ActionRecorded;
use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::Shell;
use crate::host::{Hosts, PendingAck};

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The host control that carries `action` to the process; `None` for a control that reaches no
/// host (mark complete). `run_request` finds the JSON-RPC id of a `permission.requested` event.
fn relay_of(
    action: &HumanAction,
    text: Option<&str>,
    run_request: impl Fn(&str) -> Option<String>,
) -> Result<Option<HostControl>, String> {
    let control = match action {
        HumanAction::Answer { question_event, .. } => HostControl::Answer {
            question_id: question_event.to_string(),
            text: text
                .filter(|text| !text.is_empty())
                .ok_or("an answer needs text")?
                .to_owned(),
        },
        HumanAction::Approve { request_event } | HumanAction::Deny { request_event } => {
            let request_id = run_request(&request_event.to_string())
                .ok_or_else(|| format!("{request_event} is not an open permission request"))?;
            if matches!(action, HumanAction::Approve { .. }) {
                HostControl::Approve { request_id }
            } else {
                HostControl::Deny { request_id }
            }
        }
        HumanAction::MarkComplete { .. } => return Ok(None),
        other => {
            if matches!(other, HumanAction::Message { .. }) && text.is_none_or(str::is_empty) {
                return Err("a message needs text".to_owned());
            }
            control::relay(other, text).ok_or("this control has no host line")?
        }
    };
    Ok(Some(control))
}

impl Hosts {
    /// Writes the control to the hosted process and records it, in that order: a control the
    /// process cannot take (a request already answered) leaves no row. The store is locked
    /// while the run's lock is held, so the pump cannot read the acknowledgement before the
    /// control it acknowledges is recorded.
    pub fn human_action<R: Runtime>(
        &self,
        app: &AppHandle<R>,
        launch_id: &str,
        action: HumanAction,
        text: Option<String>,
    ) -> Result<ActionRecorded, String> {
        let run = self.run(launch_id)?;
        let mut guard = lock(&run.inner);
        let inner = &mut *guard;
        let shell = app.state::<Shell>();
        let about = about_event(&shell, &action)?;
        let relay = relay_of(&action, text.as_deref(), |event| {
            run.request_for_event(event)
        })?;
        if let Some(relay) = relay {
            let key_call = match &action {
                HumanAction::Answer { question_event, .. } => inner
                    .mapper
                    .open_requests()
                    .question_for_event(&question_event.to_string())
                    .map(|question| question.tool_call_id.clone()),
                _ => None,
            };
            let sent = inner
                .channel
                .send(&relay, inner.mapper.open_requests_mut())
                .map_err(|err| err.to_string())?;
            log_host_control(launch_id, &action, &relay, &sent.id, sent.method);
            let key = match &action {
                HumanAction::Approve { .. } | HumanAction::Deny { .. } => {
                    format!("permission:{}", sent.id)
                }
                HumanAction::Answer { .. } => match key_call {
                    Some(call) => format!("call:{call}"),
                    None => sent.id.clone(),
                },
                _ => sent.id.clone(),
            };
            lock(&run.pending).insert(
                key,
                PendingAck {
                    action: action.clone(),
                    about: about.clone(),
                },
            );
        }
        let recorded = record(
            app,
            &shell,
            &MissionId(launch_id.to_owned()),
            &action,
            about.as_ref(),
        )?;
        run.status_waiting(&guard.mapper);
        Ok(recorded)
    }
}

fn log_host_control(
    launch_id: &str,
    _action: &HumanAction,
    relay: &HostControl,
    request_id: &str,
    method: &str,
) {
    tracing::info!(
        launch_id,
        control = ?relay,
        request_id,
        method,
        "host_control"
    );
}

/// The event an action concerns, read from the store: the question or request it answers.
pub(crate) fn about_event(shell: &Shell, action: &HumanAction) -> Result<Option<Event>, String> {
    let Some(id) = action.payload_ptr() else {
        return Ok(None);
    };
    lock(&shell.store)
        .event(id)
        .map(Some)
        .map_err(|err| format!("cannot read {id}: {err}"))
}

/// Records the row and the event, and tells the window about the event.
pub fn record<R: Runtime>(
    app: &AppHandle<R>,
    shell: &Shell,
    mission: &MissionId,
    action: &HumanAction,
    about: Option<&Event>,
) -> Result<ActionRecorded, String> {
    let (recorded, unknown_total) = {
        let mut store = lock(&shell.store);
        let recorded = store
            .record_human_action(action, mission, Timestamp::now(), &shell.template, about)
            .map_err(|err| err.to_string())?;
        let unknown = store.unknown_count(mission).unwrap_or(0);
        (recorded, unknown)
    };
    tracing::info!(
        mission_id = %mission,
        action_id = recorded.action_id,
        event_id = %recorded.event.id,
        kind = action.kind(),
        "human_action"
    );
    let _ = app.emit(
        "events_appended",
        &EventBatch {
            mission_id: mission.clone(),
            events: vec![recorded.event.clone()],
            unknown_total,
            retracted: Vec::new(),
        },
    );
    Ok(ActionRecorded {
        action_id: recorded.action_id,
        event_id: recorded.event.id.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use std::io::Write;
    use std::sync::{Arc, Mutex};

    use observatory_core::config::Config;
    use observatory_core::ingest::AttachStatus;
    use observatory_core::model::{EventId, FeatureId, SessionId};
    use observatory_core::store::{HostedRun, Store};

    use super::*;

    const MESSAGE: &str = "Keep it to the one feature.";
    const ANSWER: &str = "salud";
    const REASON: &str = "blocked on the vendor key";

    #[derive(Clone, Default)]
    struct Capture(Arc<Mutex<Vec<u8>>>);

    impl Write for Capture {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            lock(&self.0).extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    fn logged(run: impl FnOnce()) -> String {
        let capture = Capture::default();
        let writer = capture.clone();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(move || writer.clone())
            .with_ansi(false)
            .finish();
        tracing::subscriber::with_default(subscriber, run);
        String::from_utf8(lock(&capture.0).clone()).expect("the log is text")
    }

    fn shell(name: &str) -> (Shell, std::path::PathBuf) {
        let cfg = Config::default();
        let path = std::env::temp_dir().join(format!(
            "observatory-actions-{name}-{}.sqlite",
            std::process::id()
        ));
        let mut store = Store::open(&path).expect("the store opens");
        store
            .record_hosted(&HostedRun {
                mission_name: "hosted".into(),
                session: SessionId("mission-1".to_owned()),
                spool: path.with_extension("jsonl"),
                droid_version: None,
                started_at: "2026-10-25T09:00:00.000Z".into(),
            })
            .expect("the hosted mission row");
        let shell = Shell {
            template: Arc::new(cfg.cycle.load_template().template),
            store: Mutex::new(store),
            status: Arc::new(Mutex::new(AttachStatus::default())),
            cfg,
        };
        (shell, path)
    }

    #[test]
    fn host_control_log_carries_lengths_not_text() {
        let cases = [
            (
                HumanAction::message(MESSAGE),
                HostControl::Message {
                    text: MESSAGE.to_owned(),
                },
                MESSAGE,
            ),
            (
                HumanAction::answer(EventId("question-1".to_owned()), ANSWER),
                HostControl::Answer {
                    question_id: "question-1".to_owned(),
                    text: ANSWER.to_owned(),
                },
                ANSWER,
            ),
            (
                HumanAction::mark_skipped(FeatureId("f-1".to_owned()), REASON),
                HostControl::Message {
                    text: format!("Skip f-1: {REASON}"),
                },
                REASON,
            ),
        ];
        for (action, relay, text) in cases {
            let log = logged(|| log_host_control("launch", &action, &relay, "req-1", "message"));
            println!("{log}");
            assert!(!log.contains(text), "the log carries the text: {log}");
            assert!(
                log.contains("text_len") || log.contains("reason_len"),
                "the log carries no length: {log}"
            );
        }
    }

    #[test]
    fn recorded_action_log_carries_text_len() {
        let (shell, path) = shell("recorded");
        let app = tauri::test::mock_app();
        let action = HumanAction::message(MESSAGE);
        let log = logged(|| {
            record(
                app.handle(),
                &shell,
                &MissionId("mission-1".to_owned()),
                &action,
                None,
            )
            .expect("the action is recorded");
        });
        println!("{log}");
        assert!(log.contains("human_action"), "no human_action line: {log}");
        assert!(log.contains("text_len"), "the line has no text_len: {log}");
        assert!(!log.contains(MESSAGE), "the line carries the text: {log}");
        let _ = std::fs::remove_file(path);
    }
}
