use anyhow::Result;
use itertools::Itertools;
use osmpbfreader::{OsmId, OsmObj, Tags};
use std::collections::{BTreeMap, HashMap};
use std::io;

use crate::util;

pub fn write(relations: &BTreeMap<OsmId, OsmObj>, mut out: impl io::Write) -> Result<()> {
    let mut count_relations = 0;
    let mut count_admin = HashMap::<&str, usize>::new();
    let mut count_boundaries = HashMap::<&str, usize>::new();
    let mut count_tags = HashMap::<&str, usize>::new();
    let mut count_types = HashMap::<&str, usize>::new();

    for obj in relations
        .values()
        .filter(|obj| util::filter_all_relations(obj))
    {
        count_relations += 1;

        let tags = obj.tags();

        add_count(tags, &mut count_admin, "admin_level");
        add_count(tags, &mut count_boundaries, "boundary");
        add_count(tags, &mut count_types, "type");

        for tag in tags
            .keys()
            .filter(|tag| !matches!(tag.as_str(), "boundary" | "type"))
        {
            *count_tags.entry(tag).or_default() += 1;
        }
    }

    write!(
        out,
        "\
Stats
--------------------
Total number of relations: {count_relations}

Administrative levels (count):

{}
Boundary values (count):

{}
Type values (count):

{}
Other tags ({}):

{}",
        to_string(&count_admin),
        to_string(&count_boundaries),
        to_string(&count_types),
        count_tags.len(),
        to_string(&count_tags),
    )?;

    Ok(())
}

fn add_count<'a>(tags: &'a Tags, counts: &mut HashMap<&'a str, usize>, key: &str) {
    if let Some(value) = tags.get(key) {
        *counts.entry(value).or_default() += 1;
    }
}

fn sort_count<'a>(map: &'a HashMap<&'a str, usize>) -> impl Iterator<Item = (&'a &str, &usize)> {
    map.iter().sorted_by(|a, b| Ord::cmp(&b.1, &a.1))
}

fn to_string(map: &HashMap<&str, usize>) -> String {
    let mut out = String::new();

    for (value, count) in sort_count(map) {
        out.push_str(&format!("{value} {count}\n"));
    }

    out
}
