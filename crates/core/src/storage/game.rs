use std::env::consts::OS;
use std::fs::{create_dir_all, write};
use std::io::{Error, ErrorKind, Result};
use std::path::{Path, PathBuf, absolute};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use super::version::VersionStorage;
use crate::consts::PLATFORM_NATIVES_DIR_NAME;
use crate::error::unification::UnifiedResult;
use crate::model::mojang::{
    MojangBaseUrl, PistonMetaAssetIndexObjects, PistonMetaData, PistonMetaDownload,
    PistonMetaLibraries, PistonMetaLibrariesDownloadsArtifact,
};
use crate::online::downloader::{DownloadTask, ElementalDownloader};
use crate::online::mojang::MojangService;

pub struct GameStorage {
    pub root: String, // ..../.minecraft
}

impl GameStorage {
    pub fn new(root: impl Into<String>) -> Result<Self> {
        Ok(Self {
            root: absolute(root.into())?.to_string_lossy().to_string(),
        })
    }

    pub fn new_ensure_dir(root: impl Into<String>) -> Result<Self> {
        let root = absolute(root.into())?.to_string_lossy().to_string();
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
            .to_stdio()?;

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
            .join(PLATFORM_NATIVES_DIR_NAME);
        create_dir_all(&path)?;
        Ok(path.to_string_lossy().to_string())
    }

    pub fn join<P: AsRef<Path>>(&self, path: P) -> PathBuf {
        Path::new(&self.root).join(path)
    }

    pub fn download_libraries(
        &self,
        version_name: &str,
        libraries: &Vec<PistonMetaLibraries>,
        baseurl: &MojangBaseUrl,
        token: &CancellationToken,
    ) -> Vec<Result<Option<JoinHandle<()>>>> {
        libraries
            .iter()
            .map(|library| self.download_library(version_name, library, baseurl, &token.clone()))
            .collect()
    }

    pub fn download_library(
        &self,
        version_name: &str,
        library: &PistonMetaLibraries,
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
                ElementalDownloader::shared().new_task(
                    DownloadTask::new(
                        &download.url,
                        self.get_ensure_library_path(download)?,
                        Some(version_name.to_string()),
                    ),
                    token.clone(),
                );
            }
            if OS == "macos" {
                if let Some(download) = classifiers.get("natives-osx") {
                    ElementalDownloader::shared().new_task(
                        DownloadTask::new(
                            &download.url,
                            self.get_ensure_library_path(download)?,
                            Some(version_name.to_string()),
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
            ElementalDownloader::shared().new_task(
                DownloadTask::new(
                    &url,
                    self.get_ensure_library_path(artifact)?,
                    Some(version_name.to_string()),
                ),
                token.clone(),
            );
        }

        Ok(Some(ElementalDownloader::shared().new_task(
            DownloadTask::new(url, path, Some(version_name.to_string())),
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
        version_name: &str,
        data: PistonMetaAssetIndexObjects,
        baseurl: &MojangBaseUrl,
        token: &CancellationToken,
    ) -> Result<Vec<JoinHandle<()>>> {
        let mut tasks = vec![];
        for (_, v) in data.objects {
            tasks.push(DownloadTask::new(
                baseurl.get_object_url(v.hash.clone()),
                self.get_ensure_object_path(v.hash)?,
                Some(version_name.to_string()),
            ));
        }

        Ok(ElementalDownloader::shared().new_tasks(tasks, token.clone()))
    }

    pub fn download_pistonmeta_all(&self) {
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

    pub fn version_exist(&self, version_name: impl Into<String>) -> bool {
        let name = version_name.into();
        let dir = self.join("versions").join(&name);

        dir.join(format!("{}.jar", &name)).exists() && dir.join(format!("{}.json", name)).exists()
    }

    pub fn save_pistonmeta_data(&self, version_name: &str, data: &PistonMetaData) -> Result<()> {
        let parent = self.join("versions").join(version_name);
        create_dir_all(&parent)?;

        write(
            parent.join(format!("{version_name}.json")),
            serde_json::to_string(data)?,
        )
    }

    pub fn get_version(&self, version_name: impl Into<String>) -> Result<VersionStorage> {
        let name = version_name.into();
        if self.version_exist(&name) {
            Ok(VersionStorage {
                root: self //It can be proved to be absolute path
                    .join("versions")
                    .join(name)
                    .to_string_lossy()
                    .to_string(),
            })
        } else {
            Err(Error::new(
                ErrorKind::NotFound,
                format!("Can't find a vaild version named '{name}'"),
            ))
        }
    }

    pub fn extract_version_natives(&self, version_name: &str) -> Result<()> { 
        todo!()
    }
}
