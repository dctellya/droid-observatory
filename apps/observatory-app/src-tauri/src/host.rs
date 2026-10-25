//! Hosting a mission (FR-L3, architecture §4.5): the `droid exec` child, its stdin writer, the
//! spool its stdout is copied to, and the pump that turns spool lines into stored events,
//! `events_appended` batches, live output and status. Core decides what a line means and what
//! a control writes; this module moves bytes and owns the process.

use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use observatory_core::config::Config;
use observatory_core::ingest::rpc::{
    Ack, AckOutcome, ControlChannel, HostControl, RpcMapper, RpcSource, Spool, encode_initialize,
    method, spool_path,
};
use observatory_core::ingest::{EventBatch, RawRecord, Source};
use observatory_core::mapping::MappingVersion;
use observatory_core::model::{CycleTemplate, Event, MissionId, SessionId};
use observatory_core::store::HostedRun;
use observatory_core::view::{
    AckRow, ControlSent, HostedStatus, LaunchConfig, Launched, OutputLines, PermissionRow,
    QuestionRow, session_uuid,
};
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{ChildStdin, ChildStdout, Command};
use tokio::sync::{mpsc as async_mpsc, watch};

use crate::Shell;
use crate::process::{ProcessGroup, spawn_group};

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// What the hosted runs share: the registry, and the way to tell the ingest thread which
/// session files are not to be attached because a spool already carries them.
pub struct Hosts {
    runs: Mutex<HashMap<String, Arc<Run>>>,
    exclude: mpsc::Sender<SessionId>,
}

struct Run {
    group: ProcessGroup,
    inner: Mutex<Inner>,
    status: Mutex<HostedStatus>,
}

/// The writer and the mapper share a lock: answering a question closes it in the mapper's
/// open requests, and the pump opens them.
struct Inner {
    channel: ControlChannel<ChannelWriter>,
    mapper: RpcMapper,
}

/// A `Write` over a channel: the stdin task owns the pipe and writes what the channel carries.
struct ChannelWriter(async_mpsc::UnboundedSender<Vec<u8>>);

impl Write for ChannelWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0
            .send(buf.to_vec())
            .map_err(|_| io::Error::from(io::ErrorKind::BrokenPipe))?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Hosts {
    pub fn new(exclude: mpsc::Sender<SessionId>) -> Self {
        Self {
            runs: Mutex::new(HashMap::new()),
            exclude,
        }
    }

    fn run(&self, launch_id: &str) -> Result<Arc<Run>, String> {
        lock(&self.runs)
            .get(launch_id)
            .cloned()
            .ok_or_else(|| format!("{launch_id} is not a hosted run of this app"))
    }

    /// Ends the process group of every hosted run: SIGTERM, `grace`, SIGKILL.
    pub async fn reap_all(&self, grace: Duration) {
        let runs: Vec<(String, Arc<Run>)> = lock(&self.runs)
            .iter()
            .map(|(id, run)| (id.clone(), run.clone()))
            .collect();
        for (launch_id, run) in runs {
            let reaped = run.group.reap(grace).await;
            tracing::info!(
                launch_id,
                pid = run.group.pid,
                killed = reaped.killed,
                "host_reaped"
            );
        }
    }

    /// Writes a control to the hosted process. The answer is the id of what was written; the
    /// acknowledgement of Droid follows in `hosted_status`.
    pub fn control(&self, launch_id: &str, control: &HostControl) -> Result<ControlSent, String> {
        let run = self.run(launch_id)?;
        let mut inner = lock(&run.inner);
        let Inner { channel, mapper } = &mut *inner;
        let sent = channel
            .send(control, mapper.open_requests_mut())
            .map_err(|err| err.to_string())?;
        run.status_waiting(mapper);
        tracing::info!(
            launch_id,
            control = ?control,
            request_id = %sent.id,
            method = sent.method,
            "host_control"
        );
        Ok(ControlSent {
            request_id: sent.id,
            method: sent.method.to_owned(),
        })
    }

    pub fn status(&self, launch_id: &str) -> Result<HostedStatus, String> {
        Ok(lock(&self.run(launch_id)?.status).clone())
    }
}

impl Run {
    /// What waits for the human: the open questions and permission requests.
    fn status_waiting(&self, mapper: &RpcMapper) {
        let open = mapper.open_requests();
        let mut status = lock(&self.status);
        status.questions = open
            .open_questions()
            .into_iter()
            .map(|asked| QuestionRow {
                event_id: asked.event_id,
                prompts: asked.prompts.into_iter().map(|p| p.question).collect(),
            })
            .collect();
        status.permissions = open
            .waiting_permissions()
            .into_iter()
            .map(|waiting| PermissionRow {
                request_id: waiting.request_id,
                tools: waiting.tools,
                can_approve: waiting.can_approve,
                can_deny: waiting.can_deny,
            })
            .collect();
    }
}

/// Starts the hosted mission described by `config` and returns at once; the pump task streams
/// from there on.
pub async fn launch<R: Runtime>(
    app: &AppHandle<R>,
    hosts: &Hosts,
    cfg: &Config,
    template: &Arc<CycleTemplate>,
    config: LaunchConfig,
) -> Result<Launched, String> {
    if !std::path::Path::new(&config.repo).is_dir() {
        return Err(format!("the repo {} is not a directory", config.repo));
    }
    let mission_text = std::fs::read_to_string(&config.mission_file).map_err(|err| {
        format!(
            "cannot read the mission file {}: {err}",
            config.mission_file
        )
    })?;
    let session_id = session_uuid(random_bytes().map_err(|err| format!("no random bytes: {err}"))?);
    let launch_id = session_id.clone();

    let spool_file = spool_path(&cfg.mission.resolved_spool_base(), &launch_id);
    let spool =
        Spool::create(&spool_file).map_err(|err| format!("cannot create the spool: {err}"))?;
    let source = RpcSource::open(&spool_file, &cfg.ingest).map_err(|err| err.to_string())?;
    let stderr_log = std::fs::File::create(spool_file.with_extension("stderr.log"))
        .map_err(|err| format!("cannot create the stderr log: {err}"))?;

    let args = config.command_args();
    let mut command = Command::new(&cfg.mission.droid_bin);
    command
        .args(&args)
        .current_dir(&config.repo)
        .stderr(stderr_log);
    let _ = hosts.exclude.send(SessionId(session_id.clone()));
    let spawned = spawn_group(command)
        .map_err(|err| format!("cannot start {}: {err}", cfg.mission.droid_bin))?;
    let (Some(stdin), Some(stdout)) = (spawned.stdin, spawned.stdout) else {
        return Err("the hosted process has no pipes".to_owned());
    };

    let (writes, queued) = async_mpsc::unbounded_channel();
    tokio::spawn(write_stdin(stdin, queued));
    let mut channel = ControlChannel::new(ChannelWriter(writes));
    let sent = channel
        .initialize(&config, &session_id)
        .map_err(|err| err.to_string())?;
    let command_line = format!("{} {}", cfg.mission.droid_bin, args.join(" "));
    tracing::info!(
        launch_id,
        pid = spawned.group.pid,
        cwd = %config.repo,
        command = %command_line,
        spool = %spool_file.display(),
        "host_launch"
    );
    tracing::info!(
        launch_id,
        request_id = %sent.id,
        method = sent.method,
        params = %encode_initialize(&config, &session_id, 1).params,
        "host_control"
    );

    let run = Arc::new(Run {
        group: spawned.group,
        inner: Mutex::new(Inner {
            channel,
            mapper: RpcMapper::with_template(&launch_id, MappingVersion::V1, template.clone()),
        }),
        status: Mutex::new(HostedStatus::starting(&launch_id)),
    });
    lock(&hosts.runs).insert(launch_id.clone(), run.clone());

    let (lines, received) = async_mpsc::unbounded_channel();
    tokio::spawn(read_stdout(stdout, lines));
    let pump = Pump {
        app: app.clone(),
        run,
        launch_id: launch_id.clone(),
        batch: Duration::from_millis(cfg.ui.event_batch_ms),
        exit_wait: Duration::from_millis(cfg.mission.reap_grace_ms),
        spool,
        source,
        auto_approve: cfg.mission.auto_approve_proposal,
        first_message: config.first_message(&mission_text),
        mission_name: config.branch.clone(),
        config,
        mission: None,
        pending: Vec::new(),
        unknown_total: 0,
        output: Vec::new(),
        output_before: 0,
        flush_at: None,
    };
    tokio::spawn(pump.run(received, spawned.exit));

    Ok(Launched {
        launch_id,
        session_id,
        spool: spool_file.display().to_string(),
        pid: spawned.group.pid,
        command: command_line,
    })
}

fn random_bytes() -> io::Result<[u8; 16]> {
    let mut bytes = [0_u8; 16];
    std::fs::File::open("/dev/urandom")?.read_exact(&mut bytes)?;
    Ok(bytes)
}

async fn write_stdin(mut stdin: ChildStdin, mut queued: async_mpsc::UnboundedReceiver<Vec<u8>>) {
    while let Some(bytes) = queued.recv().await {
        if stdin.write_all(&bytes).await.is_err() || stdin.flush().await.is_err() {
            break;
        }
    }
}

async fn read_stdout(stdout: ChildStdout, lines: async_mpsc::UnboundedSender<Vec<u8>>) {
    let mut reader = BufReader::new(stdout);
    loop {
        let mut line = Vec::new();
        match reader.read_until(b'\n', &mut line).await {
            Ok(0) | Err(_) => return,
            Ok(_) => {
                if lines.send(line).is_err() {
                    return;
                }
            }
        }
    }
}

struct Pump<R: Runtime> {
    app: AppHandle<R>,
    run: Arc<Run>,
    launch_id: String,
    batch: Duration,
    exit_wait: Duration,
    spool: Spool,
    source: RpcSource,
    auto_approve: bool,
    config: LaunchConfig,
    first_message: String,
    mission_name: String,
    mission: Option<MissionId>,
    pending: Vec<Event>,
    unknown_total: u64,
    output: Vec<Vec<u8>>,
    output_before: u64,
    flush_at: Option<Instant>,
}

type ExitCode = watch::Receiver<Option<Option<i32>>>;

impl<R: Runtime> Pump<R> {
    async fn run(mut self, mut lines: async_mpsc::UnboundedReceiver<Vec<u8>>, mut exit: ExitCode) {
        loop {
            let received = match self.flush_at {
                Some(at) => {
                    let wait = at.saturating_duration_since(Instant::now());
                    tokio::time::timeout(wait, lines.recv()).await.ok()
                }
                None => Some(lines.recv().await),
            };
            match received {
                Some(Some(line)) => self.line(line),
                Some(None) => break,
                None => self.flush(),
            }
        }
        let code = tokio::time::timeout(self.exit_wait, exit.wait_for(Option::is_some))
            .await
            .ok()
            .and_then(Result::ok)
            .and_then(|status| *status)
            .flatten();
        for record in self.source.drain_partial() {
            self.record(&record);
        }
        let finished = lock(&self.run.inner).mapper.finish();
        self.store(finished);
        lock(&self.run.status).exited(code);
        tracing::info!(launch_id = %self.launch_id, code = ?code, "host_exit");
        self.flush();
    }

    fn line(&mut self, mut line: Vec<u8>) {
        while matches!(line.last(), Some(b'\n' | b'\r')) {
            line.pop();
        }
        if let Err(err) = self.spool.append_line(&line) {
            tracing::error!(%err, launch_id = %self.launch_id, "spool");
            return;
        }
        loop {
            match self.source.next_batch() {
                Ok(records) if !records.is_empty() => {
                    for record in &records {
                        self.record(record);
                    }
                }
                Ok(_) => break,
                Err(err) => {
                    tracing::error!(%err, launch_id = %self.launch_id, "spool read");
                    break;
                }
            }
        }
        self.schedule_flush();
    }

    fn schedule_flush(&mut self) {
        self.flush_at
            .get_or_insert_with(|| Instant::now() + self.batch);
    }

    fn record(&mut self, record: &RawRecord) {
        self.output.push(record.bytes.clone());
        let run = self.run.clone();
        let mut inner = lock(&run.inner);
        let ack = inner.channel.observe(&record.bytes);
        let events = inner.mapper.map(record);
        if let Some(ack) = ack {
            self.on_ack(&run, &mut inner, &ack);
        }
        self.approve_mission_proposals(&mut inner);
        run.status_waiting(&inner.mapper);
        drop(inner);
        self.store(events);
    }

    fn on_ack(&self, run: &Run, inner: &mut Inner, ack: &Ack) {
        let (accepted, message) = match &ack.outcome {
            AckOutcome::Accepted => (true, None),
            AckOutcome::Rejected { message, .. } => (false, Some(message.clone())),
        };
        tracing::info!(
            launch_id = %self.launch_id,
            request_id = %ack.id,
            method = %ack.method,
            accepted,
            "host_ack"
        );
        let mut status = lock(&run.status);
        status.record_ack(AckRow {
            request_id: ack.id.clone(),
            method: ack.method.clone(),
            accepted,
            message,
        });
        drop(status);
        if ack.method == method::INITIALIZE_SESSION && accepted {
            lock(&run.status).initialised();
            match inner.channel.mission_settings(&self.config) {
                Ok(sent) => tracing::info!(
                    launch_id = %self.launch_id,
                    request_id = %sent.id,
                    method = sent.method,
                    "host_control"
                ),
                Err(err) => tracing::error!(%err, launch_id = %self.launch_id, "host_control"),
            }
        }
        // The mission starts only on the models the form named: a rejected update leaves the
        // first message unsent and the rejection in the status.
        if ack.method == method::UPDATE_SESSION_SETTINGS && accepted {
            self.send(
                inner,
                &HostControl::Message {
                    text: self.first_message.clone(),
                },
            );
        }
    }

    /// `droid exec --mission` accepts the orchestrator's proposal without asking; a hosted
    /// session is asked, and the launch screen is the human's consent to run it.
    fn approve_mission_proposals(&self, inner: &mut Inner) {
        if !self.auto_approve {
            return;
        }
        let proposals: Vec<String> = inner
            .mapper
            .open_requests()
            .permissions()
            .iter()
            .filter(|permission| permission.is_mission_proposal())
            .map(|permission| permission.request_id.clone())
            .collect();
        for request_id in proposals {
            tracing::info!(launch_id = %self.launch_id, %request_id, "host_auto_approve");
            self.send(inner, &HostControl::Approve { request_id });
        }
    }

    fn send(&self, inner: &mut Inner, control: &HostControl) {
        let Inner { channel, mapper } = inner;
        match channel.send(control, mapper.open_requests_mut()) {
            Ok(sent) => tracing::info!(
                launch_id = %self.launch_id,
                control = ?control,
                request_id = %sent.id,
                method = sent.method,
                "host_control"
            ),
            Err(err) => tracing::error!(%err, launch_id = %self.launch_id, "host_control"),
        }
    }

    /// Stores the events, in the order they were read, once the rows they hang from exist.
    fn store(&mut self, events: Vec<Event>) {
        let Some(first) = events.first() else {
            return;
        };
        let mission = first.mission_id.clone();
        let shell = self.app.state::<Shell>();
        let mut store = lock(&shell.store);
        if self.mission.is_none() {
            let recorded = store.record_hosted(&HostedRun {
                mission_name: self.mission_name.clone(),
                session: SessionId(self.launch_id.clone()),
                spool: self.spool.path().to_owned(),
                droid_version: None,
                started_at: first.at.to_rfc3339_millis(),
            });
            if let Err(err) = recorded {
                tracing::error!(%err, launch_id = %self.launch_id, "store");
            }
            self.mission = Some(mission.clone());
        }
        if let Err(err) = store.insert_events(&events) {
            tracing::error!(%err, launch_id = %self.launch_id, "store");
        }
        self.unknown_total = store.unknown_count(&mission).unwrap_or(0);
        drop(store);
        self.pending.extend(events);
        self.schedule_flush();
    }

    fn flush(&mut self) {
        self.flush_at = None;
        if let (Some(mission), false) = (&self.mission, self.pending.is_empty()) {
            let batch = EventBatch {
                mission_id: mission.clone(),
                events: std::mem::take(&mut self.pending),
                unknown_total: self.unknown_total,
                retracted: Vec::new(),
            };
            tracing::info!(
                launch_id = %self.launch_id,
                count = batch.events.len(),
                unknown_total = batch.unknown_total,
                event_ids = %batch.events.iter().map(|event| event.id.to_string()).collect::<Vec<_>>().join(","),
                "events_appended"
            );
            let _ = self.app.emit("events_appended", &batch);
        }
        if !self.output.is_empty() {
            let lines = OutputLines::of(&self.launch_id, self.output_before, &self.output);
            self.output_before = lines.total;
            self.output.clear();
            let _ = self.app.emit("live_output", &lines);
        }
        let status = lock(&self.run.status).clone();
        let _ = self.app.emit("hosted_status", &status);
    }
}
