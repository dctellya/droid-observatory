mod inspect;
mod replay;

use std::io::{self, BufWriter, ErrorKind, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use observatory_core::config::Config;
use observatory_core::store::Store;

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
        /// Replay only this file, not the child sessions found beside it.
        #[arg(long)]
        root_only: bool,
        /// Join the turns of this Droid log file (`droid-log-single.log`) to the sessions and
        /// print them as `turn.usage` lines with per-role totals; repeat the flag for rotated
        /// files, oldest first. The files are only read.
        #[arg(long, value_name = "FILE")]
        usage_log: Vec<PathBuf>,
        /// Write the sessions and event pointers to this SQLite store. Without it, the
        /// store at `[store].path` is written when a config file is in effect
        /// (`OBSERVATORY_CONFIG` or `~/.droid-observatory/config.toml`), else none.
        #[arg(long, value_name = "PATH")]
        store: Option<PathBuf>,
    },
    /// Print the raw line an event was mapped from, found by its stored pointer.
    Inspect {
        /// Event id, `<session id>:<byte offset>:<sub>`.
        event_id: String,
        /// The SQLite store to read; default `[store].path`.
        #[arg(long, value_name = "PATH")]
        store: Option<PathBuf>,
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
    let (config, config_file) = Config::load_with_origin(None)?;
    match command {
        Command::Replay {
            file,
            root_only,
            usage_log,
            store,
        } => {
            // Pointers outlive the working directory only when they are absolute.
            let file = std::path::absolute(&file)?;
            let usage_log = usage_log
                .iter()
                .map(std::path::absolute)
                .collect::<Result<Vec<_>, _>>()?;
            let store_path =
                store.or_else(|| config_file.is_some().then(|| config.store.resolved_path()));
            let mut store = store_path.as_deref().map(Store::open).transpose()?;
            let mut out = BufWriter::new(io::stdout().lock());
            replay::replay(
                &file,
                root_only,
                &usage_log,
                &config.ingest,
                store.as_mut(),
                &mut out,
                &mut io::stderr().lock(),
            )?;
            out.flush()?;
            Ok(())
        }
        Command::Inspect { event_id, store } => {
            let store = store.unwrap_or_else(|| config.store.resolved_path());
            let mut out = BufWriter::new(io::stdout().lock());
            inspect::inspect(&store, &event_id, &mut out)?;
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
