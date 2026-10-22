use std::process::ExitCode;

use observatory_core::config::Config;

fn main() -> ExitCode {
    if let Err(err) = Config::load(None) {
        eprintln!("observatory-cli: {err}");
        return ExitCode::FAILURE;
    }
    println!(
        "observatory-cli {}: no commands yet",
        env!("CARGO_PKG_VERSION")
    );
    ExitCode::SUCCESS
}
