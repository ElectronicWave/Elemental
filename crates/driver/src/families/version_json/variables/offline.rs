use std::path::Path;

use elemental_core::storage::layout::Layout;
use elemental_infra::SafeRelativePath;
use elemental_schema::mojang::piston::PistonMetaData;

use crate::{
    error::{Error, Result},
    families::version_json::{
        classpath::join_classpath,
        extensions::PistonMetaLibrariesExt,
        layout::{BaseInstanceLayout, BaseRootLayout},
        resource::{VersionJsonInstanceResource, VersionJsonRootResource},
        rules::VersionJsonRuleContext,
    },
};

use super::{LauncherVariables, UserType};

impl LauncherVariables {
    pub fn offline_player(
        player_name: String,
        game_directory: String,
        version_root: String,
        natives_directory: String,
        pistonmeta: &PistonMetaData,
    ) -> Result<Self> {
        let rule_context = VersionJsonRuleContext::current();
        let game_root = Path::new(&game_directory);
        let root_layout = BaseRootLayout;
        let instance_layout = BaseInstanceLayout;
        let assets_root =
            root_layout.try_get_extended_resource(game_root, VersionJsonRootResource::Assets)?;
        let version_root_path = Path::new(&version_root);
        let libraries_root = root_layout
            .try_get_extended_resource(game_root, VersionJsonRootResource::Libraries(None))?;
        let version_name = version_root_path
            .file_name()
            .ok_or_else(|| Error::MissingVersionRootFileName {
                path: version_root_path.to_path_buf(),
            })?
            .to_string_lossy()
            .to_string();
        let version_jar = instance_layout
            .try_get_extended_resource(version_root_path, VersionJsonInstanceResource::Jar)?;
        let mut classpath = Vec::new();
        for library in pistonmeta
            .libraries
            .iter()
            .filter(|library| library.is_allowed(&rule_context))
        {
            let downloads = library.downloads()?;
            let Some(artifact) = downloads.artifact.as_ref() else {
                continue;
            };
            if artifact.path.contains("natives") {
                continue;
            }

            let relative_path = SafeRelativePath::parse(artifact.path.as_str())?;
            let path = root_layout.try_get_extended_resource(
                game_root,
                VersionJsonRootResource::Libraries(Some(relative_path)),
            )?;
            classpath.push(path.to_string_lossy().to_string());
        }
        classpath.push(version_jar.to_string_lossy().to_string());
        let classpath = join_classpath(classpath);

        Ok(Self {
            auth_player_name: player_name,
            version_name,
            game_directory: game_directory.clone(),
            assets_root: assets_root.to_string_lossy().to_string(),
            assets_index_name: pistonmeta.assets.clone(),
            user_type: UserType::Legacy,
            version_type: pistonmeta.release_type.clone(),
            natives_directory,
            library_directory: libraries_root.to_string_lossy().to_string(),
            classpath,
            ..Self::default()
        })
    }
}
