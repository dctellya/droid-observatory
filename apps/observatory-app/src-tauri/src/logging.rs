use std::fs::OpenOptions;
use std::path::Path;
use std::sync::Mutex;

use tracing_subscriber::field::MakeExt;
use tracing_subscriber::filter::filter_fn;
use tracing_subscriber::fmt::format::debug_fn;
use tracing_subscriber::fmt::time::UtcTime;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{EnvFilter, Layer, Registry, fmt};

/// Where the host writes its latency intervals as JSON lines; unset, nothing is written.
pub const TIMING_LOG_ENV: &str = "OBSERVATORY_TIMING_LOG";

/// The `tracing` target of an interval the e2e suite reads (VAL-E2E-003).
pub const TIMING_TARGET: &str = "observatory::timing";

/// Fields the app log never prints, whatever a call site passes (architecture §12, invariant I7).
const BLOCKED_FIELDS: [&str; 4] = ["authorization", "token", "header", "secret"];

pub(crate) fn field_is_blocked(name: &str) -> bool {
    BLOCKED_FIELDS
        .iter()
        .any(|blocked| name.eq_ignore_ascii_case(blocked))
}

/// INFO lines on stdout with a UTC timestamp: the app log the validators read (architecture §12).
/// With `OBSERVATORY_TIMING_LOG` set, the `observatory::timing` events also go to that file as
/// JSON lines.
pub fn init() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let fields = debug_fn(|writer, field, value| match field.name() {
        name if field_is_blocked(name) => Ok(()),
        "message" => write!(writer, "{value:?}"),
        name => write!(writer, "{name}={value:?}"),
    })
    .delimited(" ");
    let stdout = fmt::layer()
        .fmt_fields(fields)
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

#[cfg(test)]
mod tests {
    use super::field_is_blocked;

    #[test]
    fn fields_named_like_secrets_are_dropped_from_the_app_log() {
        for name in [
            "authorization",
            "token",
            "header",
            "secret",
            "Authorization",
        ] {
            assert!(field_is_blocked(name), "{name}");
        }
        for name in ["state", "endpoint", "status", "url", "message"] {
            assert!(!field_is_blocked(name), "{name}");
        }
    }
}
