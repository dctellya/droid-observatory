use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::time::UtcTime;

/// INFO lines on stdout with a UTC timestamp: the app log the validators read (architecture §12).
pub fn init() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_timer(UtcTime::rfc_3339())
        .with_writer(std::io::stdout)
        .try_init();
}
