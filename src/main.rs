use std::env::args;

use anyhow::{anyhow, Result};

fn main() -> Result<()> {
    let path = args()
        .nth(1)
        .ok_or_else(|| anyhow!("expected input path as first argument"))?;

    let f = std::fs::File::open(path)?;

    let mut pbf = osmpbfreader::OsmPbfReader::new(f);
    let objs = pbf.get_objs_and_deps(|obj| obj.is_way() && obj.tags().contains_key("highway"))?;
    for (id, obj) in &objs {
        if obj.is_relation() {
            println!("{id:?}: {obj:?}");
        }
    }

    Ok(())
}
