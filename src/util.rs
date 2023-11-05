use anyhow::Result;
use osmpbfreader::{OsmId, OsmObj, OsmPbfReader};
use std::{collections::BTreeMap, path::PathBuf};

const TARGET_BOUNDARY_TYPES: &[&str] = &[
    "administrative",
    "state_border",
    "country_border",
    "state border",
];

pub fn filter_target_relations(obj: &OsmObj) -> bool {
    filter_all_relations(obj)
        && obj.tags().get("boundary").map_or(false, |boundary| {
            TARGET_BOUNDARY_TYPES.contains(&boundary.as_str())
        })
}

pub fn filter_all_relations(obj: &OsmObj) -> bool {
    obj.is_relation()
        && obj.tags().contains_key("name")
        && obj.tags().get("admin_level").map_or(false, |admin_level| {
            matches!(admin_level.as_str(), "2" | "4" | "6" | "7" | "8")
        })
}

pub fn load_relations<F>(path: PathBuf, pred: F) -> Result<BTreeMap<OsmId, OsmObj>>
where
    F: FnMut(&OsmObj) -> bool,
{
    let f = std::fs::File::open(path)?;
    let mut pbf = OsmPbfReader::new(f);
    let relations = pbf.get_objs_and_deps(pred)?;
    Ok(relations)
}
