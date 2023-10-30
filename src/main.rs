use std::{collections::HashMap, env::args};

use anyhow::{anyhow, Result};

fn main() -> Result<()> {
    let path = args()
        .nth(1)
        .ok_or_else(|| anyhow!("expected input path as first argument"))?;

    let f = std::fs::File::open(path)?;

    let mut pbf = osmpbfreader::OsmPbfReader::new(f);
    let objs = pbf.get_objs_and_deps(|obj| {
        obj.is_relation()
            && obj.tags().contains("boundary", "administrative")
            && obj.tags().contains_key("name")
            && {
                if let Some(admin_level) = obj.tags().get("admin_level") {
                    admin_level == "2"
                        || admin_level == "4"
                        || admin_level == "6"
                        || admin_level == "7"
                        || admin_level == "8"
                } else {
                    false
                }
            }
    })?;

    let mut tags = HashMap::new();

    for obj in objs.values() {
        if obj.is_relation() {
            // println!("{id:?}: {obj:?} {:?}", name);
            let _ = obj
                .tags()
                .iter()
                .filter(|(k, _)| !k.contains(':'))
                .map(|(k, _)| {
                    if let Some(tag) = tags.get_mut(k) {
                        *tag += 1;
                    } else {
                        tags.insert(k, 1);
                    }
                })
                .collect::<Vec<_>>();
        }
    }

    dbg!(tags);

    Ok(())
}
