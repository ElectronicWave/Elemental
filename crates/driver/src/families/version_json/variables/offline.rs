use std::path::Path;

use elemental_core::storage::layout::Layout;
use elemental_infra::SafeRelativePath;
use elemental_schema::mojang::piston::PistonMetaData;

use crate::{
    error::{Error, Result},
    families::version_json::{
        classpath::{classpath_separator, join_classpath},
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
        let classpath = pistonmeta
            .libraries
            .iter()
            .filter(|library| library.is_allowed(&rule_context))
            .map(|library| -> Result<Option<String>> {
                let Some(artifact) = library.downloads.artifact.as_ref() else {
                    return Ok(None);
                };
                if artifact.path.contains("natives") {
                    return Ok(None);
                }

                let relative_path = SafeRelativePath::parse(artifact.path.as_str())?;
                root_layout
                    .try_get_extended_resource(
                        game_root,
                        VersionJsonRootResource::Libraries(Some(relative_path)),
                    )
                    .map(|path| Some(path.to_string_lossy().to_string()))
                    .map_err(Error::from)
            })
            .collect::<Result<Vec<Option<String>>>>()?
            .into_iter()
            .flatten()
            .chain(std::iter::once(version_jar.to_string_lossy().to_string()))
            .collect::<Vec<String>>();
        let classpath = join_classpath(classpath);

        Ok(Self {
            auth_player_name: player_name,
            version_name,
            game_directory: game_directory.clone(),
            assets_root: assets_root.to_string_lossy().to_string(),
            assets_index_name: pistonmeta.assets.clone(),
            auth_uuid: String::new(),
            auth_access_token: String::new(),
            user_properties: "{}".to_owned(),
            clientid: String::new(),
            auth_xuid: String::new(),
            user_type: UserType::Legacy,
            version_type: pistonmeta.release_type.clone(),
            resolution_width: "854".to_owned(),
            resolution_height: "480".to_owned(),
            quick_play_path: None,
            quick_play_singleplayer: None,
            quick_play_multiplayer: None,
            quick_play_realms: None,
            natives_directory,
            launcher_name: "Elemental".to_owned(),
            launcher_version: env!("CARGO_PKG_VERSION").to_owned(),
            library_directory: libraries_root.to_string_lossy().to_string(),
            classpath_separator: classpath_separator().to_owned(),
            classpath,
        })
    }
}
