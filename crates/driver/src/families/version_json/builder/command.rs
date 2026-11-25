use elemental_core::storage::layout::Layoutable;
use elemental_core::{auth::credential::Credential, launcher::command::LaunchCommand};
use elemental_schema::mojang::piston::PistonMetaData;

use crate::{
    error::{Error, Result},
    families::version_json::{
        VersionJsonInstanceResource, extensions::PistonMetaDataExt, rules::VersionJsonRuleContext,
        storage::VersionJsonMetadataStorageExt,
    },
};

use super::{VersionJsonLaunchBuilder, arguments, paths::LaunchPaths};

impl VersionJsonLaunchBuilder {
    pub fn build_command(mut self) -> Result<LaunchCommand> {
        let metadata = self
            .version
            .metadata()
            .map_err(|source| Error::ReadVersionMetadata {
                source: Box::new(source),
            })?;
        let rule_context = VersionJsonRuleContext::current();
        // The builder is public and can be used without the readiness checks.
        let version_jar = self
            .version
            .try_get_resource(VersionJsonInstanceResource::Jar)?;
        if !version_jar.exists() {
            return Err(Error::VersionJarMissing { path: version_jar });
        }
        let paths = LaunchPaths::resolve(&self.version)?;

        let credential = self.credential.clone();
        self.apply_default_variables(&metadata, &credential, &paths)?;

        let raw_jvm_arguments = metadata.jvm_arguments(&rule_context);
        let module_path_entries =
            arguments::collect_module_path_entries(&self.inner, raw_jvm_arguments.as_slice())?;
        self.inner.populate_classpath(
            &self.version,
            &metadata,
            &rule_context,
            &module_path_entries,
        )?;

        let jvm_arguments = arguments::build_jvm_arguments(
            &self.version,
            &self.inner,
            &self.extra_jvm_arguments,
            &metadata,
            raw_jvm_arguments,
        )?;
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
        paths: &LaunchPaths,
    ) -> Result<()> {
        self.inner.apply_credential(credential);
        self.inner.apply_metadata(&self.version, metadata)?;
        self.inner.apply_paths(
            &paths.version_root,
            &paths.assets_root,
            &paths.libraries_root,
            &paths.natives_directory,
        );
        Ok(())
    }
}
