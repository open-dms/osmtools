mod locality;
mod stats;
mod util;

use std::{
    io::{self, stdout},
    path::PathBuf,
};

use anyhow::Result;
use clap::{Parser, Subcommand};
use log::info;
use simple_logger::SimpleLogger;

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
    Stats {
        /// Path to output file. If unspecified output is written to stdout.
        #[arg(short, long)]
        all: bool,
    },
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

    if let Some(Commands::Stats { all }) = cli.command {
        info!("Getting stats");
        stats::write(
            &util::load_relations(
                cli.in_file,
                if all {
                    util::filter_all_relations
                } else {
                    util::filter_target_relations
                },
            )?,
            out,
        )?;
    } else {
        info!("Extracting localities");
        locality::write(
            &util::load_relations(cli.in_file, util::filter_target_relations)?,
            out,
        )?;
    }

    Ok(())
}
