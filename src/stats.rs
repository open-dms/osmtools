use anyhow::Result;
use itertools::Itertools;
use osmpbfreader::OsmId;
use osmpbfreader::OsmObj;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::io;

use crate::util::filter_all_relations;

pub fn to_stats(relations: &BTreeMap<OsmId, OsmObj>, mut out: impl io::Write) -> Result<()> {
    let mut boundary_types = HashMap::<&str, usize>::new();

    for boundary in relations
        .values()
        .filter(|obj| filter_all_relations(obj))
        .filter_map(|obj| obj.tags().get("boundary"))
    {
        *boundary_types.entry(boundary).or_default() += 1;
    }

    for (boundary_type, count) in boundary_types.iter().sorted_by(|a, b| Ord::cmp(&b.1, &a.1)) {
        writeln!(out, "{boundary_type} {count}")?;
    }

    Ok(())
}
