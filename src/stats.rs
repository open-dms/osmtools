use anyhow::Result;
use itertools::Itertools;
use osmpbfreader::{OsmId, OsmObj, Tags};
use std::collections::{BTreeMap, HashMap};
use std::io;

use crate::util;

pub fn write(relations: &BTreeMap<OsmId, OsmObj>, mut out: impl io::Write) -> Result<()> {
    writeln!(out, "Stats\n--------------------")?;
    writeln!(out, "total relations: {}", relations.len())?;

    let mut tags_count = HashMap::<&str, usize>::new();
    let mut boundaries_count = HashMap::<&str, usize>::new();
    let mut types_count = HashMap::<&str, usize>::new();

    for obj in relations
        .values()
        .filter(|obj| util::filter_all_relations(obj))
    {
        let tags = obj.tags();

        add_count(tags, &mut boundaries_count, "boundary");
        add_count(tags, &mut types_count, "type");

        for tag in tags.keys().filter(|tag| tag.as_str() != "boundary") {
            *tags_count.entry(tag).or_default() += 1;
        }
    }

    writeln!(out, "Boundary values (count):\n")?;

    for (value, count) in sort_count(&boundaries_count) {
        writeln!(out, "{value} {count}")?;
    }

    writeln!(out, "\nType values (count):\n")?;

    for (value, count) in sort_count(&types_count) {
        writeln!(out, "{value} {count}")?;
    }

    writeln!(out, "\nOther tags (count):\n")?;

    for (tag, count) in sort_count(&tags_count) {
        writeln!(out, "{tag} {count}")?;
    }

    Ok(())
}

fn add_count<'a>(tags: &'a Tags, counts: &mut HashMap<&'a str, usize>, key: &str) {
    if let Some(value) = tags.get(key) {
        *counts.entry(value).or_default() += 1;
    }
}

fn sort_count<'a>(
    map: &'a HashMap<&'a str, usize>,
) -> impl Iterator<Item = (&'a &'a str, &'a usize)> {
    map.iter().sorted_by(|a, b| Ord::cmp(&b.1, &a.1))
}
