mod app;
mod event;
mod handlers;
mod io_event;
mod network;
mod routes;
mod runner;
mod theme;
mod views;

use clap::Parser;
use std::path::PathBuf;
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
