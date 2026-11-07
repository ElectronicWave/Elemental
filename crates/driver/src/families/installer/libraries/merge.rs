use std::collections::HashSet;

use elemental_schema::mojang::piston::PistonMetaLibraries;

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
    let (coordinates, extension) = notation.split_once('@').unwrap_or((notation, "jar"));
    let segments = coordinates.split(':').collect::<Vec<&str>>();

    match segments.as_slice() {
        [group, artifact, _version] => format!("{group}:{artifact}@{extension}"),
        [group, artifact, _version, classifier] => {
            format!("{group}:{artifact}:{classifier}@{extension}")
        }
        _ => notation.to_owned(),
    }
}
