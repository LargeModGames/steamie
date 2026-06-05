mod app;
mod event;
mod handlers;
mod io_event;
mod network;
mod protocol;
mod routes;
mod runner;
mod theme;
mod views;

use clap::Parser;
use std::{fs, path::PathBuf};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};
use vapour_core::Config;

#[derive(Parser)]
#[command(name = "vapour", version, about = "A terminal Steam client")]
struct Cli {
    /// Path to config file (default: ~/.config/vapour/config.toml)
    #[arg(short, long)]
    config: Option<PathBuf>,
}

#[tokio::main]
async fn main() {
    let _log_guard = init_tracing();
    let cli = Cli::parse();

    let config = match cli.config {
        Some(path) => Config::load_from(path),
        None => Config::load(),
    };

    let config = match config {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };

    if let Err(e) = runner::run(config).await {
        eprintln!("vapour: {e}");
        std::process::exit(1);
    }
}

fn init_tracing() -> Option<tracing_appender::non_blocking::WorkerGuard> {
    let state_dir = dirs::state_dir()
        .or_else(dirs::config_dir)
        .or_else(|| dirs::home_dir().map(|h| h.join(".config")))?
        .join("vapour");
    fs::create_dir_all(&state_dir).ok()?;

    let file_appender = tracing_appender::rolling::never(state_dir, "vapour.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::builder()
            .with_default_directive(LevelFilter::INFO.into())
            .from_env_lossy()
    });

    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            fmt::layer()
                .with_ansi(false)
                .with_writer(non_blocking)
                .with_target(true)
                .with_file(true)
                .with_line_number(true),
        )
        .init();

    Some(guard)
}
