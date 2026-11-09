use std::collections::HashSet;

use elemental_schema::mojang::piston::PistonMetaLibraries;

use crate::maven::MavenCoordinate;

pub fn merge_libraries_prefer_embedded(
    base_libraries: Vec<PistonMetaLibraries>,
    embedded_libraries: Vec<PistonMetaLibraries>,
) -> Vec<PistonMetaLibraries> {
    let embedded_keys = embedded_libraries
        .iter()
        .map(library_identity_key)
        .collect::<HashSet<String>>();
    let mut merged = base_libraries
        .into_iter()
        .filter(|library| !embedded_keys.contains(&library_identity_key(library)))
        .collect::<Vec<PistonMetaLibraries>>();
    let mut seen = merged
        .iter()
        .map(library_identity_key)
        .collect::<HashSet<String>>();

    for library in embedded_libraries {
        let identity = library_identity_key(&library);
        if seen.insert(identity) {
            merged.push(library);
        }
    }

    merged
}

fn library_identity_key(library: &PistonMetaLibraries) -> String {
    let notation = library.name.as_str();
    let Some(coordinate) = MavenCoordinate::parse(notation) else {
        return notation.to_owned();
    };

    match coordinate.classifier() {
        Some(classifier) => format!(
            "{}:{}:{classifier}@{}",
            coordinate.group(),
            coordinate.artifact(),
            coordinate.extension()
        ),
        None => format!(
            "{}:{}@{}",
            coordinate.group(),
            coordinate.artifact(),
            coordinate.extension()
        ),
    }
}
