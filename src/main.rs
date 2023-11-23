mod filter;
mod geom;
mod stats;
mod util;

use std::{
    io::{self, stdout, BufWriter, Write},
    path::PathBuf,
};

use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use log::info;
use osmpbfreader::OsmObj;
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

    /// Query for relations with matching name. (Sub)string or pattern allowed.
    #[arg(short, long)]
    query: Option<String>,

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
        if cli.query.is_some() {
            // todo implement --query for stats
            bail!("Sorry, '--query' is not implemented for stats yet.");
        }
        info!("Getting stats");
        stats::write(
            &util::load_relations(
                cli.in_file,
                if all { filter::all } else { filter::by_target },
            )?,
            out,
        )?;
    } else {
        info!("Extracting localities");

        let filter = |obj: &OsmObj| -> bool {
            let query_filter = cli.query.as_ref().map(|query| filter::by_query(query));
            filter::by_target(obj) && query_filter.as_ref().map_or(true, |f| f(obj))
        };

        match cli.format.as_deref() {
            Some("raw") => {
                let objs = util::load_relations(cli.in_file, &filter)?;

                // Use a buffered writer to amortize flushes.
                let mut buffer = BufWriter::new(out);

                for relation in objs.values().filter(|obj| filter(obj)) {
                    writeln!(buffer, "{}", serde_json::to_string(&relation)?)?;
                }
            }
            Some("geojson") | None => {
                geom::write(&util::load_relations(cli.in_file, &filter)?, out)?;
            }
            _ => unreachable!(),
        }
    }

    Ok(())
}
