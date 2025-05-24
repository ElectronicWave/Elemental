use std::env::consts::{ARCH, OS};
use std::fs::{create_dir_all, write};
use std::io::{Error, ErrorKind, Result};
use std::path::{Path, PathBuf};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::model::mojang::{
    MojangBaseUrl, PistonMetaAssetIndexObjects, PistonMetaData, PistonMetaDownload,
    PistonMetaLibraries, PistonMetaLibrariesDownloadsArtifact,
};
use crate::online::downloader::{DownloadTask, DownloadTaskCallback, ElementalDownloader};
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

    pub fn get_ensure_client_path(&self, version_name: &str) -> Result<String> {
        let path: PathBuf = self.join("versions").join(version_name);
        create_dir_all(&path)?;
        Ok(path
            .join(format!("{}.jar", version_name))
            .to_string_lossy()
            .to_string())
    }
    pub fn get_ensure_version_natives_path<P: AsRef<Path>>(
        &self,
        version_name: P,
    ) -> Result<String> {
        let path = self
            .join("versions")
            .join(version_name)
            .join(format!("natives-{}-{}", OS, ARCH));
        create_dir_all(&path)?;
        Ok(path.to_string_lossy().to_string())
    }

    pub fn join<P: AsRef<Path>>(&self, path: P) -> PathBuf {
        Path::new(&self.root).join(path)
    }

    fn native_library_task(
        save_path: String,
        extract_path: String,
        download: &PistonMetaLibrariesDownloadsArtifact,
    ) -> DownloadTask {
        DownloadTask::new_callback(
            download.url.clone(),
            save_path.clone(),
            None,
            DownloadTaskCallback::NATIVELIB(save_path, extract_path, vec![]),
        )
    }
    pub fn download_libraries(
        &self,
        libraries: Vec<PistonMetaLibraries>,
        version_name: &str,
        baseurl: &MojangBaseUrl,
        token: &CancellationToken,
    ) -> Vec<Result<Option<JoinHandle<()>>>> {
        libraries
            .iter()
            .map(|library| self.download_library(library, version_name, baseurl, &token.clone()))
            .collect()
    }
    pub fn download_library(
        &self,
        library: &PistonMetaLibraries,
        version_name: &str,
        baseurl: &MojangBaseUrl,
        token: &CancellationToken,
    ) -> Result<Option<JoinHandle<()>>> {
        // 1. Check Rules
        if let Some(rules) = &library.rules {
            if !rules.iter().all(|v| v.is_allow()) {
                return Ok(None);
            }
        }

        // 2. Download Native Lib (Legacy)
        if let Some(classifiers) = &library.downloads.classifiers {
            if let Some(download) = classifiers.get(&format!("natives-{}", OS)) {
                let path = self.get_ensure_library_path(download)?;
                ElementalDownloader::shared().new_task(
                    Self::native_library_task(
                        path,
                        self.get_ensure_version_natives_path(&version_name)?,
                        download,
                    ),
                    token.clone(),
                );
            }
            if OS == "macos" {
                if let Some(download) = classifiers.get("natives-osx") {
                    let path = self.get_ensure_library_path(download)?;
                    ElementalDownloader::shared().new_task(
                        Self::native_library_task(
                            path,
                            self.get_ensure_version_natives_path(&version_name)?,
                            download,
                        ),
                        token.clone(),
                    );
                }
            }
        }

        // 3. Download Artifacts (.minecraft)
        let artifact = &library.downloads.artifact;
        let path = self.get_ensure_library_path(artifact)?;
        let url = artifact
            .url
            .replace("libraries.minecraft.net", &baseurl.libraries);

        // 4。 Latest Natives File
        if artifact.path.ends_with(&format!("-natives-{}.jar", OS))
            || OS == "macos" && artifact.path.ends_with("-natives-osx.jar")
        {
            let path = self.get_ensure_library_path(artifact)?;
            ElementalDownloader::shared().new_task(
                Self::native_library_task(
                    path,
                    self.get_ensure_version_natives_path(&version_name)?,
                    artifact,
                ),
                token.clone(),
            );
        }

        Ok(Some(ElementalDownloader::shared().new_task(
            DownloadTask::new(url, path, None),
            token.clone(),
        )))
    }
    pub fn download_client(
        &self,
        version_name: &str,
        download: &PistonMetaDownload,
        baseurl: &MojangBaseUrl,
        token: &CancellationToken,
    ) -> Result<JoinHandle<()>> {
        let path = self.get_ensure_client_path(version_name)?;
        Ok(ElementalDownloader::shared().new_task(
            DownloadTask::new(
                download
                    .url
                    .replace("piston-data.mojang.com", &baseurl.pistondata),
                path,
                None,
            ),
            token.clone(),
        ))
    }

    pub fn download_objects(
        &self,
        data: PistonMetaAssetIndexObjects,
        baseurl: &MojangBaseUrl,
        token: &CancellationToken,
    ) -> Result<Vec<JoinHandle<()>>> {
        let mut tasks = vec![];

        for (_, v) in data.objects {
            tasks.push(DownloadTask::new(
                baseurl.get_object_url(v.hash.clone()),
                self.get_ensure_object_path(v.hash)?,
                None,
            ));
        }

        Ok(ElementalDownloader::shared().new_tasks(tasks, token.clone()))
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
                if dir.path().join(format!("{}.jar", name)).exists()
                    && dir.path().join(format!("{}.json", name)).exists()
                {
                    return Some(name);
                }
                None
            })
            .collect())
    }

    pub fn save_pistonmeta_data(&self, version_name: &str, data: &PistonMetaData) -> Result<()> {
        write(
            self.join("versions")
                .join(version_name)
                .join(format!("{version_name}.json")),
            serde_json::to_string(data)?,
        )
    }

    pub fn get_version_launcherenv(&self) {}
}
