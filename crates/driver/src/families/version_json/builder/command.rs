use elemental_core::storage::layout::Layoutable;
use elemental_core::{
    auth::credential::{Credential, CredentialKind},
    launcher::command::LaunchCommand,
};
use elemental_schema::mojang::piston::PistonMetaData;

use crate::{
    error::{Error, Result},
    families::version_json::{
        VersionJsonInstanceResource, classpath::classpath_separator, extensions::PistonMetaDataExt,
        rules::VersionJsonRuleContext, storage::VersionJsonMetadataStorageExt, variables::UserType,
    },
};

use super::{VersionJsonLaunchBuilder, paths::LaunchPaths};

impl<L, VL> VersionJsonLaunchBuilder<L, VL>
where
    L: crate::families::version_json::layout::VersionJsonRootLayout,
    VL: crate::families::version_json::layout::VersionJsonInstanceLayout,
{
    pub fn build_command(mut self) -> Result<LaunchCommand> {
        let metadata = self
            .version
            .metadata()
            .map_err(|source| Error::ReadVersionMetadata {
                source: Box::new(source),
            })?;
        let rule_context = VersionJsonRuleContext::current();
        let version_name = self
            .version
            .name()
            .ok_or_else(|| Error::MissingVersionName {
                path: self.version.path.clone(),
            })?;
        // The builder is public and can be used without the readiness checks.
        let version_jar = self
            .version
            .try_get_resource(VersionJsonInstanceResource::Jar)?;
        if !version_jar.exists() {
            return Err(Error::VersionJarMissing { path: version_jar });
        }
        let paths = LaunchPaths::resolve(&self.version)?;

        let credential = self.credential.clone();
        self.apply_default_variables(&metadata, &credential, version_name, &paths);

        let raw_jvm_arguments = metadata.jvm_arguments(&rule_context);
        let module_path_entries = self.collect_module_path_entries(raw_jvm_arguments.as_slice())?;
        self.inner.classpath =
            self.build_classpath(&metadata, &rule_context, &module_path_entries)?;

        let jvm_arguments = self.build_jvm_arguments(&metadata, raw_jvm_arguments)?;
        let game_arguments = self.build_game_arguments(&metadata, &rule_context)?;

        Ok(LaunchCommand::new(
            self.runtime,
            jvm_arguments,
            metadata.main_class.clone(),
            game_arguments,
        )
        .with_cwd(paths.version_root))
    }

    fn apply_default_variables(
        &mut self,
        metadata: &PistonMetaData,
        credential: &Credential,
        version_name: String,
        paths: &LaunchPaths,
    ) {
        if self.inner.auth_player_name.is_empty() {
            self.inner.auth_player_name = credential.username.clone();
        }

        self.inner.version_name = version_name;
        self.inner.game_directory = paths.version_root.to_string_lossy().to_string();
        self.inner.assets_root = paths.assets_root.to_string_lossy().to_string();
        self.inner.assets_index_name = metadata.assets.clone();
        self.inner.auth_uuid = credential.uuid.clone();
        self.inner.auth_access_token = credential.access_token.clone();
        self.inner.user_type = match credential.kind {
            CredentialKind::Legacy => UserType::Legacy,
            CredentialKind::Mojang => UserType::Mojang,
            CredentialKind::Msa => UserType::Msa,
        };
        if self.inner.clientid.is_empty() {
            self.inner.clientid = credential.client_id.clone().unwrap_or_default();
        }
        if self.inner.auth_xuid.is_empty() {
            self.inner.auth_xuid = credential.xuid.clone().unwrap_or_default();
        }
        self.inner.version_type = metadata.release_type.clone();
        self.inner.library_directory = paths.libraries_root.to_string_lossy().to_string();
        self.inner.classpath_separator = classpath_separator().to_owned();
        self.inner.natives_directory = paths.natives_directory.to_string_lossy().to_string();
    }
}
