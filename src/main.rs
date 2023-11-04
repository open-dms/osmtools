use std::{
    collections::{BTreeMap, HashMap},
    io::{stdout, BufWriter, Write},
    path::PathBuf,
};

use anyhow::Result;
use clap::{Parser, Subcommand};
use itertools::Itertools;
use osmpbfreader::{OsmId, OsmObj, OsmPbfReader};
use serde_json::to_string;

const TARGET_BOUNDARY_TYPES: &[&str] = &[
    "administrative",
    "state_border",
    "country_border",
    "state border",
];

#[derive(Parser)]
#[command(name = "osmpbf-filter")]
#[command(bin_name = "osmpbf-filter")]
struct Cli {
    #[arg(short, long)]
    in_file: PathBuf,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Stats,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    eprintln!("Unpacking relations from {:?}", cli.in_file);

    match &cli.command {
        Some(Commands::Stats) => {
            let relations = load_relations(cli.in_file, filter_all_relations)?;
            eprintln!("Gathering some stats..");
            to_stats(&relations);
        }
        None => {
            let relations = load_relations(cli.in_file, filter_target_relations)?;
            to_jsonl(&relations)?;
        }
    }

    Ok(())
}

fn filter_target_relations(obj: &OsmObj) -> bool {
    filter_all_relations(obj)
        && obj.tags().get("boundary").map_or(false, |boundary| {
            TARGET_BOUNDARY_TYPES.contains(&boundary.as_str())
        })
}

fn filter_all_relations(obj: &OsmObj) -> bool {
    obj.is_relation()
        && obj.tags().contains_key("name")
        && obj.tags().get("admin_level").map_or(false, |admin_level| {
            matches!(admin_level.as_str(), "2" | "4" | "6" | "7" | "8")
        })
}

fn load_relations<F>(path: PathBuf, pred: F) -> Result<BTreeMap<OsmId, OsmObj>>
where
    F: FnMut(&OsmObj) -> bool,
{
    let f = std::fs::File::open(path)?;
    let mut pbf = OsmPbfReader::new(f);
    let relations = pbf.get_objs_and_deps(pred)?;
    Ok(relations)
}

fn to_stats(relations: &BTreeMap<OsmId, OsmObj>) {
    let mut boundary_types = HashMap::<&str, usize>::new();

    for boundary in relations
        .values()
        .filter(|obj| filter_all_relations(obj))
        .filter_map(|obj| obj.tags().get("boundary"))
    {
        *boundary_types.entry(boundary).or_default() += 1;
    }

    for (boundary_type, count) in boundary_types.iter().sorted_by(|a, b| Ord::cmp(&b.1, &a.1)) {
        println!("{boundary_type} {count}");
    }
}

fn to_jsonl(relations: &BTreeMap<OsmId, OsmObj>) -> Result<()> {
    let mut buffer = BufWriter::new(stdout());

    for relation in relations
        .values()
        .filter(|obj| filter_target_relations(obj))
    {
        let serialized = to_string(&relation)?;
        writeln!(buffer, "{serialized}")?;
    }

    buffer.flush()?;

    Ok(())
}
