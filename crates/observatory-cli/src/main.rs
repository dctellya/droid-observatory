mod replay;

use std::io::{self, BufWriter, ErrorKind, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use observatory_core::config::Config;

#[derive(Parser)]
#[command(version, about = "Headless replay and inspection of Droid sessions")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Print one line per event of a session file, then `unknown events: N`.
    Replay {
        /// The orchestrator session `.jsonl`.
        file: PathBuf,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli.command) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) if is_broken_pipe(err.as_ref()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("observatory-cli: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run(command: Command) -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::load(None)?;
    match command {
        Command::Replay { file } => {
            let mut out = BufWriter::new(io::stdout().lock());
            replay::replay(&file, &config.ingest, &mut out)?;
            out.flush()?;
            Ok(())
        }
    }
}

/// `observatory-cli replay x | head` closes the pipe early; that is not a failure.
fn is_broken_pipe(err: &(dyn std::error::Error + 'static)) -> bool {
    err.downcast_ref::<io::Error>()
        .is_some_and(|err| err.kind() == ErrorKind::BrokenPipe)
}
