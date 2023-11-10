use osmpbfreader::OsmObj;
use regex::Regex;

/// Filter for relations having name and a range of `admin_level`.
pub fn all(obj: &OsmObj) -> bool {
    obj.is_relation()
}

/// Filter for relations. In addition to `filter::all`, add boundary types.
pub fn by_target(obj: &OsmObj) -> bool {
    let tags = obj.tags();
    all(obj)
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

/// Filter relations by a query that can be a substring or a regex pattern
pub fn by_query(query: &str) -> impl Fn(&OsmObj) -> bool {
    let pattern = query.to_lowercase();
    let regex = Regex::new(query).ok();

    move |obj: &OsmObj| {
        if let Some(name) = obj.tags().get("name") {
            match &regex {
                Some(re) => re.is_match(name), // Use regex for matching if it's valid
                None => name.to_lowercase().contains(&pattern), // Fallback to case-insensitive substring match
            }
        } else {
            false // If the object doesn't have a name tag, it doesn't match
        }
    }
}
