use elemental_core::{auth::credential::UserCredential, launcher::command::LaunchCommand};
use elemental_schema::mojang::piston::PistonMetaData;

use crate::{
    error::{Error, Result},
    families::version_json::{
        classpath::classpath_separator, extensions::PistonMetaDataExt,
        rules::VersionJsonRuleContext, storage::VersionJsonVersionStorageExt, variables::UserType,
    },
};

use super::{VersionJsonLaunchBuilder, paths::LaunchPaths};

impl<A, L, VL> VersionJsonLaunchBuilder<A, L, VL>
where
    A: elemental_core::auth::authorizer::Authorizer,
    L: crate::families::version_json::layout::VersionJsonRootLayout,
    VL: crate::families::version_json::layout::VersionJsonInstanceLayout,
{
    pub async fn build_command(mut self) -> Result<LaunchCommand> {
        let metadata = self
            .version
            .metadata()
            .map_err(|source| Error::ReadVersionMetadata {
                source: Box::new(source),
            })?;
        let rule_context = VersionJsonRuleContext::current();
        let credential = self
            .authorizer
            .authorize()
            .await
            .map_err(|source| Error::Authorize { source })?;
        let version_name = self
            .version
            .name()
            .ok_or_else(|| Error::MissingVersionName {
                path: self.version.path.clone(),
            })?;
        let paths = LaunchPaths::resolve(&self.version)?;
        paths.ensure_version_jar_exists()?;

        self.apply_default_variables(&metadata, credential, version_name, &paths);

        let raw_jvm_arguments = metadata.jvm_arguments(&rule_context);
        let module_path_entries = self.collect_module_path_entries(raw_jvm_arguments.as_slice())?;
        self.inner.classpath =
            self.build_classpath(&metadata, &rule_context, &paths, &module_path_entries)?;

        let command_arguments =
            self.build_command_arguments(&metadata, &rule_context, raw_jvm_arguments)?;

        Ok(
            LaunchCommand::new(self.runtime.executable(), command_arguments)
                .with_cwd(paths.version_root),
        )
    }

    fn apply_default_variables(
        &mut self,
        metadata: &PistonMetaData,
        credential: UserCredential,
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
        self.inner.auth_uuid = credential.uuid;
        self.inner.auth_access_token = credential.access_token;
        self.inner.user_type = if self.inner.auth_access_token.is_empty() {
            UserType::Legacy
        } else {
            UserType::Msa
        };
        self.inner.version_type = metadata.release_type.clone();
        self.inner.library_directory = paths.libraries_root.to_string_lossy().to_string();
        self.inner.classpath_separator = classpath_separator().to_owned();
        self.inner.natives_directory = paths.natives_directory.to_string_lossy().to_string();
    }
}
