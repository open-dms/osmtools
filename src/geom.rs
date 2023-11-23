use std::{
    collections::{BTreeMap, HashMap, HashSet},
    io::Write,
    io::{self, BufWriter},
};

use anyhow::{anyhow, bail, Context, Result};
use geojson::{self, GeoJson, Geometry};
use log::error;
use osmpbfreader::{OsmId, OsmObj, Ref, Way};
use serde_json::json;

use crate::filter;

#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone)]
struct Position(
    ordered_float::OrderedFloat<f64>,
    ordered_float::OrderedFloat<f64>,
);

impl Position {
    pub fn new(x: f64, y: f64) -> Self {
        Self(x.into(), y.into())
    }
}

#[derive(Clone)]
struct Line(Vec<Position>);

impl Line {
    fn start(&self) -> &Position {
        self.0.first().expect("line cannot be empty")
    }

    fn end(&self) -> &Position {
        self.0.last().expect("line cannot be empty")
    }

    fn extend(&mut self, tail: &Line) -> Result<()> {
        if tail.start() == self.end() {
            // If the current end matches the next start, extend normally
            self.0.extend_from_slice(&tail.0[1..]);
        } else if tail.end() == self.end() {
            // If the current end matches the next end, extend in reverse
            self.0.extend(tail.0.iter().rev().skip(1));
        } else {
            bail!("Linestrings do not form a continuous path");
        }

        Ok(())
    }
}

impl TryFrom<Vec<Position>> for Line {
    type Error = &'static str;

    fn try_from(value: Vec<Position>) -> std::result::Result<Self, Self::Error> {
        if value.len() < 2 {
            return Err("cannot construct line with less than two points");
        }

        Ok(Self(value))
    }
}

pub fn write(objs: &BTreeMap<OsmId, OsmObj>, out: impl io::Write) -> Result<()> {
    // Use a buffered writer to amortize flushes.
    let mut buffer = BufWriter::new(out);

    for relation in objs.values().filter(|obj| filter::by_target(obj)) {
        match to_feature(relation, objs) {
            Ok(feature) => {
                let serialized = feature.to_string();
                writeln!(buffer, "{serialized}")?;
            }
            Err(e) => error!("{e}: {}", e.root_cause()),
        }
    }

    Ok(())
}

fn to_feature(obj: &OsmObj, all_objs: &BTreeMap<OsmId, OsmObj>) -> Result<geojson::GeoJson> {
    let tags = obj.tags();
    let name = {
        let n = tags
            .get("name")
            .ok_or_else(|| anyhow!("'name' is missing"))?;
        tags.get("name:prefix")
            .map(|p| format!("{p} {n}"))
            .unwrap_or(n.to_string())
    };
    let admin_level = tags
        .get("admin_level")
        .ok_or_else(|| anyhow!("'admin_level' is missing"))?;
    let ars = tags
        .get("de:regionalschluessel")
        .ok_or_else(|| anyhow!("'de:regionalschluessel' is missing"))?;

    let serde_json::Value::Object(properties) = json!({
        "name": name,
        "adminLevel":admin_level.parse::<u8>()?,
        "ars": ars,
    }) else {
        todo!()
    };

    let geometry = Geometry::new(
        as_polygon(obj, all_objs)
            .with_context(|| format!("cannot convert object '{name}' to polygon"))?,
    );

    Ok(GeoJson::Feature(geojson::Feature {
        id: Some(geojson::feature::Id::Number(
            serde_json::value::Number::from(
                obj.relation()
                    .ok_or_else(|| anyhow!("'relation' is missing"))?
                    .id
                    .0,
            ),
        )),
        geometry: Some(geometry),
        properties: Some(properties),
        ..geojson::Feature::default()
    }))
}

fn as_polygon(obj: &OsmObj, all_objs: &BTreeMap<OsmId, OsmObj>) -> Result<geojson::Value> {
    let to_coords = |way: &Way| -> Option<Vec<Position>> {
        way.nodes
            .iter()
            .map(|node_id| {
                let node = all_objs.get(&OsmId::Node(*node_id))?;
                Some(Position::new(
                    f64::from(node.node()?.decimicro_lon) / 10_000_000.0,
                    f64::from(node.node()?.decimicro_lat) / 10_000_000.0,
                ))
            })
            .collect()
    };

    let linestrings = obj
        .relation()
        .ok_or_else(|| anyhow!("'relation' is missing"))?
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
        .filter_map(|xs: Vec<_>| Line::try_from(xs).ok())
        .collect();

    // todo report missing geometry or broken linering
    let mut linering = create_continuous_linering(&linestrings)?;

    // respect right hand rule
    if is_clockwise(&linering) {
        linering.0.reverse();
    }

    Ok(geojson::Value::Polygon(vec![linering
        .0
        .iter()
        .map(|p| vec![*p.0, *p.1])
        .collect()]))
}

/// Create a continuous ring from line strings.
fn create_continuous_linering(linestrings: &Vec<Line>) -> Result<Line> {
    if linestrings.is_empty() {
        bail!("no linestrings")
    }

    // Convert the endpoint positions to a hashable type (tuple) and build the index map
    let mut endpoints: HashMap<Position, Vec<usize>> = HashMap::new();
    for (i, linestring) in linestrings.iter().enumerate() {
        let start = Position::new(*linestring.start().0, *linestring.start().1);
        let end = Position::new(*linestring.end().0, *linestring.end().1);
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
        let current_end_key = continuous_line.end();

        let Some(indices) = endpoints.get(&current_end_key) else {
            bail!("No more matching linestrings found")
        };

        let Some(&next_index) = indices.iter().find(|i| !used_linestrings.contains(i)) else {
            bail!("No matching linestring found");
        };

        let next_linestring = &linestrings[next_index];
        continuous_line.extend(next_linestring)?;

        used_linestrings.insert(next_index);
    }

    // Check if the start and end positions match to close the loop
    if continuous_line.start() != continuous_line.end() {
        bail!("Ends of the linestrings don't form a ring");
    }

    Ok(continuous_line)
}

/// Calculate the orientation of the ring
fn is_clockwise(ring: &Line) -> bool {
    // Calculate the signed area under the curve (Shoelace formula).

    let cur = ring.0.iter();
    let next = ring.0.iter().chain(ring.0.iter()).skip(1);
    cur.zip(next)
        .map(|(c, n)| *((n.0 - c.0) * (n.1 + c.1)))
        .sum::<f64>()
        > 0.0
}

#[cfg(test)]
mod test {
    #[test]
    fn is_clockwise() {
        use super::{is_clockwise, Line, Position};

        // Points in the four quadrants of a cartesian coordinate system. Numbering here is
        // counterclockwise.
        let q1 = Position::new(1.0, 1.0);
        let q2 = Position::new(1.0, -1.0);
        let q3 = Position::new(-1.0, -1.0);
        let q4 = Position::new(-1.0, 1.0);

        // Segments with two elements are both clock- and counterclockwise.
        assert!(!is_clockwise(&Line::try_from(vec![q1, q2]).unwrap()));
        assert!(!is_clockwise(&Line::try_from(vec![q2, q1]).unwrap()));

        assert!(is_clockwise(&Line::try_from(vec![q1, q2, q3, q4]).unwrap()));
        assert!(!is_clockwise(
            &Line::try_from(vec![q4, q3, q2, q1]).unwrap()
        ));
    }
}
