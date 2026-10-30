//! The metrics exporter to the local OTLP collector (architecture §10, D-046). Core decides
//! whether it starts (`resolve_start`) and what a mission contributes (`mission_sample`); this
//! module sends it: the eleven instruments, the periodic reader and the HTTP client. Off unless
//! `[telemetry].enabled`; the optional header item is read from the keychain only when configured.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use observatory_core::config::{Config, TelemetryConfig};
use observatory_core::ingest::OpenedMission;
use observatory_core::metrics::{Notes, PriceTable};
use observatory_core::model::{CycleTemplate, Timestamp};
use observatory_core::secrets;
use observatory_core::telemetry::{
    ExporterStart, InstrumentKind, Labels, MissionSample, SampleInput, TelemetryReport,
    mission_sample, resolve_start,
};
use opentelemetry::KeyValue;
use opentelemetry::metrics::{Counter, Histogram, MeterProvider};
use opentelemetry_http::{Bytes, HttpClient, HttpError, Request, Response};
use opentelemetry_otlp::{Protocol, WithExportConfig, WithHttpConfig};
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::metrics::{PeriodicReader, SdkMeterProvider};

/// The price table of the default config, for a run whose configured path does not resolve from
/// the working directory (a bundled app has no repository around it).
const BUNDLED_PRICES: &str = include_str!("../../../../config/prices/2026-10-22.toml");

/// Headers the SDK builds itself and the client lets through. Anything else, including a header
/// the SDK would read from `OTEL_EXPORTER_OTLP_HEADERS`, is dropped: only the configured item's
/// header is added to a request (D-010, D-046).
const SDK_HEADERS: [&str; 3] = ["content-type", "content-encoding", "user-agent"];

/// The HTTP client of the exporter: ureq, one log line per POST with its status (never a header),
/// and the header allow-list above.
#[derive(Debug)]
struct LoggedClient {
    agent: ureq::Agent,
    extra_header: Option<String>,
}

impl LoggedClient {
    fn new(extra_header: Option<&str>, timeout: Duration) -> Self {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_global(Some(timeout))
            .build()
            .into();
        Self {
            agent,
            extra_header: extra_header.map(str::to_ascii_lowercase),
        }
    }

    fn keeps(&self, name: &str) -> bool {
        SDK_HEADERS.contains(&name) || self.extra_header.as_deref() == Some(name)
    }
}

#[async_trait]
impl HttpClient for LoggedClient {
    async fn send_bytes(&self, request: Request<Bytes>) -> Result<Response<Bytes>, HttpError> {
        let (parts, body) = request.into_parts();
        let url = parts.uri.to_string();
        let mut builder = Request::builder().method(parts.method).uri(parts.uri);
        for (name, value) in &parts.headers {
            if self.keeps(name.as_str()) {
                builder = builder.header(name, value);
            }
        }
        let request = builder.body(body.to_vec())?;
        let response = match self.agent.run(request) {
            Ok(response) => response,
            Err(error) => {
                tracing::warn!(url = %url, %error, "telemetry.post failed");
                return Err(Box::new(error));
            }
        };
        let status = response.status();
        tracing::info!(method = "POST", url = %url, status = status.as_u16(), "telemetry.post");
        let (parts, mut body) = response.into_parts();
        let bytes = body.read_to_vec()?;
        Ok(Response::from_parts(parts, Bytes::from(bytes)))
    }
}

enum Instrument {
    Counter(Counter<f64>),
    Histogram(Histogram<f64>),
}

#[derive(Default)]
struct Memory {
    /// The mission being exported and since when; another one waits for the switch interval.
    mission: Option<(String, Instant)>,
    /// The running total last added to each counter series.
    totals: BTreeMap<(String, Labels), f64>,
    /// Observations already recorded: histogram name and the point's key.
    observed: BTreeSet<(String, String)>,
}

/// The running exporter: a meter provider whose periodic reader posts to the collector.
pub struct Exporter {
    provider: SdkMeterProvider,
    instruments: HashMap<String, Instrument>,
    memory: Mutex<Memory>,
    switch_after: Duration,
}

fn attributes(labels: &Labels) -> Vec<KeyValue> {
    labels
        .iter()
        .map(|(label, value)| KeyValue::new(label.as_str(), value.clone()))
        .collect()
}

impl Exporter {
    /// Builds the instruments of the plan and starts the periodic reader. The first POST comes
    /// one `interval` after the first recorded value.
    pub fn start(
        start: &ExporterStart,
        interval: Duration,
        service_name: &str,
    ) -> Result<Self, String> {
        let url = format!("{}/v1/metrics", start.endpoint.trim_end_matches('/'));
        let timeout = Duration::from_secs(10);
        let client = LoggedClient::new(start.header.as_ref().map(|h| h.name()), timeout);
        let mut exporter = opentelemetry_otlp::MetricExporter::builder()
            .with_http()
            .with_http_client(client)
            .with_protocol(Protocol::HttpBinary)
            .with_endpoint(url)
            .with_timeout(timeout);
        if let Some(header) = &start.header {
            exporter = exporter.with_headers(HashMap::from([(
                header.name().to_owned(),
                header.expose_value().to_owned(),
            )]));
        }
        let exporter = exporter.build().map_err(|error| error.to_string())?;
        let reader = PeriodicReader::builder(exporter)
            .with_interval(interval)
            .build();
        let version = env!("CARGO_PKG_VERSION");
        let resource = Resource::builder_empty()
            .with_attributes([
                KeyValue::new("service.name", service_name.to_owned()),
                KeyValue::new("service.version", version),
            ])
            .build();
        let provider = SdkMeterProvider::builder()
            .with_resource(resource)
            .with_reader(reader)
            .build();
        let meter = provider.meter("observatory");
        let mut instruments = HashMap::new();
        for planned in &start.plan.instruments {
            let instrument = match &planned.kind {
                InstrumentKind::Counter => {
                    Instrument::Counter(meter.f64_counter(planned.name.clone()).build())
                }
                InstrumentKind::Histogram { buckets } => Instrument::Histogram(
                    meter
                        .f64_histogram(planned.name.clone())
                        .with_boundaries(buckets.iter().map(|b| *b as f64).collect())
                        .build(),
                ),
            };
            instruments.insert(planned.name.clone(), instrument);
        }
        let names: Vec<&str> = start
            .plan
            .instruments
            .iter()
            .map(|i| i.name.as_str())
            .collect();
        tracing::info!(
            instruments = %names.join(","),
            service_name,
            service_version = version,
            endpoint = %start.endpoint,
            interval_s = interval.as_secs(),
            "telemetry.exporter"
        );
        Ok(Self {
            provider,
            instruments,
            memory: Mutex::new(Memory::default()),
            switch_after: Duration::ZERO,
        })
    }

    /// One mission at a time: another mission's sample is ignored until `after` has passed since
    /// the current one was first recorded (`[telemetry].mission_switch_min_interval_min`).
    pub fn switch_after(mut self, after: Duration) -> Self {
        self.switch_after = after;
        self
    }

    /// Records what `sample` adds to what was recorded before: counters by the growth of their
    /// running total, observations once. Returns the number of values recorded.
    pub fn observe(&self, mission: &str, sample: &MissionSample) -> usize {
        let mut memory = self
            .memory
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match &memory.mission {
            Some((current, since)) if current != mission => {
                if since.elapsed() < self.switch_after {
                    return 0;
                }
                *memory = Memory::default();
                memory.mission = Some((mission.to_owned(), Instant::now()));
            }
            Some(_) => {}
            None => memory.mission = Some((mission.to_owned(), Instant::now())),
        }
        let mut recorded = 0;
        for point in &sample.counters {
            let Some(Instrument::Counter(counter)) = self.instruments.get(point.name) else {
                continue;
            };
            let key = (point.name.to_owned(), point.labels.clone());
            let last = memory.totals.get(&key).copied().unwrap_or(0.0);
            if point.value > last {
                counter.add(point.value - last, &attributes(&point.labels));
                memory.totals.insert(key, point.value);
                recorded += 1;
            }
        }
        for point in &sample.observations {
            let Some(Instrument::Histogram(histogram)) = self.instruments.get(point.name) else {
                continue;
            };
            if memory
                .observed
                .insert((point.name.to_owned(), point.key.clone()))
            {
                histogram.record(point.value, &attributes(&point.labels));
                recorded += 1;
            }
        }
        recorded
    }

    /// Posts what was recorded now rather than at the next interval.
    pub fn flush(&self) {
        if let Err(error) = self.provider.force_flush() {
            tracing::warn!(%error, "telemetry.flush");
        }
    }

    pub fn shutdown(&self) {
        if let Err(error) = self.provider.shutdown() {
            tracing::warn!(%error, "telemetry.shutdown");
        }
    }
}

/// What the shell holds: the report the status bar shows and, when export is on, the exporter.
pub struct Telemetry {
    report: Mutex<TelemetryReport>,
    exporter: Option<Exporter>,
    cfg: TelemetryConfig,
    template: std::sync::Arc<CycleTemplate>,
    prices: Option<PriceTable>,
    /// The last time a live batch was folded in, so a busy mission costs one sample per interval.
    last_live: Mutex<Option<Instant>>,
}

pub(crate) fn load_prices(path: &str) -> Option<PriceTable> {
    match PriceTable::load(Path::new(path)) {
        Ok(table) => Some(table),
        Err(error) => {
            tracing::warn!(%error, path, "telemetry price table, using the bundled one");
            PriceTable::parse(BUNDLED_PRICES).ok()
        }
    }
}

/// The text of the price table at `path` when it parses, else the bundled one: what the window's
/// views parse so they name the same table the shell's report does.
pub(crate) fn price_table_text(path: &str) -> String {
    std::fs::read_to_string(path)
        .ok()
        .filter(|text| PriceTable::parse(text).is_ok())
        .unwrap_or_else(|| BUNDLED_PRICES.to_owned())
}

impl Telemetry {
    /// Decides, starts and logs the one `telemetry{state, endpoint}` line. The keychain is read
    /// only for a configured `headers_keychain_item`.
    pub fn start(cfg: &Config, template: std::sync::Arc<CycleTemplate>) -> Self {
        let decided = resolve_start(&cfg.telemetry, |service| {
            secrets::lookup_service(service)
                .map(|found| found.map(|secret| secret.expose().to_owned()))
                .map_err(|error| error.to_string())
        });
        let (report, exporter) = match decided {
            Err(report) => (report, None),
            Ok(start) => {
                let interval = Duration::from_secs(cfg.telemetry.metric_export_interval_s);
                match Exporter::start(&start, interval, &cfg.telemetry.service_name) {
                    Ok(exporter) => {
                        let since = Timestamp::now().to_rfc3339_millis();
                        let switch =
                            Duration::from_secs(cfg.telemetry.mission_switch_min_interval_min * 60);
                        (start.report(&since), Some(exporter.switch_after(switch)))
                    }
                    Err(error) => {
                        tracing::warn!(%error, "telemetry exporter");
                        let mut report = TelemetryReport::exporting(&start.endpoint, "");
                        report.status.enabled = false;
                        report.status.reason = "exporter not built".into();
                        report.state = "off, exporter not built".into();
                        (report, None)
                    }
                }
            }
        };
        tracing::info!(state = %report.state, endpoint = %report.endpoint, "telemetry");
        let prices = exporter
            .is_some()
            .then(|| load_prices(&cfg.metrics.price_table))
            .flatten();
        Self {
            report: Mutex::new(report),
            exporter,
            cfg: cfg.telemetry.clone(),
            template,
            prices,
            last_live: Mutex::new(None),
        }
    }

    pub fn report(&self) -> TelemetryReport {
        self.report
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    /// Folds a mission's events into the instruments; nothing happens while export is off.
    pub fn observe(&self, opened: &OpenedMission) {
        let (Some(exporter), Some(prices)) = (&self.exporter, &self.prices) else {
            return;
        };
        let Some(end) = opened
            .events
            .iter()
            .filter(|event| !event.kind.is_panel_only())
            .map(|event| event.at)
            .max()
        else {
            return;
        };
        let sample = mission_sample(&SampleInput {
            mission: &opened.summary.name,
            events: &opened.events,
            template: &self.template,
            notes: &Notes::default(),
            prices,
            end,
            cfg: &self.cfg,
        });
        exporter.observe(&opened.summary.name, &sample);
    }

    /// True once per export interval while export is on: the ingest thread folds in a mission
    /// when this says so.
    pub fn due_for_live_sample(&self) -> bool {
        if self.exporter.is_none() {
            return false;
        }
        let mut last = self
            .last_live
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let every = Duration::from_secs(self.cfg.metric_export_interval_s);
        if last.is_some_and(|at| at.elapsed() < every) {
            return false;
        }
        *last = Some(Instant::now());
        true
    }

    pub fn stop(&self) {
        if let Some(exporter) = &self.exporter {
            exporter.flush();
            exporter.shutdown();
        }
    }
}

#[cfg(test)]
mod live {
    use std::sync::Arc;
    use std::time::Duration;

    use observatory_core::config::Config;
    use observatory_core::store::Store;

    use super::Telemetry;

    /// Owner-run receipt against the local stack (VAL-TELEM-006 (a), -008): the default endpoint, no
    /// header item, the M0 capture replayed through the store as `open_replay` does, two export
    /// intervals of 15 s. Ignored: it needs the collector on localhost:4318.
    ///
    /// ```sh
    /// OTEL_EXPORTER_OTLP_HEADERS=x=y cargo test -p observatory-app --lib live_export -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore = "needs the local OTLP stack on localhost:4318"]
    fn live_export_reaches_the_local_stack() {
        crate::logging::init();
        let mut cfg = Config::default();
        cfg.telemetry.enabled = true;
        cfg.metrics.price_table = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../config/prices/2026-10-22.toml"
        )
        .to_owned();
        let template = Arc::new(cfg.cycle.load_template().template);
        let telemetry = Telemetry::start(&cfg, template.clone());
        let report = telemetry.report();
        println!("status bar: {}", report.bar_text());
        assert!(report.status.enabled, "{report:?}");

        let dir = tempfile::tempdir().expect("a scratch directory");
        let path = dir.path().join("observatory-live.sqlite");
        let mut store = Store::open(&path).expect("store");
        let capture = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../fixtures/streams/2026-10-22-observatory-v1-m0.jsonl"
        );
        let opened = store
            .replay_file_with(std::path::Path::new(capture), &cfg.ingest, template)
            .expect("the capture replays");
        telemetry.observe(&opened);
        std::thread::sleep(Duration::from_secs(
            2 * cfg.telemetry.metric_export_interval_s + 2,
        ));
        telemetry.stop();
    }
}
