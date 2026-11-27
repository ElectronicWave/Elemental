use std::collections::{HashMap, HashSet};

use elemental_schema::mojang::piston::{PistonMetaArguments, PistonMetaData, PistonMetaLibraries};

use crate::{
    error::{Error, Result},
    families::version_json::merge_piston_meta_arguments,
};

use super::{MetadataPatchStepDiff, NamedMetadataPatch, PatchTrace};

/// Applies an ordered set of [`NamedMetadataPatch`] overlays onto `base` and
/// returns both the final [`PistonMetaData`] and per-step diffs.
///
/// Ordering respects explicit `before` / `after` constraints first, breaking
/// ties by `(priority, original_index)`. Cycles fail with [`Error::PatchOrderCycle`].
pub fn apply_metadata_patches_traced(
    mut metadata: PistonMetaData,
    patches: &[NamedMetadataPatch],
) -> Result<PatchTrace> {
    if patches.is_empty() {
        return Ok(PatchTrace {
            metadata,
            steps: Vec::new(),
        });
    }

    let ordered = order_patches(patches)?;
    let mut steps = Vec::with_capacity(ordered.len());
    let mut main_class_owner: Option<(&str, &str)> = None;

    for entry in ordered {
        let patch = &entry.patch;

        let main_class_diff = if let Some(main_class) = patch.main_class.as_deref() {
            if let Some((first_patch, first_main_class)) = main_class_owner
                && first_main_class != main_class
            {
                return Err(Error::PatchMainClassConflict {
                    first_patch: first_patch.to_owned(),
                    first_main_class: first_main_class.to_owned(),
                    second_patch: entry.id.clone(),
                    second_main_class: main_class.to_owned(),
                });
            }
            main_class_owner = Some((entry.id.as_str(), main_class));
            if metadata.main_class != main_class {
                let previous = std::mem::replace(&mut metadata.main_class, main_class.to_owned());
                Some((previous, main_class.to_owned()))
            } else {
                None
            }
        } else {
            None
        };

        let mut removed_libraries = Vec::new();
        if !patch.remove_library_prefixes.is_empty() {
            metadata.libraries.retain(|library| {
                let matched = patch
                    .remove_library_prefixes
                    .iter()
                    .any(|prefix| library.name.starts_with(prefix));
                if matched {
                    removed_libraries.push(library.name.clone());
                }
                !matched
            });
        }

        let mut replaced_libraries = Vec::new();
        let mut added_libraries = Vec::new();
        for library in &patch.add_libraries {
            if upsert_library(&mut metadata.libraries, library.clone()) {
                replaced_libraries.push(library.name.clone());
            } else {
                added_libraries.push(library.name.clone());
            }
        }

        if !patch.game_arguments.is_empty() || !patch.jvm_arguments.is_empty() {
            let overlay = PistonMetaArguments {
                game: patch.game_arguments.clone(),
                jvm: patch.jvm_arguments.clone(),
            };
            let merged = merge_piston_meta_arguments(
                metadata.arguments.take(),
                metadata.minecraft_arguments.take(),
                Some(overlay),
            )?;
            metadata.arguments = merged;
            metadata.minecraft_arguments = None;
        }

        let logging_changed = if let Some(logging) = &patch.logging {
            metadata.logging = Some(logging.clone());
            true
        } else {
            false
        };

        let java_major_diff = if let Some(java_major_version) = patch.java_major_version {
            if metadata.java_version.major_version != java_major_version {
                let previous = metadata.java_version.major_version;
                metadata.java_version.major_version = java_major_version;
                Some((previous, java_major_version))
            } else {
                None
            }
        } else {
            None
        };

        steps.push(MetadataPatchStepDiff {
            id: entry.id.clone(),
            priority: patch.priority,
            effective_mode: entry.effective_mode,
            main_class: main_class_diff,
            removed_libraries,
            replaced_libraries,
            added_libraries,
            added_game_arguments: patch.game_arguments.clone(),
            added_jvm_arguments: patch.jvm_arguments.clone(),
            logging_changed,
            java_major_version: java_major_diff,
        });
    }

    Ok(PatchTrace { metadata, steps })
}

/// Applies an ordered set of [`NamedMetadataPatch`] overlays onto `base`.
pub fn apply_metadata_patches(
    base: PistonMetaData,
    patches: &[NamedMetadataPatch],
) -> Result<PistonMetaData> {
    Ok(apply_metadata_patches_traced(base, patches)?.metadata)
}

fn order_patches(patches: &[NamedMetadataPatch]) -> Result<Vec<&NamedMetadataPatch>> {
    let n = patches.len();
    let index_by_id = patches
        .iter()
        .enumerate()
        .map(|(index, entry)| (entry.id.as_str(), index))
        .collect::<HashMap<_, _>>();

    let mut edges: Vec<HashSet<usize>> = vec![HashSet::new(); n];
    let mut in_degree = vec![0usize; n];

    for (index, entry) in patches.iter().enumerate() {
        for target in &entry.patch.before {
            if let Some(&target_idx) = index_by_id.get(target.as_str())
                && target_idx != index
                && edges[index].insert(target_idx)
            {
                in_degree[target_idx] += 1;
            }
        }
        for dependency in &entry.patch.after {
            if let Some(&dep_idx) = index_by_id.get(dependency.as_str())
                && dep_idx != index
                && edges[dep_idx].insert(index)
            {
                in_degree[index] += 1;
            }
        }
    }

    let mut ready = (0..n)
        .filter(|&idx| in_degree[idx] == 0)
        .collect::<Vec<_>>();
    let mut ordered = Vec::with_capacity(n);

    while !ready.is_empty() {
        ready.sort_by_key(|&idx| (patches[idx].patch.priority, idx));
        let next = ready.remove(0);
        ordered.push(&patches[next]);

        for &neighbor in &edges[next] {
            in_degree[neighbor] -= 1;
            if in_degree[neighbor] == 0 {
                ready.push(neighbor);
            }
        }
    }

    if ordered.len() != n {
        let cycle = (0..n)
            .filter(|&idx| in_degree[idx] > 0)
            .map(|idx| patches[idx].id.clone())
            .collect();
        return Err(Error::PatchOrderCycle { cycle });
    }

    Ok(ordered)
}

/// Returns `true` when an existing library entry was replaced in place.
fn upsert_library(
    libraries: &mut Vec<PistonMetaLibraries>,
    replacement: PistonMetaLibraries,
) -> bool {
    if let Some(existing) = libraries
        .iter_mut()
        .find(|library| library.name == replacement.name)
    {
        *existing = replacement;
        true
    } else {
        libraries.push(replacement);
        false
    }
}

#[cfg(test)]
mod tests {
    use elemental_schema::mojang::piston::{
        PistonMetaAssetIndex, PistonMetaDownload, PistonMetaDownloads, PistonMetaGenericArgument,
        PistonMetaJavaVersion, PistonMetaLibrariesSource,
    };

    use super::*;
    use crate::families::version_json::patch::{MetadataPatch, PatchMode};

    fn sample_metadata() -> PistonMetaData {
        PistonMetaData {
            arguments: Some(PistonMetaArguments {
                game: vec![PistonMetaGenericArgument::Plain("--username".to_owned())],
                jvm: vec![PistonMetaGenericArgument::Plain("-Xmx2G".to_owned())],
            }),
            minecraft_arguments: None,
            inherits_from: None,
            client_version: None,
            asset_index: PistonMetaAssetIndex {
                id: "1.20".to_owned(),
                sha1: String::new(),
                size: 0,
                total_size: 0,
                url: "https://example.invalid/assets.json".to_owned(),
            },
            assets: "1.20".to_owned(),
            compliance_level: 0,
            downloads: PistonMetaDownloads {
                client: PistonMetaDownload {
                    sha1: String::new(),
                    size: 0,
                    url: "https://example.invalid/client.jar".to_owned(),
                },
            },
            id: "1.20.1".to_owned(),
            java_version: PistonMetaJavaVersion {
                component: "java-runtime-gamma".to_owned(),
                major_version: 17,
            },
            libraries: vec![maven_lib("org.lwjgl:lwjgl:3.3.1")],
            logging: None,
            main_class: "net.minecraft.client.main.Main".to_owned(),
            minimum_launcher_version: 21,
            release_type: "release".to_owned(),
            time: "2024-01-01T00:00:00Z".to_owned(),
            release_time: "2024-01-01T00:00:00Z".to_owned(),
        }
    }

    fn maven_lib(name: &str) -> PistonMetaLibraries {
        PistonMetaLibraries {
            name: name.to_owned(),
            source: PistonMetaLibrariesSource::Maven {
                url: Some("https://repo.example.invalid/".to_owned()),
                sha1: None,
                size: None,
                extra: serde_json::Map::new(),
            },
            natives: None,
            rules: None,
            extract: None,
        }
    }

    #[test]
    fn patches_replace_libraries_merge_args_and_record_step_diffs() {
        let base = sample_metadata();
        let patches = vec![
            NamedMetadataPatch {
                id: "optifine".to_owned(),
                effective_mode: PatchMode::Virtual,
                patch: MetadataPatch {
                    priority: 20,
                    main_class: Some("net.minecraft.launchwrapper.Launch".to_owned()),
                    add_libraries: vec![
                        maven_lib("org.lwjgl:lwjgl:3.3.3"),
                        maven_lib("optifine:OptiFine:1.20.1"),
                    ],
                    game_arguments: vec![
                        PistonMetaGenericArgument::Plain("--tweakClass".to_owned()),
                        PistonMetaGenericArgument::Plain("optifine.OptiFineTweaker".to_owned()),
                    ],
                    ..Default::default()
                },
            },
            NamedMetadataPatch {
                id: "lwjgl-override".to_owned(),
                effective_mode: PatchMode::Physical,
                patch: MetadataPatch {
                    priority: 10,
                    main_class: Some("net.minecraft.launchwrapper.Launch".to_owned()),
                    remove_library_prefixes: vec!["org.lwjgl:lwjgl:".to_owned()],
                    add_libraries: vec![maven_lib("org.lwjgl:lwjgl:3.3.3")],
                    game_arguments: vec![
                        PistonMetaGenericArgument::Plain("--tweakClass".to_owned()),
                        PistonMetaGenericArgument::Plain(
                            "com.mumfrey.liteloader.launch.LiteLoaderTweaker".to_owned(),
                        ),
                    ],
                    jvm_arguments: vec![PistonMetaGenericArgument::Plain("-Dtest=1".to_owned())],
                    java_major_version: Some(21),
                    ..Default::default()
                },
            },
        ];

        let trace = apply_metadata_patches_traced(base, &patches).unwrap();

        assert_eq!(
            trace.metadata.main_class,
            "net.minecraft.launchwrapper.Launch"
        );
        assert_eq!(trace.metadata.java_version.major_version, 21);
        assert_eq!(trace.steps.len(), 2);

        let step0 = &trace.steps[0];
        assert_eq!(step0.id, "lwjgl-override");
        assert_eq!(step0.effective_mode, PatchMode::Physical);
        assert_eq!(
            step0.main_class,
            Some((
                "net.minecraft.client.main.Main".to_owned(),
                "net.minecraft.launchwrapper.Launch".to_owned(),
            ))
        );
        assert_eq!(step0.removed_libraries, vec!["org.lwjgl:lwjgl:3.3.1"]);
        assert_eq!(step0.added_libraries, vec!["org.lwjgl:lwjgl:3.3.3"]);
        assert_eq!(step0.java_major_version, Some((17, 21)));

        let step1 = &trace.steps[1];
        assert_eq!(step1.id, "optifine");
        assert_eq!(step1.effective_mode, PatchMode::Virtual);
        assert_eq!(step1.main_class, None);
        assert_eq!(step1.replaced_libraries, vec!["org.lwjgl:lwjgl:3.3.3"]);
        assert_eq!(step1.added_libraries, vec!["optifine:OptiFine:1.20.1"]);

        let arguments = trace.metadata.arguments.unwrap();
        let game_plain = arguments
            .game
            .iter()
            .map(|arg| match arg {
                PistonMetaGenericArgument::Plain(value) => value.as_str(),
                PistonMetaGenericArgument::Rule(_) => panic!("unexpected rule argument"),
            })
            .collect::<Vec<_>>();
        assert_eq!(
            game_plain,
            vec![
                "--username",
                "--tweakClass",
                "com.mumfrey.liteloader.launch.LiteLoaderTweaker",
                "--tweakClass",
                "optifine.OptiFineTweaker",
            ]
        );
    }

    #[test]
    fn before_after_constraints_override_priority_and_detect_cycles() {
        let base = sample_metadata();
        let patches = vec![
            NamedMetadataPatch {
                id: "a".to_owned(),
                effective_mode: PatchMode::Virtual,
                patch: MetadataPatch {
                    priority: 1,
                    after: vec!["b".to_owned()],
                    ..Default::default()
                },
            },
            NamedMetadataPatch {
                id: "b".to_owned(),
                effective_mode: PatchMode::Virtual,
                patch: MetadataPatch {
                    priority: 100,
                    ..Default::default()
                },
            },
        ];

        let trace = apply_metadata_patches_traced(base.clone(), &patches).unwrap();
        let ids = trace
            .steps
            .iter()
            .map(|step| step.id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(ids, vec!["b", "a"]);

        let cyclic = vec![
            NamedMetadataPatch {
                id: "a".to_owned(),
                effective_mode: PatchMode::Virtual,
                patch: MetadataPatch {
                    before: vec!["b".to_owned()],
                    ..Default::default()
                },
            },
            NamedMetadataPatch {
                id: "b".to_owned(),
                effective_mode: PatchMode::Virtual,
                patch: MetadataPatch {
                    before: vec!["a".to_owned()],
                    ..Default::default()
                },
            },
        ];
        let err = apply_metadata_patches_traced(base, &cyclic).unwrap_err();
        assert!(matches!(err, Error::PatchOrderCycle { .. }));
    }

    #[test]
    fn conflicting_main_class_patches_return_error() {
        let base = sample_metadata();
        let patches = vec![
            NamedMetadataPatch {
                id: "a".to_owned(),
                effective_mode: PatchMode::Virtual,
                patch: MetadataPatch {
                    main_class: Some("example.MainA".to_owned()),
                    ..Default::default()
                },
            },
            NamedMetadataPatch {
                id: "b".to_owned(),
                effective_mode: PatchMode::Virtual,
                patch: MetadataPatch {
                    main_class: Some("example.MainB".to_owned()),
                    ..Default::default()
                },
            },
        ];

        let err = apply_metadata_patches(base, &patches).unwrap_err();
        assert!(matches!(err, Error::PatchMainClassConflict { .. }));
    }
}
