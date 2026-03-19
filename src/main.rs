use clap::Parser;
use system_monitor::{app, cli::Cli};

fn main() -> system_monitor::AppResult<()> {
    let cli = Cli::parse();
    app::run(cli)
}
