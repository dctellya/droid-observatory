//! The exporter on the wire (architecture §10, VAL-TELEM-006 to -009): a local listener on
//! 127.0.0.1 stands in for the collector and records what the app sends. The keychain is never
//! read: the header item comes through a stub.

use std::collections::BTreeSet;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::mpsc::{Receiver, channel};
use std::sync::{Mutex, Once};
use std::time::Duration;

use observatory_app_lib::telemetry::Exporter;
use observatory_core::config::Config;
use observatory_core::telemetry::{
    ExporterStart, Label, MissionSample, Point, SeriesPlan, resolve_start,
};
use opentelemetry_proto::tonic::collector::metrics::v1::ExportMetricsServiceRequest;
use prost::Message;

const SECRET_TEXT: &str = "Authorization=Basic YWJj=";

static LOG: Mutex<Vec<u8>> = Mutex::new(Vec::new());

struct LogWriter;

impl Write for LogWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        LOG.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// One process-wide subscriber (the exporter logs from its own thread) and the environment the
/// app must ignore.
fn setup() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        // SAFETY: runs once, before any exporter thread of this process exists.
        unsafe { std::env::set_var("OTEL_EXPORTER_OTLP_HEADERS", "x=y") };
        let _ = tracing_subscriber::fmt()
            .with_writer(|| LogWriter)
            .with_ansi(false)
            .try_init();
    });
}

struct Received {
    request_line: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

/// A collector that answers every request with 200 and hands the requests to the test.
fn listener() -> (String, Receiver<Received>) {
    let socket = TcpListener::bind("127.0.0.1:0").expect("bind");
    let endpoint = format!("http://{}", socket.local_addr().unwrap());
    let (sender, received) = channel();
    std::thread::spawn(move || {
        for stream in socket.incoming() {
            let Ok(mut stream) = stream else { return };
            let mut raw = Vec::new();
            let mut chunk = [0u8; 4096];
            let (head_end, length) = loop {
                let n = stream.read(&mut chunk).unwrap_or(0);
                if n == 0 {
                    break (raw.len(), 0);
                }
                raw.extend_from_slice(&chunk[..n]);
                if let Some(at) = raw.windows(4).position(|w| w == b"\r\n\r\n") {
                    let head = String::from_utf8_lossy(&raw[..at]).to_ascii_lowercase();
                    let length = head
                        .lines()
                        .find_map(|l| l.strip_prefix("content-length:"))
                        .and_then(|v| v.trim().parse::<usize>().ok())
                        .unwrap_or(0);
                    break (at + 4, length);
                }
            };
            while raw.len() < head_end + length {
                let n = stream.read(&mut chunk).unwrap_or(0);
                if n == 0 {
                    break;
                }
                raw.extend_from_slice(&chunk[..n]);
            }
            let head = String::from_utf8_lossy(&raw[..head_end.saturating_sub(4)]).into_owned();
            let mut lines = head.lines();
            let request_line = lines.next().unwrap_or_default().to_owned();
            let headers = lines
                .filter_map(|l| l.split_once(':'))
                .map(|(n, v)| (n.trim().to_ascii_lowercase(), v.trim().to_owned()))
                .collect();
            let body = raw[head_end..].to_vec();
            let _ = stream.write_all(
                b"HTTP/1.1 200 OK\r\ncontent-type: application/x-protobuf\r\ncontent-length: 0\r\nconnection: close\r\n\r\n",
            );
            let _ = sender.send(Received {
                request_line,
                headers,
                body,
            });
        }
    });
    (endpoint, received)
}

fn point(name: &'static str, labels: &[(Label, &str)], value: f64, key: &str) -> Point {
    Point {
        name,
        labels: labels.iter().map(|(l, v)| (*l, (*v).to_owned())).collect(),
        value,
        key: key.to_owned(),
    }
}

/// One point for each of the eleven instruments, with the labels each plans.
fn full_sample() -> MissionSample {
    let m = (Label::Mission, "wire");
    let ms = (Label::Milestone, "m5");
    let rm = [
        m,
        (Label::Role, "worker"),
        (Label::Model, "claude-sonnet-5-5"),
    ];
    MissionSample {
        counters: vec![
            point(
                "observatory.waste_seconds",
                &[m, ms, (Label::Cause, "idle_queued")],
                30.0,
                "",
            ),
            point(
                "observatory.rework_windows",
                &[m, ms, (Label::Cause, "check")],
                1.0,
                "",
            ),
            point("observatory.fix_loops", &[m, ms], 1.0, ""),
            point("observatory.tokens.input", &rm, 100.0, ""),
            point("observatory.tokens.output", &rm, 50.0, ""),
            point("observatory.tokens.cache_read", &rm, 25.0, ""),
            point("observatory.tokens.cache_write", &rm, 5.0, ""),
            point("observatory.cost_usd", &rm, 0.25, ""),
            point("observatory.unknown_events", &[m], 2.0, ""),
        ],
        observations: vec![
            point("observatory.mission.cycle_seconds", &[m], 5400.0, "wire"),
            point("observatory.feature.cycle_seconds", &[m], 900.0, "f1"),
        ],
    }
}

fn start(item: Option<&str>, endpoint: &str) -> ExporterStart {
    let mut cfg = Config::default().telemetry;
    cfg.enabled = true;
    cfg.endpoint = endpoint.to_owned();
    if let Some(item) = item {
        cfg.headers_keychain_item = item.to_owned();
    }
    resolve_start(&cfg, |_| Ok(Some(SECRET_TEXT.to_owned()))).expect("the exporter starts")
}

fn first_post(start: &ExporterStart, received: &Receiver<Received>) -> Received {
    let exporter = Exporter::start(start, Duration::from_secs(60), "droid-observatory")
        .expect("the exporter builds");
    exporter.observe("wire", &full_sample());
    exporter.flush();
    let first = received
        .recv_timeout(Duration::from_secs(10))
        .expect("the collector received a POST");
    exporter.shutdown();
    first
}

#[test]
fn header_is_sent_once_split_on_the_first_equals_and_the_environment_is_ignored() {
    setup();
    let (endpoint, received) = listener();
    let sent = first_post(
        &start(Some("droid-observatory/otlp-headers-stub"), &endpoint),
        &received,
    );

    assert!(
        sent.request_line.starts_with("POST /v1/metrics "),
        "{}",
        sent.request_line
    );
    let auth: Vec<_> = sent
        .headers
        .iter()
        .filter(|(n, _)| n == "authorization")
        .collect();
    assert_eq!(
        auth.len(),
        1,
        "exactly one Authorization header: {:?}",
        sent.headers
    );
    assert_eq!(auth[0].1, "Basic YWJj=");
    assert!(
        sent.headers.iter().all(|(n, _)| n != "x"),
        "OTEL_EXPORTER_OTLP_HEADERS is not read: {:?}",
        sent.headers
    );
    let names: Vec<&str> = sent.headers.iter().map(|(n, _)| n.as_str()).collect();
    println!("received headers: {names:?}");
}

#[test]
fn an_exporter_without_an_item_sends_no_header_of_its_own() {
    setup();
    let (endpoint, received) = listener();
    let sent = first_post(&start(None, &endpoint), &received);
    assert!(
        sent.headers
            .iter()
            .all(|(n, _)| n != "authorization" && n != "x")
    );
}

#[test]
fn exactly_the_eleven_instruments_reach_the_wire_without_units() {
    setup();
    let (endpoint, received) = listener();
    let sent = first_post(&start(None, &endpoint), &received);
    let request = ExportMetricsServiceRequest::decode(sent.body.as_slice()).expect("protobuf");

    let cfg = Config::default().telemetry;
    let plan = SeriesPlan::from(&cfg);
    let mut names = BTreeSet::new();
    for resource in &request.resource_metrics {
        let attributes: BTreeSet<&str> = resource
            .resource
            .as_ref()
            .expect("a resource")
            .attributes
            .iter()
            .map(|kv| kv.key.as_str())
            .collect();
        assert_eq!(
            attributes,
            BTreeSet::from(["service.name", "service.version"]),
            "no resource attribute beyond the two"
        );
        for scope in &resource.scope_metrics {
            for metric in &scope.metrics {
                assert_eq!(metric.unit, "", "{} has no unit", metric.name);
                names.insert(metric.name.clone());
                let is_histogram = matches!(
                    metric.data,
                    Some(opentelemetry_proto::tonic::metrics::v1::metric::Data::Histogram(_))
                );
                let planned = plan.instruments.iter().find(|i| i.name == metric.name);
                let want_histogram = planned.is_some_and(|i| {
                    matches!(
                        i.kind,
                        observatory_core::telemetry::InstrumentKind::Histogram { .. }
                    )
                });
                assert_eq!(is_histogram, want_histogram, "{} kind", metric.name);
                if let Some(opentelemetry_proto::tonic::metrics::v1::metric::Data::Histogram(h)) =
                    &metric.data
                {
                    let bounds = &h.data_points[0].explicit_bounds;
                    let want = match &planned.unwrap().kind {
                        observatory_core::telemetry::InstrumentKind::Histogram { buckets } => {
                            buckets.iter().map(|b| *b as f64).collect::<Vec<_>>()
                        }
                        observatory_core::telemetry::InstrumentKind::Counter => unreachable!(),
                    };
                    assert_eq!(bounds, &want, "{} buckets", metric.name);
                }
            }
        }
    }
    let want: BTreeSet<String> = cfg.mission_metrics.iter().cloned().collect();
    assert_eq!(
        names, want,
        "exactly the eleven names of [telemetry].mission_metrics"
    );
    println!("instruments on the wire: {names:?}");
}

#[test]
fn the_app_log_has_the_post_status_and_never_the_header() {
    setup();
    let (endpoint, received) = listener();
    let _ = first_post(
        &start(Some("droid-observatory/otlp-headers-stub"), &endpoint),
        &received,
    );
    let log = String::from_utf8_lossy(&LOG.lock().unwrap()).into_owned();
    assert!(log.contains("telemetry.post"), "{log}");
    assert!(log.contains("status=200"), "{log}");
    assert!(log.contains("/v1/metrics"), "{log}");
    for leak in ["Authorization=", "Basic ", "YWJj", "authorization"] {
        assert!(!log.contains(leak), "{leak:?} in the app log:\n{log}");
    }
}
