use std::collections::HashSet;

use elemental_schema::mojang::piston::{PistonMetaAssetIndexObjects, PistonMetaData};

use crate::{
    error::{Error, Result},
    families::version_json::{
        InstanceStorage, VersionJsonAssetIndexStorageExt, VersionJsonMetadataStorageExt,
        VersionJsonRuleContext, classpath::resolve_classpath,
    },
};

use super::VersionJsonInstallStatus;

mod metadata;
mod prepare;

#[derive(Debug, Clone)]
pub struct ResolvedVersionJsonMetadata {
    pub metadata: PistonMetaData,
    pub asset_index_objects: PistonMetaAssetIndexObjects,
}

#[derive(Debug, Clone)]
pub struct ResolvedVersionJsonInstance {
    pub version: InstanceStorage,
    pub metadata: PistonMetaData,
    pub asset_index_objects: PistonMetaAssetIndexObjects,
}

#[derive(Debug, Clone)]
pub struct PreparedVersionJsonInstance {
    pub version: InstanceStorage,
    pub metadata: PistonMetaData,
    pub install_status: VersionJsonInstallStatus,
}

impl ResolvedVersionJsonInstance {
    pub fn load(version: InstanceStorage) -> Result<Self> {
        let metadata = version.metadata()?;
        let asset_index_objects = version
            .parent
            .asset_index_objects(metadata.asset_index.id.as_str())?;
        Ok(Self {
            version,
            metadata,
            asset_index_objects,
        })
    }

    pub async fn into_prepared(self) -> Result<PreparedVersionJsonInstance> {
        let status = self.status().await?;
        self.into_prepared_with_status(status)
    }

    pub(super) fn into_prepared_with_status(
        self,
        status: VersionJsonInstallStatus,
    ) -> Result<PreparedVersionJsonInstance> {
        if !status.is_ready() {
            let version_name = self
                .version
                .name()
                .ok_or_else(|| Error::MissingVersionName {
                    path: self.version.path.clone(),
                })?;
            return Err(Error::VersionNotPrepared {
                version: version_name,
                status: format!("{status:?}"),
            });
        }
        Ok(PreparedVersionJsonInstance {
            install_status: status,
            version: self.version,
            metadata: self.metadata,
        })
    }

    pub fn required_java_major_version(&self) -> usize {
        self.metadata.java_version.major_version
    }
}

impl PreparedVersionJsonInstance {
    pub fn instance_root(&self) -> &std::path::Path {
        self.version.path.as_path()
    }

    pub fn required_java_major_version(&self) -> usize {
        self.metadata.java_version.major_version
    }

    pub fn classpath(&self) -> Result<Vec<std::path::PathBuf>> {
        resolve_classpath(
            &self.version,
            &self.metadata,
            &VersionJsonRuleContext::current(),
            &HashSet::new(),
        )
    }
}
