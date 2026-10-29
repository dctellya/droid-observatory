#[cfg(test)]
mod tests {
    use std::io::Write;
    use std::sync::{Arc, Mutex};

    use observatory_core::config::Config;
    use observatory_core::control::merge::MergeState;
    use observatory_core::ingest::AttachStatus;
    use observatory_core::ingest::forge::{Pr, PrState};
    use observatory_core::ingest::github::{GitHubClient, HttpResponse, HttpTransport};
    use observatory_core::model::{EventKind, MissionId, SessionId, Timestamp};
    use observatory_core::store::{EventScope, HostedRun, Store, TimeRange};
    use tauri::Manager;

    use super::*;
    use crate::Shell;

    const MISSION: &str = "mission-1";
    const REFUSED: &str = "Merge of PR #24 into demo/base refused: token lacks pull_requests:write. Merge in GitHub; the badge updates from the next poll.";

    /// A transport that answers every merge request with the 403 of a read-only token.
    #[derive(Clone, Default)]
    struct ReadOnly {
        requests: Arc<Mutex<Vec<String>>>,
    }

    impl HttpTransport for ReadOnly {
        fn get(&self, url: &str) -> Result<HttpResponse, String> {
            Err(format!("unexpected GET {url}"))
        }

        fn put(&self, url: &str, _body: &str) -> Result<HttpResponse, String> {
            self.requests.lock().unwrap().push(format!("PUT {url}"));
            Ok(HttpResponse {
                status: 403,
                body: r#"{"message":"Resource not accessible by personal access token"}"#.into(),
                rate_limit_remaining: Some(4999),
                rate_limit_reset: None,
            })
        }
    }

    #[derive(Clone, Default)]
    struct Capture(Arc<Mutex<Vec<u8>>>);

    impl Write for Capture {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
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
        String::from_utf8(capture.0.lock().unwrap().clone()).expect("the log is text")
    }

    fn pr() -> Pr {
        Pr {
            number: 24,
            head_sha: "f4de0fe".into(),
            head: "demo/red-1".into(),
            base: "demo/base".into(),
            url: "https://api.github.com/repos/dctellya/droid-observatory/pulls/24".into(),
            created_at: Timestamp::parse("2026-10-29T10:00:00Z").unwrap(),
            state: PrState::Open,
            auto_merge: false,
        }
    }

    fn app(name: &str) -> (tauri::App<tauri::test::MockRuntime>, std::path::PathBuf) {
        let cfg = Config::default();
        let path = std::env::temp_dir().join(format!(
            "observatory-merge-{name}-{}.sqlite",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let mut store = Store::open(&path).expect("the store opens");
        store
            .record_hosted(&HostedRun {
                mission_name: "hosted".into(),
                session: SessionId(MISSION.to_owned()),
                spool: path.with_extension("jsonl"),
                droid_version: None,
                started_at: "2026-10-29T09:00:00.000Z".into(),
            })
            .expect("the hosted mission row");
        let app = tauri::test::mock_app();
        app.manage(Shell {
            template: Arc::new(cfg.cycle.load_template().template),
            store: Mutex::new(store),
            status: Arc::new(Mutex::new(AttachStatus::default())),
            cfg,
        });
        (app, path)
    }

    fn mission() -> MissionId {
        MissionId(MISSION.to_owned())
    }

    fn desk_with_open_pr() -> MergeDesk {
        let desk = MergeDesk::default();
        desk.note(&mission(), Some(&pr()), None);
        desk
    }

    fn client(transport: &ReadOnly) -> GitHubClient<ReadOnly> {
        GitHubClient::new(transport.clone(), "dctellya/droid-observatory")
    }

    #[test]
    fn a_confirmed_press_sends_one_put_and_records_one_refused_overlay_row() {
        let (app, path) = app("one");
        let desk = desk_with_open_pr();
        let transport = ReadOnly::default();
        let outcome = press_merge(
            app.handle(),
            &desk,
            &client(&transport),
            &mission(),
            24,
            "demo/base",
        )
        .expect("the press is answered");
        assert_eq!(outcome.state, MergeState::Refused);
        assert_eq!(outcome.message, REFUSED);
        let puts = transport.requests.lock().unwrap().clone();
        assert_eq!(puts.len(), 1, "{puts:?}");
        assert!(puts[0].ends_with("/repos/dctellya/droid-observatory/pulls/24/merge"));

        let shell = app.state::<Shell>();
        let store = shell.store.lock().unwrap();
        let rows = store.human_actions(&mission()).unwrap();
        assert_eq!(rows.len(), 1, "{rows:?}");
        assert_eq!(
            (rows[0].kind.as_str(), rows[0].state.as_str()),
            ("merge", "refused")
        );
        let events = store
            .events(&EventScope::Mission(mission()), TimeRange::ALL)
            .unwrap();
        let approvals: Vec<_> = events
            .iter()
            .filter(|event| event.kind == EventKind::ApprovalGiven)
            .collect();
        assert_eq!(approvals.len(), 1);
        assert_eq!(&approvals[0].lane, shell.template.final_lane());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn two_presses_send_two_requests_and_leave_two_refused_rows() {
        let (app, path) = app("two");
        let desk = desk_with_open_pr();
        let transport = ReadOnly::default();
        for _ in 0..2 {
            press_merge(
                app.handle(),
                &desk,
                &client(&transport),
                &mission(),
                24,
                "demo/base",
            )
            .expect("the press is answered");
        }
        assert_eq!(transport.requests.lock().unwrap().len(), 2);
        let shell = app.state::<Shell>();
        let rows = shell
            .store
            .lock()
            .unwrap()
            .human_actions(&mission())
            .unwrap();
        let refused = rows
            .iter()
            .filter(|row| row.kind == "merge" && row.state == "refused")
            .count();
        assert_eq!(refused, 2, "{rows:?}");
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn a_press_the_poll_does_not_back_sends_nothing_and_records_nothing() {
        let (app, path) = app("unbacked");
        let transport = ReadOnly::default();
        let unseen = press_merge(
            app.handle(),
            &MergeDesk::default(),
            &client(&transport),
            &mission(),
            24,
            "demo/base",
        );
        assert!(unseen.is_err(), "no PR was polled: {unseen:?}");
        let desk = desk_with_open_pr();
        let elsewhere = press_merge(
            app.handle(),
            &desk,
            &client(&transport),
            &mission(),
            24,
            "main",
        );
        assert!(elsewhere.is_err(), "another target: {elsewhere:?}");
        let mut closed = pr();
        closed.state = PrState::Closed;
        desk.note(&mission(), Some(&closed), None);
        let gone = press_merge(
            app.handle(),
            &desk,
            &client(&transport),
            &mission(),
            24,
            "demo/base",
        );
        assert!(gone.is_err(), "a closed PR: {gone:?}");
        assert!(transport.requests.lock().unwrap().is_empty());
        let shell = app.state::<Shell>();
        assert!(
            shell
                .store
                .lock()
                .unwrap()
                .human_actions(&mission())
                .unwrap()
                .is_empty()
        );
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn the_log_names_the_request_the_forbidden_answer_and_the_refusal() {
        let (app, path) = app("log");
        let desk = desk_with_open_pr();
        let transport = ReadOnly::default();
        let log = logged(|| {
            press_merge(
                app.handle(),
                &desk,
                &client(&transport),
                &mission(),
                24,
                "demo/base",
            )
            .expect("the press is answered");
        });
        println!("{log}");
        let line = |needle: &str| log.lines().filter(|line| line.contains(needle)).count();
        assert_eq!(line("merge_requested"), 1, "{log}");
        assert_eq!(line("github.request"), 1, "{log}");
        assert!(log.contains("status=403"), "{log}");
        assert_eq!(line("merge_result"), 1, "{log}");
        assert!(log.contains("refused"), "{log}");
        assert!(!log.contains("github_pat"), "{log}");
        let _ = std::fs::remove_file(path);
    }
}
