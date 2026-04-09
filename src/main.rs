//! `pubsub` CLI — in-process harness for the library (demo, fan-out drill, stdin pipeline).

use std::process::ExitCode;

use anyhow::Result;
use clap::{Parser, Subcommand};
use pub_sub_pipeline::art::{HELP_LONG, TAGLINE};
use pub_sub_pipeline::scenarios;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(
    name = "pubsub",
    version,
    about = TAGLINE,
    long_about = HELP_LONG
)]
struct Cli {
    /// Enable `tracing` spans/events (honors `RUST_LOG`, default `info` when unset).
    #[arg(short, long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Run the full scripted report (broadcast → map pipeline → merge).
    Demo,
    /// Read integers from stdin (one per line); print squares to stdout.
    Squares,
    /// Publish `count` messages to an in-memory bus and drain all subscribers.
    Fanout {
        /// Topic name (in-process only).
        #[arg(short, long, default_value = "alerts")]
        topic: String,
        /// Number of subscriber tasks.
        #[arg(short, long, default_value_t = 3)]
        subscribers: usize,
        /// Number of publishes (`0..count`).
        #[arg(short, long, default_value_t = 5)]
        count: u32,
    },
}

fn init_tracing(verbose: bool) -> Result<()> {
    if !verbose {
        return Ok(());
    }

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .compact()
        .try_init()
        .map_err(|e| anyhow::anyhow!("failed to install tracing subscriber: {e}"))?;

    Ok(())
}

async fn dispatch(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Demo => {
            let report = scenarios::run_demo_report().await?;
            print!("{report}");
        }
        Command::Squares => scenarios::run_squares_stdin_stdout().await?,
        Command::Fanout {
            topic,
            subscribers,
            count,
        } => {
            let report = scenarios::run_fanout_report(subscribers, count, &topic).await?;
            print!("{report}");
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    if let Err(e) = init_tracing(cli.verbose) {
        eprintln!("{e:#}");
        return ExitCode::from(1);
    }

    if cli.verbose {
        tracing::info!(target = "pubsub.cli", command = ?cli.command, "starting command");
    }

    match dispatch(cli).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::from(1)
        }
    }
}
