use clap::Parser;

use crate::model::SortField;

#[derive(Debug, Clone, Parser)]
#[command(
    name = "system-monitor",
    version,
    about = "A top-like system monitor that starts in the terminal and is designed to move into the browser later."
)]
pub struct Cli {
    #[arg(
        short = 'i',
        long = "interval",
        default_value_t = 1000,
        value_name = "MS",
        value_parser = clap::value_parser!(u64).range(100..)
    )]
    pub interval_ms: u64,

    #[arg(short, long, value_enum, default_value_t = SortField::Cpu)]
    pub sort: SortField,

    #[arg(long, help = "Sort in ascending order instead of descending.")]
    pub ascending: bool,

    #[arg(short, long, value_name = "TEXT")]
    pub filter: Option<String>,

    #[arg(short, long, default_value_t = 25, value_name = "COUNT")]
    pub limit: usize,

    #[arg(long, help = "Render a single snapshot as plain text and exit.")]
    pub once: bool,

    #[arg(long, help = "Emit a single snapshot as JSON and exit.")]
    pub json: bool,
}
