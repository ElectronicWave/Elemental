use std::fs::{create_dir_all, write};
use std::io::{Error, ErrorKind, Result};
use std::path::{Path, PathBuf, absolute};

use super::version::VersionStorage;
use crate::error::unification::UnifiedResult;
use crate::model::mojang::{
    MojangBaseUrl, PistonMetaAssetIndexObjects, PistonMetaData, PistonMetaDownload,
    PistonMetaLibraries, PistonMetaLibrariesDownloadsArtifact,
};
use crate::online::downloader::{DownloadTask, ElementalDownloader};
use crate::online::mojang::MojangService;
use crate::storage::jar::JarFile;

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
        asset_index_url: String,
    ) -> Result<PistonMetaAssetIndexObjects> {
        let objs = service
            .pistonmeta_assetindex_objects(&asset_index_url)
            .await
            .to_stdio()?;
        let parent = self.join("assets").join("indexes");
        let data = serde_json::to_string(&objs)?;

        write(
            parent.join(asset_index_url.split("/").last().ok_or(Error::new(
                ErrorKind::Other,
                "Split asset index url failed!",
            ))?),
            data,
        )?;

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

    pub fn get_ensure_client_path(&self, version_name: impl Into<String>) -> Result<String> {
        let name = version_name.into();
        let path: PathBuf = self.join("versions").join(&name);
        create_dir_all(&path)?;
        Ok(path
            .join(format!("{}.jar", name))
            .to_string_lossy()
            .to_string())
    }

    pub fn join<P: AsRef<Path>>(&self, path: P) -> PathBuf {
        Path::new(&self.root).join(path)
    }
    /// Use [crate::online::downloader::ElementalDownloader::shared] to download file
    ///
    /// you can provide `version_name` and wait all tasks via [crate::online::downloader::ElementalDownloader::wait_group_tasks]
    ///
    /// if there has no task group named `version_name` in downloader, this function will create task group automatically.
    pub async fn download_version_all(
        &self,
        service: &MojangService,
        version_id: impl Into<String>,
        version_name: impl Into<String>,
    ) -> Result<()> {
        let version_name = version_name.into();
        let version_id = version_id.into();
        let launchmeta = service.launchmeta().await.unwrap();
        let pistonmeta = service
            .pistonmeta(
                launchmeta
                    .versions
                    .iter()
                    .find(|data| data.id == version_id)
                    .ok_or(Error::new(
                        ErrorKind::Other,
                        format!("Can't find version named `{}`", version_id),
                    ))?
                    .url
                    .clone(),
            )
            .await?;

        self.save_pistonmeta_data(&version_name, &pistonmeta)?;
        let objs = self
            .get_and_save_objects_index(&service, pistonmeta.asset_index.url.clone())
            .await?;

        let baseurl = &service.baseurl;
        let downloader = ElementalDownloader::shared();
        if !downloader.has_task_group(&version_name) {
            downloader.create_task_group(&version_name);
        }
        self.download_objects(&version_name, objs, baseurl)?;
        self.download_client(&version_name, &pistonmeta.downloads.client, baseurl)?;
        self.download_libraries(&version_name, &pistonmeta.libraries, baseurl);
        Ok(())
    }

    pub fn download_libraries(
        &self,
        version_name: &str,
        libraries: &Vec<PistonMetaLibraries>,
        baseurl: &MojangBaseUrl,
    ) -> Vec<Result<()>> {
        libraries
            .iter()
            .map(|library| self.download_library(version_name, library, baseurl))
            .collect()
    }

    pub fn download_library(
        &self,
        version_name: &str,
        library: &PistonMetaLibraries,
        baseurl: &MojangBaseUrl,
    ) -> Result<()> {
        // 1. Check Rules
        if let Some(rules) = &library.rules {
            if !rules.iter().all(|v| v.is_allow()) {
                return Ok(());
            }
        }

        // TODO 2. Check Feats

        // 3. Download Artifact
        let artifact = &library.downloads.artifact;
        let path = self.get_ensure_library_path(artifact)?;
        let url = artifact
            .url
            .replace("libraries.minecraft.net", &baseurl.libraries);

        ElementalDownloader::shared().add_task(DownloadTask::new(
            url,
            path,
            version_name.to_string(),
            Some(artifact.size),
        ));

        // 4. Download Native Lib (Legacy)
        if let Some(download) = library.try_get_classifiers_native_artifact() {
            ElementalDownloader::shared().add_task(DownloadTask::new(
                &download
                    .url
                    .replace("libraries.minecraft.net", &baseurl.libraries),
                self.get_ensure_library_path(download)?,
                version_name.to_string(),
                Some(artifact.size),
            ));
        }

        Ok(())
    }

    pub fn download_client(
        &self,
        version_name: &str,
        download: &PistonMetaDownload,
        baseurl: &MojangBaseUrl,
    ) -> Result<()> {
        let path = self.get_ensure_client_path(version_name)?;
        ElementalDownloader::shared().add_task(DownloadTask::new(
            download
                .url
                .replace("piston-data.mojang.com", &baseurl.pistondata),
            path,
            version_name.to_string(),
            Some(download.size),
        ));
        Ok(())
    }

    pub fn download_objects(
        &self,
        version_name: &str,
        data: PistonMetaAssetIndexObjects,
        baseurl: &MojangBaseUrl,
    ) -> Result<()> {
        let mut tasks = vec![];
        for (_, v) in data.objects {
            tasks.push(DownloadTask::new(
                baseurl.get_object_url(v.hash.clone()),
                self.get_ensure_object_path(v.hash)?,
                version_name.to_string(),
                Some(v.size),
            ));
        }

        ElementalDownloader::shared().add_tasks(tasks);
        Ok(())
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

    pub fn save_pistonmeta_data(
        &self,
        version_name: impl Into<String>,
        data: &PistonMetaData,
    ) -> Result<()> {
        let version_name = version_name.into();
        let parent = self.join("versions").join(&version_name);
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
                    .join(&name)
                    .to_string_lossy()
                    .to_string(),
                name,
            })
        } else {
            Err(Error::new(
                ErrorKind::NotFound,
                format!("Can't find a vaild version named '{name}'"),
            ))
        }
    }

    pub fn extract_version_natives(&self, version_name: impl Into<String>) -> Result<()> {
        let version_name = version_name.into();
        let libraries = self.join("libraries");
        let version = self.get_version(&version_name)?;
        let dest = version.get_ensure_natives_path()?;
        let data = version.pistonmeta()?.libraries;

        for library in data {
            if let Some(rules) = &library.rules {
                if !rules.iter().all(|v| v.is_allow()) {
                    continue;
                }
            }
            if let Some(artifact) = library.try_get_classifiers_native_artifact() {
                let src = libraries.join(&artifact.path);
                JarFile::new(src).extract_blocking(&dest)?;
            }
            if let Some(artifact) = library.try_get_native_artifact() {
                let src = libraries.join(&artifact.path);
                JarFile::new(src).extract_blocking(&dest)?;
            }
        }

        //? Check / Validate

        Ok(())
    }
}
