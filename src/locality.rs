use std::{
    collections::BTreeMap,
    io::Write,
    io::{self, BufWriter},
};

use anyhow::Result;
use osmpbfreader::{OsmId, OsmObj};
use serde_json::to_string;

use crate::util::filter_target_relations;

pub fn to_jsonl(relations: BTreeMap<OsmId, OsmObj>, out: impl io::Write) -> Result<()> {
    // Use a buffered writer to amortize flushes.
    let mut buffer = BufWriter::new(out);

    for relation in relations
        .values()
        .filter(|obj| filter_target_relations(obj))
    {
        let serialized = to_string(&relation)?;
        writeln!(buffer, "{serialized}")?;
    }

    Ok(())
}
