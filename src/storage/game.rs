use std::fs::{create_dir_all, write};
use std::io::{Error, ErrorKind, Result};
use std::path::{Path, PathBuf};

use tokio_util::sync::CancellationToken;

use crate::model::mojang::{
    MojangBaseUrl, PistonMetaAssetIndexObjects, PistonMetaDownload, PistonMetaLibraries,
    PistonMetaLibrariesDownloadsArtifact,
};
use crate::online::downloader::ElementalDownloader;
use crate::online::mojang::MojangService;

pub struct GameStorage {
    root: String, // ..../.minecraft
}

impl GameStorage {
    pub fn new(root: impl Into<String>) -> Self {
        Self { root: root.into() }
    }

    pub fn new_ensure_dir(root: impl Into<String>) -> Result<Self> {
        let root = root.into();
        create_dir_all(&root)?;

        Ok(Self { root })
    }

    pub fn get_ensure_object_indexes_path(&self, version_id: String) -> Result<String> {
        let parent = self.join("assets").join("indexes");

        create_dir_all(&parent)?;

        Ok(parent
            .join(format!("{version_id}.json"))
            .to_string_lossy()
            .to_string())
    }

    pub async fn get_and_save_objects_index(
        &self,
        service: &MojangService,
        version_id: String,
        asset_index_url: String,
    ) -> Result<PistonMetaAssetIndexObjects> {
        let objs = service
            .pistonmeta_assetindex_objects(asset_index_url)
            .await
            .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))?;

        let path = self.get_ensure_object_indexes_path(version_id)?;
        let data = serde_json::to_string(&objs)?;

        write(path, data)?;

        Ok(objs)
    }

    pub fn get_ensure_object_path(&self, hash: String) -> Result<String> {
        let parent = self
            .join("assets")
            .join("objects")
            .join(hash.get(0..2).unwrap());

        create_dir_all(&parent)?;

        Ok(parent.join(hash).to_string_lossy().to_string())
    }

    pub fn get_ensure_library_path(
        &self,
        library: &PistonMetaLibrariesDownloadsArtifact,
    ) -> Result<String> {
        let path = self.join("libraries").join(&library.path);
        let parent = path
            .parent()
            .ok_or(Error::new(ErrorKind::Other, "No such directory"))?;

        create_dir_all(parent)?;
        Ok(path.to_string_lossy().to_string())
    }

    pub fn get_ensure_client_path(&self, version_name: String) -> Result<String> {
        let path: PathBuf = self.join("versions").join(&version_name);
        create_dir_all(&path)?;
        Ok(path
            .join(format!("{}.jar", version_name))
            .to_string_lossy()
            .to_string())
    }

    pub fn join<P: AsRef<Path>>(&self, path: P) -> PathBuf {
        Path::new(&self.root).join(path)
    }

    pub fn download_library(
        &self,
        library: PistonMetaLibraries,
        baseurl: MojangBaseUrl,
        token: CancellationToken,
        callback: Option<fn(status: bool, url: String)>,
    ) -> Result<()> {
        // 1. Check Rules
        if let Some(rules) = &library.rules {
            if !rules.iter().all(|v| v.is_allow()) {
                return Ok(());
            }
        }

        // 2. Download Native Lib (Version) // TODO Extract
        if let Some(classifiers) = &library.downloads.classifiers {
            todo!()
        }

        // 3. Download Artifacts (.minecraft)
        let artifact = &library.downloads.artifact;
        let path = self.get_ensure_library_path(artifact)?;
        let url = artifact
            .url
            .replace("libraries.minecraft.net", &baseurl.libraries);
        ElementalDownloader::shared().new_task(url, path, token, callback);

        Ok(())
    }
    pub fn download_client(
        &self,
        version_name: String,
        download: PistonMetaDownload,
        baseurl: MojangBaseUrl,
        token: CancellationToken,
        callback: Option<fn(status: bool, url: String)>,
    ) -> Result<()> {
        let path = self.get_ensure_client_path(version_name)?;
        ElementalDownloader::shared().new_task(
            download
                .url
                .replace("piston-data.mojang.com", &baseurl.pistondata),
            path,
            token,
            callback,
        );
        Ok(())
    }

    pub fn download_objects(
        &self,
        data: PistonMetaAssetIndexObjects,
        baseurl: MojangBaseUrl,
        token: CancellationToken,
        callback: Option<fn(status: bool, url: String)>,
    ) -> Result<()> {
        let mut tasks = vec![];

        for (_, v) in data.objects {
            tasks.push((
                baseurl.get_object_url(v.hash.clone()),
                self.get_ensure_object_path(v.hash)?,
            ));
        }

        ElementalDownloader::shared().new_tasks(tasks, token.clone(), callback);

        Ok(())
    }

    pub fn download_pistonmeta_all(&self) {
        todo!()
    }

    pub fn validate_version(&self) {
        todo!()
    }

    pub fn exists_version(&self, version_name: String) -> bool {
        self.join("versions").join(version_name).exists()
    }

    pub fn get_versions(&self) -> Result<Vec<String>> {
        Ok(self
            .join("versions")
            .read_dir()?
            .into_iter()
            .filter_map(|e| {
                if e.is_err() {
                    return None;
                }

                let dir = e.as_ref().unwrap();
                let name = dir.file_name().to_string_lossy().to_string();
                if dir.path().join(format!("{}.jar", name)).exists() {
                    return Some(name);
                }
                None
            })
            .collect())
    }

    pub fn get_version_launcherenv(&self) {}
}
