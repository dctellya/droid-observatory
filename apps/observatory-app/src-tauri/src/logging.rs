use std::fs::OpenOptions;
use std::path::Path;
use std::sync::Mutex;

use tracing_subscriber::filter::filter_fn;
use tracing_subscriber::fmt::time::UtcTime;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{EnvFilter, Layer, Registry, fmt};

/// Where the host writes its latency intervals as JSON lines; unset, nothing is written.
pub const TIMING_LOG_ENV: &str = "OBSERVATORY_TIMING_LOG";

/// The `tracing` target of an interval the e2e suite reads (VAL-E2E-003).
pub const TIMING_TARGET: &str = "observatory::timing";

/// INFO lines on stdout with a UTC timestamp: the app log the validators read (architecture §12).
/// With `OBSERVATORY_TIMING_LOG` set, the `observatory::timing` events also go to that file as
/// JSON lines.
pub fn init() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let stdout = fmt::layer()
        .with_timer(UtcTime::rfc_3339())
        .with_writer(std::io::stdout)
        .with_filter(filter);
    let timing = std::env::var_os(TIMING_LOG_ENV).and_then(|path| timing_layer(Path::new(&path)));
    let _ = Registry::default().with(timing).with(stdout).try_init();
}

fn timing_layer(path: &Path) -> Option<impl Layer<Registry> + use<>> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).ok()?;
    }
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .ok()?;
    Some(
        fmt::layer()
            .json()
            .with_writer(Mutex::new(file))
            .with_filter(filter_fn(|meta| meta.target() == TIMING_TARGET)),
    )
}

/// One latency interval, in milliseconds, for the timing log.
pub fn interval(name: &str, ms: f64, count: usize) {
    tracing::info!(target: "observatory::timing", interval = name, ms, count);
}
