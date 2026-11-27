pub mod catalog;
mod manifest;
mod profile;
pub mod source;

use async_trait::async_trait;
use elemental_core::http::SwappableClient;
use elemental_driver::{
    catalog::GameVersion,
    descriptors::{FORGE_DRIVER, VANILLA_DRIVER},
    driver::InstanceProbe,
    families::version_json::{
        MetadataPatch, PatchMode, PistonMetaLibrariesExt, VersionJsonPlatform,
        VersionJsonRootResource, profile_library_to_piston,
    },
};
use elemental_infra::{
    SafeRelativePath,
    downloader::{DownloadBatch, DownloadTask},
};

use crate::{
    addon::{
        Addon, AddonCatalogQuery, AddonCompatibility, AddonContext, AddonDescriptor, AddonOrigin,
        AddonPrepareOutput, AddonPrepareRequest, AddonRelease, InstalledAddon,
    },
    error::{Error, Result},
};

use self::{catalog::LiteLoaderCatalog, source::LiteLoaderSource};

pub const LITELOADER_ADDON_ID: &str = "liteloader";
const LITELOADER_LIBRARY_PREFIX: &str = "com.mumfrey:liteloader:";

/// Composable LiteLoader addon supporting Vanilla and Forge host instances.
#[derive(Debug, Clone)]
pub struct LiteLoaderAddon {
    source: LiteLoaderSource,
}

impl Default for LiteLoaderAddon {
    fn default() -> Self {
        Self::new(LiteLoaderSource::default())
    }
}

impl LiteLoaderAddon {
    pub fn new(source: LiteLoaderSource) -> Self {
        Self { source }
    }

    pub fn with_http_client(mut self, client: SwappableClient) -> Self {
        self.source = self.source.with_http_client(client);
        self
    }
}

#[async_trait]
impl Addon for LiteLoaderAddon {
    fn descriptor(&self) -> AddonDescriptor {
        AddonDescriptor::new(LITELOADER_ADDON_ID, "LiteLoader")
    }

    async fn supported_game_versions(&self) -> Result<Option<Vec<GameVersion>>> {
        let catalog = LiteLoaderCatalog::new(self.source.clone());
        Ok(Some(catalog.game_versions().await?))
    }

    async fn available_versions(
        &self,
        query: &AddonCatalogQuery<'_>,
    ) -> Result<Option<Vec<AddonRelease>>> {
        if query.driver != VANILLA_DRIVER && query.driver != FORGE_DRIVER {
            return Ok(Some(Vec::new()));
        }
        let catalog = LiteLoaderCatalog::new(self.source.clone());
        let releases = catalog.loader_versions(query.game_version).await?;
        Ok(Some(
            releases
                .into_iter()
                .map(|release| {
                    AddonRelease::new(release.game_version, release.loader_version.to_string())
                        .with_channel(release.channel)
                        .with_published_at(release.published_at)
                        .with_prerelease(release.prerelease)
                })
                .collect(),
        ))
    }

    fn check_compatibility(&self, ctx: &AddonContext<'_>) -> AddonCompatibility {
        if ctx.driver == VANILLA_DRIVER || ctx.driver == FORGE_DRIVER {
            AddonCompatibility::Compatible
        } else {
            AddonCompatibility::Incompatible {
                reason: format!(
                    "LiteLoader can only be layered onto Vanilla or Forge, not '{}'",
                    ctx.driver.id
                ),
            }
        }
    }

    async fn inspect(&self, probe: &InstanceProbe) -> Result<Option<InstalledAddon>> {
        let Some(metadata) = &probe.metadata else {
            return Ok(None);
        };
        let Some(library) = metadata
            .libraries
            .iter()
            .find(|library| library.name.starts_with(LITELOADER_LIBRARY_PREFIX))
        else {
            return Ok(None);
        };

        let version = library
            .name
            .strip_prefix(LITELOADER_LIBRARY_PREFIX)
            .map(str::to_owned);
        Ok(Some(InstalledAddon {
            id: LITELOADER_ADDON_ID.to_owned(),
            name: "LiteLoader".to_owned(),
            version,
            origin: AddonOrigin::External,
            effective_patch_mode: PatchMode::Physical,
            requires_runtime_hook: false,
        }))
    }

    async fn prepare(&self, request: &AddonPrepareRequest<'_>) -> Result<AddonPrepareOutput> {
        let version = request
            .addon_version
            .ok_or_else(|| Error::VersionRequired {
                id: LITELOADER_ADDON_ID.to_owned(),
            })?;

        let profile = self
            .source
            .profile_json(request.game_version.as_str(), version)
            .await?;

        let mut libraries = Vec::with_capacity(profile.libraries.len());
        let platform = VersionJsonPlatform::current();
        let mut download_tasks = Vec::new();

        for profile_library in profile.libraries {
            let piston_library = profile_library_to_piston(profile_library)?;
            for artifact in piston_library.version_artifacts(&platform)? {
                let target = request.instance.try_get_parent_resource(
                    VersionJsonRootResource::Libraries(Some(SafeRelativePath::parse(
                        artifact.path.as_str(),
                    )?)),
                )?;
                if !target.exists() {
                    let mut task = DownloadTask::new(artifact.url, target);
                    if let Some(size) = artifact.size {
                        task = task.size(size as u64);
                    }
                    if let Some(sha1) = artifact.sha1 {
                        task = task.sha1(sha1);
                    }
                    download_tasks.push(task);
                }
            }
            libraries.push(piston_library);
        }

        if !download_tasks.is_empty() {
            request
                .session
                .submit(DownloadBatch::new(download_tasks).group("liteloader-addon"))
                .await?;
        }

        let (game_arguments, jvm_arguments) = match profile.arguments {
            Some(args) => (args.game, args.jvm),
            None => (Vec::new(), Vec::new()),
        };

        let patch = MetadataPatch {
            priority: 10,
            main_class: Some(profile.main_class),
            add_libraries: libraries,
            game_arguments,
            jvm_arguments,
            ..Default::default()
        };

        Ok(AddonPrepareOutput {
            version: Some(version.to_owned()),
            metadata_patch: Some(patch),
            managed_files: Vec::new(),
        })
    }
}
