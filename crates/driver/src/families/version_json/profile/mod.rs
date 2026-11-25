use std::collections::HashSet;

use elemental_schema::fabric::ProfileJson;
use elemental_schema::mojang::piston::{
    PistonMetaArguments, PistonMetaData, PistonMetaGenericArgument,
};

use crate::error::{Error, Result};

use super::parse_argument_string;

mod library;
mod maven;

use library::{merge_libraries, profile_logging_to_piston};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LibraryReplacementFamily {
    Asm,
    Lwjgl2,
}

pub trait ProfileMergeBehavior {
    fn profile_replacement_family(&self, _library_name: &str) -> Option<LibraryReplacementFamily> {
        None
    }

    fn replaced_base_family(&self, _library_name: &str) -> Option<LibraryReplacementFamily> {
        None
    }
}

pub struct PassthroughProfileBehavior;

pub const PASSTHROUGH_PROFILE_BEHAVIOR: PassthroughProfileBehavior = PassthroughProfileBehavior;

impl ProfileMergeBehavior for PassthroughProfileBehavior {}

pub fn merge_profile_with_behavior(
    behavior: &dyn ProfileMergeBehavior,
    base_metadata: PistonMetaData,
    profile: ProfileJson,
) -> Result<PistonMetaData> {
    let PistonMetaData {
        arguments: base_arguments,
        minecraft_arguments: base_minecraft_arguments,
        client_version,
        asset_index,
        assets: base_assets,
        compliance_level,
        downloads,
        java_version,
        libraries: base_libraries,
        logging: base_logging,
        minimum_launcher_version,
        ..
    } = base_metadata;

    let profile_logging = profile.logging.map(profile_logging_to_piston);
    let merged_libraries = merge_libraries(base_libraries, profile.libraries, behavior)?;

    let arguments =
        merge_piston_meta_arguments(base_arguments, base_minecraft_arguments, profile.arguments)?;

    Ok(PistonMetaData {
        arguments,
        minecraft_arguments: None,
        inherits_from: Some(profile.inherits_from),
        client_version,
        asset_index,
        assets: profile.assets.unwrap_or(base_assets),
        compliance_level,
        downloads,
        id: profile.id,
        java_version,
        libraries: merged_libraries,
        logging: profile_logging.or(base_logging),
        main_class: profile.main_class,
        minimum_launcher_version: profile
            .minimum_launcher_version
            .unwrap_or(minimum_launcher_version),
        release_type: profile.release_type,
        time: profile.time,
        release_time: profile.release_time,
    })
}

pub fn metadata_has_replaced_library_conflicts<B>(behavior: &B, metadata: &PistonMetaData) -> bool
where
    B: ProfileMergeBehavior + ?Sized,
{
    let replacement_families = metadata
        .libraries
        .iter()
        .filter_map(|library| behavior.profile_replacement_family(library.name.as_str()))
        .collect::<HashSet<LibraryReplacementFamily>>();

    metadata.libraries.iter().any(|library| {
        let Some(family) = behavior.replaced_base_family(library.name.as_str()) else {
            return false;
        };

        replacement_families.contains(&family)
    })
}

pub fn merge_piston_meta_arguments(
    base_arguments: Option<PistonMetaArguments>,
    base_minecraft_arguments: Option<String>,
    overlay_arguments: Option<PistonMetaArguments>,
) -> Result<Option<PistonMetaArguments>> {
    let base_arguments = match (base_arguments, base_minecraft_arguments) {
        (Some(arguments), _) => Some(arguments),
        (None, Some(arguments)) => Some(PistonMetaArguments {
            game: parse_argument_string(arguments.as_str())
                .map_err(|source| Error::ParseBaseMinecraftArguments {
                    source: Box::new(source),
                })?
                .into_iter()
                .map(PistonMetaGenericArgument::Plain)
                .collect(),
            jvm: Vec::new(),
        }),
        (None, None) => None,
    };

    match (base_arguments, overlay_arguments) {
        (None, None) => Ok(None),
        (Some(base_arguments), None) => Ok(Some(base_arguments)),
        (None, Some(overlay_arguments)) => Ok(Some(overlay_arguments)),
        (Some(mut base_arguments), Some(overlay_arguments)) => {
            base_arguments.game.extend(overlay_arguments.game);
            base_arguments.jvm.extend(overlay_arguments.jvm);
            Ok(Some(base_arguments))
        }
    }
}
