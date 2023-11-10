use anyhow::Result;
use osmpbfreader::{OsmId, OsmObj, OsmPbfReader};
use std::{
    collections::BTreeMap,
    hash::{Hash, Hasher},
    path::PathBuf,
};

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
