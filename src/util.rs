use anyhow::Result;
use osmpbfreader::{OsmId, OsmObj, OsmPbfReader};
use std::{
    collections::BTreeMap,
    hash::{Hash, Hasher},
    path::PathBuf,
};

/// Filter for relations having name and a range of `admin_level`.
pub fn filter_all_relations(obj: &OsmObj) -> bool {
    obj.is_relation()
}

/// Filter for relations. In addition to `filter_all_relations`, add boundary types.
pub fn filter_target_relations(obj: &OsmObj) -> bool {
    let tags = obj.tags();
    filter_all_relations(obj)
        && tags.contains_key("name")
        && tags
            .get("type")
            .map_or(false, |value| matches!(value.as_str(), "boundary"))
        && tags
            .get("boundary")
            .map_or(false, |value| matches!(value.as_str(), "administrative"))
        && tags.contains_key("de:regionalschluessel")
        && tags.get("admin_level").map_or(false, |admin_level| {
            matches!(admin_level.as_str(), "2" | "4" | "6" | "7" | "8")
        })
}

/// Load PBF file from `path` and filter contents using `pred`.
pub fn load_relations<F>(path: PathBuf, pred: F) -> Result<BTreeMap<OsmId, OsmObj>>
where
    F: FnMut(&OsmObj) -> bool,
{
    let f = std::fs::File::open(path)?;
    let mut pbf = OsmPbfReader::new(f);
    let relations = pbf.get_objs_and_deps(pred)?;
    Ok(relations)
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct FloatTuple(pub f64, pub f64);

impl Hash for FloatTuple {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Convert each floating-point number to its raw bits for hashing.
        // This is a common way to handle floating-point numbers when you need to hash them.
        self.0.to_bits().hash(state);
        self.1.to_bits().hash(state);
    }
}

impl Eq for FloatTuple {} // No additional methods needed, just equality checks.
