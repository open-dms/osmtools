use std::{
    collections::{BTreeMap, HashMap, HashSet},
    io::Write,
    io::{self, BufWriter},
};

use anyhow::{bail, Context, Result};
use geojson::{self, GeoJson, Geometry};
use osmpbfreader::{OsmId, OsmObj, Ref, Way};
use serde_json::json;

use crate::util::{self, FloatTuple};

pub fn write(objs: &BTreeMap<OsmId, OsmObj>, out: impl io::Write) -> Result<()> {
    // Use a buffered writer to amortize flushes.
    let mut buffer = BufWriter::new(out);

    for relation in objs
        .values()
        .filter(|obj| util::filter_target_relations(obj))
    {
        if let Some(feature) = to_feature(relation, objs) {
            let serialized = feature.to_string();
            writeln!(buffer, "{serialized}")?;
        }
    }

    Ok(())
}

fn to_feature(obj: &OsmObj, all_objs: &BTreeMap<OsmId, OsmObj>) -> Option<geojson::GeoJson> {
    let tags = obj.tags();
    let name = tags.get("name")?;
    let admin_level = tags.get("admin_level")?;
    let ars = tags.get("de:regionalschluessel")?;

    let serde_json::Value::Object(properties) = json!({
        "adminLevel":admin_level.parse::<u8>().ok()?,
        "ars": ars,
        "name": name,
        "osm:id": obj.relation()?.id.0,
        "osm:type": "relation",
    }) else {
        return None;
    };

    Some(GeoJson::Feature(geojson::Feature {
        geometry: Some(Geometry::new(as_polygon(obj, all_objs)?)),
        id: Some(geojson::feature::Id::Number(
            serde_json::value::Number::from(obj.relation()?.id.0),
        )),
        properties: Some(properties),
        ..geojson::Feature::default()
    }))
}

fn as_polygon(obj: &OsmObj, all_objs: &BTreeMap<OsmId, OsmObj>) -> Option<geojson::Value> {
    let to_coords = |way: &Way| -> Option<Vec<geojson::Position>> {
        way.nodes
            .iter()
            .map(|node_id| {
                let node = all_objs.get(&OsmId::Node(*node_id))?;
                Some(vec![
                    f64::from(node.node()?.decimicro_lon) / 10_000_000.0,
                    f64::from(node.node()?.decimicro_lat) / 10_000_000.0,
                ])
            })
            .collect()
    };

    let linestrings = obj
        .relation()?
        .refs
        .iter()
        .filter_map(|child: &Ref| {
            // todo treat 'inner' and contained relations as well
            if matches!(child.role.as_str(), "outer") {
                Some(to_coords(all_objs.get(&child.member)?.way()?)?)
            } else {
                None
            }
        })
        .collect();

    // todo report missing geometry or broken linering
    let mut linering = create_continuous_linering(&linestrings).ok()?;

    // respect right hand rule
    if is_clockwise(&linering) {
        linering.reverse();
    }

    Some(geojson::Value::Polygon(vec![linering]))
}

/// create a continuous ring from line strings
fn create_continuous_linering(
    linestrings: &Vec<Vec<geojson::Position>>,
) -> Result<Vec<geojson::Position>> {
    if linestrings.is_empty() || linestrings.iter().any(|ls| ls.len() < 2) {
        bail!("No linestrings or a linestring has less than 2 positions")
    }

    // Convert the endpoint positions to a hashable type (tuple) and build the index map
    let mut endpoints: HashMap<FloatTuple, Vec<usize>> = HashMap::new();
    for (i, linestring) in linestrings.iter().enumerate() {
        let start = FloatTuple(
            linestring.first().context("Empty linestring")?[0],
            linestring.first().unwrap()[1],
        );
        let end = FloatTuple(linestring.last().unwrap()[0], linestring.last().unwrap()[1]);
        endpoints.entry(start).or_default().push(i);
        if start != end {
            // Avoid double entry for loops
            endpoints.entry(end).or_default().push(i);
        }
    }

    // Start from the first linestring
    let first_index = 0;
    let mut continuous_line = linestrings[first_index].clone();

    // Track used linestrings to prevent infinite loops
    let mut used_linestrings = HashSet::new();
    used_linestrings.insert(first_index);

    while used_linestrings.len() < linestrings.len() {
        let current_end_key = {
            let x = continuous_line.last().context("Empty line ring")?;
            FloatTuple(x[0], x[1])
        };

        let Some(indices) = endpoints.get(&current_end_key) else {
            bail!("No more matching linestrings found")
        };

        let Some(&next_index) = indices.iter().find(|i| !used_linestrings.contains(i)) else {
            bail!("No matching linestring found");
        };

        let next_linestring = &linestrings[next_index];
        let next_start_key = FloatTuple(next_linestring[0][0], next_linestring[0][1]);
        let next_end_key = {
            let x = next_linestring.last().context("Next linestring empty")?;
            FloatTuple(x[0], x[1])
        };

        if current_end_key == next_start_key {
            // If the current end matches the next start, extend normally
            continuous_line.extend_from_slice(&next_linestring[1..]);
        } else if current_end_key == next_end_key {
            // If the current end matches the next end, extend in reverse
            continuous_line.extend(
                next_linestring[..next_linestring.len() - 1]
                    .iter()
                    .rev()
                    .cloned(),
            );
        } else {
            bail!("Linestrings do not form a continuous path");
        }

        used_linestrings.insert(next_index);
    }

    // Check if the start and end positions match to close the loop
    if continuous_line.first() != continuous_line.last() {
        bail!("Ends of the linestrings don't form a ring");
    }

    Ok(continuous_line)
}

/// Calculate the orientation of the ring
fn is_clockwise(ring: &[geojson::Position]) -> bool {
    // Calculate the signed area under the curve (Shoelace formula).

    let cur = ring.iter();
    let next = ring.iter().chain(ring.iter()).skip(1);
    cur.zip(next).map(|(cur, next)| {
        // For each coordinate pair from `cur` and `next`, i.e., (x1, x2), (y1, y2), ...
        // -- LOL, this works for arbitrary dimensions.
        cur.iter()
            .zip(next.iter())
            // Only look at the first two dimensions since positions only have (x, y)
            // coordinates. Not sure why geojson needs an arbitrary number of dimension, i.e.,
            // `Vec` vs. `[f64; 2]`/`[f64; 3]` or some `Position2`/`Position3` structs.
            .take(2)
            // Zip the iterator over coordinates with an iterator yielding the sign in
            // addition. Only two elements here since we restrict ourself to two dimensions.
            .zip([-1.0, 1.0].iter())
            // In each dimension perform the coordinate sum over (cur, next) with the
            // correct sign.
            .map(|((x1, x2), sign)| x2 + sign * x1)
            // Multiply all the sums.
            .product::<f64>()
    })
    // Sum up all the area segments.
    .sum::<f64>()
    // If the area is larger than one, the ring is clockwise.
    // TODO: hier wird area==0.0 nicht behandelt weswegen rings mit zwei Punkten gleichzeitig
    // clock- und anticlockwise sind. Konsistenter wäre wahrscheinlich es in einmer der beide
    // Fälle mitzunehmen, z.B. `>=0.0`.
    > 0.0
}

#[cfg(test)]
mod test {
    #[test]
    fn is_clockwise() {
        use super::is_clockwise;

        // Points in the four quadrants of a cartesian coordinate system. Numbering here is
        // counterclockwise.
        let q1 = vec![1.0, 1.0];
        let q2 = vec![1.0, -1.0];
        let q3 = vec![-1.0, -1.0];
        let q4 = vec![-1.0, 1.0];

        // Degenerate cases.
        assert!(!is_clockwise(&[]));
        // assert!(is_clockwise(&[geojson::Position::default()])); // TODO: panics.
        assert!(!is_clockwise(&vec![q1.clone()]));

        // Segments with two elements are both clock- and counterclockwise.
        assert!(!is_clockwise(&vec![q1.clone(), q2.clone()]));
        assert!(!is_clockwise(&vec![q2.clone(), q1.clone()]));

        assert!(is_clockwise(&vec![
            q1.clone(),
            q2.clone(),
            q3.clone(),
            q4.clone()
        ]));

        assert!(!is_clockwise(&vec![
            q4.clone(),
            q3.clone(),
            q2.clone(),
            q1.clone()
        ]));
    }
}
