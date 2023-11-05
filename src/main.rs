use crate::{
    locality::to_jsonl,
    stats::to_stats,
    util::{filter_all_relations, filter_target_relations, load_relations},
};
use std::{
    io::{self, stdout},
    path::PathBuf,
};

use anyhow::Result;
use clap::{Parser, Subcommand};
use log::info;
use simple_logger::SimpleLogger;

pub mod locality;
pub mod stats;
pub mod util;

#[derive(Parser)]
struct Cli {
    /// PBF file to read.
    #[arg(short, long)]
    in_file: PathBuf,

    /// Path to output file. If unspecified output is written to stdout.
    #[arg(short, long)]
    out_file: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Output statistics about the PBF file
    Stats,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    SimpleLogger::new()
        .with_level(log::LevelFilter::Info)
        .init()?;

    info!("Unpacking relations from {:?}", cli.in_file);

    let out: Box<dyn io::Write> = if let Some(f) = cli.out_file {
        let f = std::fs::File::create(f)?;
        Box::new(f)
    } else {
        Box::new(stdout())
    };

    if let Some(Commands::Stats) = cli.command {
        to_stats(load_relations(cli.in_file, filter_all_relations)?, out)?;
    } else {
        to_jsonl(load_relations(cli.in_file, filter_target_relations)?, out)?;
    }

    Ok(())
}
