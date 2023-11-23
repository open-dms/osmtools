use anyhow::Result;
use osmpbfreader::{OsmId, OsmObj, OsmPbfReader};
use std::{collections::BTreeMap, path::PathBuf};

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
