#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VersionJsonDownloadStatus {
    pub metadata_persisted: bool,
    pub asset_index_persisted: bool,
    pub version_artifacts_ready: bool,
    pub assets_ready: bool,
}

impl VersionJsonDownloadStatus {
    pub fn is_persisted(&self) -> bool {
        self.metadata_persisted && self.asset_index_persisted
    }

    pub fn is_downloaded(&self) -> bool {
        self.is_persisted() && self.version_artifacts_ready && self.assets_ready
    }

    pub fn with_natives_extracted(self, natives_extracted: bool) -> VersionJsonInstallStatus {
        VersionJsonInstallStatus {
            metadata_persisted: self.metadata_persisted,
            asset_index_persisted: self.asset_index_persisted,
            version_artifacts_ready: self.version_artifacts_ready,
            assets_ready: self.assets_ready,
            natives_extracted,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VersionJsonInstallStatus {
    pub metadata_persisted: bool,
    pub asset_index_persisted: bool,
    pub version_artifacts_ready: bool,
    pub assets_ready: bool,
    pub natives_extracted: bool,
}

impl VersionJsonInstallStatus {
    pub fn download_status(&self) -> VersionJsonDownloadStatus {
        VersionJsonDownloadStatus {
            metadata_persisted: self.metadata_persisted,
            asset_index_persisted: self.asset_index_persisted,
            version_artifacts_ready: self.version_artifacts_ready,
            assets_ready: self.assets_ready,
        }
    }

    pub fn is_persisted(&self) -> bool {
        self.download_status().is_persisted()
    }

    pub fn is_downloaded(&self) -> bool {
        self.download_status().is_downloaded()
    }

    pub fn is_ready(&self) -> bool {
        self.is_downloaded() && self.natives_extracted
    }
}
