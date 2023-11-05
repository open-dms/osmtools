use anyhow::Result;
use itertools::Itertools;
use osmpbfreader::OsmId;
use osmpbfreader::OsmObj;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::io;
use std::vec::IntoIter;

use crate::util::filter_all_relations;

fn sort_count(map: HashMap<&str, usize>) -> IntoIter<(&str, usize)> {
    map.into_iter()
        .sorted_by(|a, b| Ord::cmp(&b.1, &a.1))
        .collect::<Vec<_>>()
        .into_iter()
}

pub fn to_stats(relations: BTreeMap<OsmId, OsmObj>, mut out: impl io::Write) -> Result<()> {
    writeln!(out, "Stats\n{}", "-".repeat(20))?;
    writeln!(out, "total relations: {}", relations.len())?;

    let mut boundary_types = HashMap::<&str, usize>::new();
    let mut tags = HashMap::<&str, usize>::new();

    for obj in relations.values().filter(|obj| filter_all_relations(obj)) {
        if let Some(boundary) = obj.tags().get("boundary") {
            *boundary_types.entry(boundary).or_default() += 1;
        }

        for tag in obj
            .tags()
            .keys()
            .filter_map(|tag| if tag != "boundary" { Some(tag) } else { None })
        {
            *tags.entry(tag).or_default() += 1;
        }
    }

    writeln!(out, "\nBoundary types (count):\n")?;

    for (boundary_type, count) in sort_count(boundary_types) {
        writeln!(out, "{boundary_type} {count}")?;
    }

    writeln!(out, "\nOther tags (count):\n")?;

    for (tag, count) in sort_count(tags) {
        writeln!(out, "{tag} {count}")?;
    }

    Ok(())
}
