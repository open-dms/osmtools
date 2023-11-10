mod geojson;
mod stats;
mod util;

use std::{
    io::{self, stdout, BufWriter, Write},
    path::PathBuf,
};

use anyhow::{bail, Result};
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

    /// Output format.
    #[arg(short, long, value_parser=["geojson", "raw"], default_value = "geojson")]
    format: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Output statistics about the PBF file
    Stats {
        /// Show stats for all relations, using minimal filters.
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
        match cli.format.as_deref() {
            Some("raw") => {
                let objs = util::load_relations(cli.in_file, util::filter_target_relations)?;

                // Use a buffered writer to amortize flushes.
                let mut buffer = BufWriter::new(out);

                for relation in objs
                    .values()
                    .filter(|obj| util::filter_target_relations(obj))
                {
                    writeln!(buffer, "{}", serde_json::to_string(&relation)?)?;
                }
            }
            Some("geojson") | None => {
                geojson::write(
                    &util::load_relations(cli.in_file, util::filter_target_relations)?,
                    out,
                )?;
            }
            _ => unreachable!(),
        }
    }

    Ok(())
}
