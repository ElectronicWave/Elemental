use std::fs::{File, create_dir_all, write};
use std::path::{Path, PathBuf, absolute};
use std::sync::Arc;

use tokio::process::Child;

use super::version::VersionStorage;
use crate::models::mojang::{
    MojangBaseUrl, PistonMetaAssetIndexObjects, PistonMetaData, PistonMetaDownload,
    PistonMetaLibraries, PistonMetaLibrariesDownloadsArtifact,
};
use crate::services::downloader::{DownloadTask, ElementalDownloader};
use crate::services::mojang::MojangService;
use crate::legacystorage::jar::JarFile;
use anyhow::{Context, Result, bail};

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

    pub fn get_ensure_object_indexes_path(&self, version_id: String) -> Result<PathBuf> {
        let parent = self.join("assets").join("indexes");

        create_dir_all(&parent)?;

        Ok(parent.join(format!("{version_id}.json")))
    }

    pub fn get_object_index(
        &self,
        index_id: impl Into<String>,
    ) -> Result<PistonMetaAssetIndexObjects> {
        let path = self
            .join("assets")
            .join("indexes")
            .join(format!("{}.json", index_id.into()));

        Ok(serde_json::from_reader(File::open(path)?)?)
    }

    pub async fn get_and_save_objects_index(
        &self,
        service: &MojangService,
        asset_index_url: String,
    ) -> Result<PistonMetaAssetIndexObjects> {
        let objs = service
            .pistonmeta_assetindex_objects(&asset_index_url)
            .await?;
        let parent = self.join("assets").join("indexes");
        create_dir_all(&parent)?;

        let data = serde_json::to_string(&objs)?;

        write(
            parent.join(
                asset_index_url
                    .split("/")
                    .last()
                    .context("Failed to extract asset index file name")?,
            ),
            data,
        )?;

        Ok(objs)
    }
    pub fn get_object_path(&self, hash: String) -> Option<PathBuf> {
        let path = self
            .join("assets")
            .join("objects")
            .join(hash.get(0..2).unwrap())
            .join(hash);
        if path.exists() { Some(path) } else { None }
    }

    pub fn get_ensure_object_path(&self, hash: String) -> Result<PathBuf> {
        let parent = self
            .join("assets")
            .join("objects")
            .join(hash.get(0..2).unwrap());

        create_dir_all(&parent)?;

        Ok(parent.join(hash))
    }

    pub fn get_ensure_library_path(
        &self,
        library: &PistonMetaLibrariesDownloadsArtifact,
    ) -> Result<PathBuf> {
        let path = self.join("libraries").join(&library.path);
        let parent = path
            .parent()
            .context("Failed to get library parent directory")?;

        create_dir_all(parent)?;
        Ok(path)
    }

    pub fn get_ensure_client_path(&self, version_name: impl Into<String>) -> Result<PathBuf> {
        let name = version_name.into();
        let path: PathBuf = self.join("versions").join(&name);
        create_dir_all(&path)?;
        Ok(path.join(format!("{}.jar", name)))
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
        downloader: &Arc<ElementalDownloader>,
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
                    .context(format!("Can't find version named `{}`", version_id))?
                    .url
                    .clone(),
            )
            .await?;

        self.save_pistonmeta_data(&version_name, &pistonmeta)?;
        let objs = self
            .get_and_save_objects_index(&service, pistonmeta.asset_index.url.clone())
            .await?;

        let baseurl = &service.baseurl;
        if !downloader.has_group(&version_name).await {
            downloader.create_group(&version_name).await?;
        }
        self.download_objects(&downloader, &version_name, objs, baseurl)
            .await?;
        self.download_client(
            &downloader,
            &version_name,
            &pistonmeta.downloads.client,
            baseurl,
        )
        .await?;
        self.download_libraries(&downloader, &version_name, &pistonmeta.libraries, baseurl)
            .await?;
        Ok(())
    }

    pub async fn download_libraries(
        &self,
        downloader: &Arc<ElementalDownloader>,
        version_name: &str,
        libraries: &Vec<PistonMetaLibraries>,
        baseurl: &MojangBaseUrl,
    ) -> Result<()> {
        for library in libraries {
            self.download_library(downloader.clone(), version_name, library, baseurl)
                .await?;
        }
        Ok(())
    }

    pub async fn download_library(
        &self,
        downloader: Arc<ElementalDownloader>,
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

        // 2. Download Artifact
        let artifact = &library.downloads.artifact;
        let path = self.get_ensure_library_path(artifact)?;
        let url = artifact
            .url
            .replace("libraries.minecraft.net", &baseurl.libraries);

        downloader
            .add_task(DownloadTask::new(
                url,
                path.to_string_lossy().to_string(),
                version_name.to_string(),
                Some(artifact.size),
                Some(artifact.sha1.clone()),
            ))
            .await?;

        // 3. Download Native Lib (Legacy)
        if let Some(download) = library.try_get_classifiers_native_artifact() {
            downloader
                .add_task(DownloadTask::new(
                    &download
                        .url
                        .replace("libraries.minecraft.net", &baseurl.libraries),
                    self.get_ensure_library_path(download)?
                        .to_string_lossy()
                        .to_string(),
                    version_name.to_string(),
                    Some(download.size),
                    Some(download.sha1.clone()),
                ))
                .await?;
        }

        Ok(())
    }

    pub async fn download_client(
        &self,
        downloader: &Arc<ElementalDownloader>,
        version_name: &str,
        download: &PistonMetaDownload,
        baseurl: &MojangBaseUrl,
    ) -> Result<()> {
        let path = self.get_ensure_client_path(version_name)?;
        downloader
            .add_task(DownloadTask::new(
                download
                    .url
                    .replace("piston-data.mojang.com", &baseurl.pistondata),
                path.to_string_lossy().to_string(),
                version_name.to_string(),
                Some(download.size),
                Some(download.sha1.clone()),
            ))
            .await?;
        Ok(())
    }

    pub async fn download_objects(
        &self,
        downloader: &Arc<ElementalDownloader>,
        version_name: &str,
        data: PistonMetaAssetIndexObjects,
        baseurl: &MojangBaseUrl,
    ) -> Result<()> {
        let mut tasks = vec![];
        for (_, v) in data.objects {
            tasks.push(DownloadTask::new(
                baseurl.get_object_url(v.hash.clone()),
                self.get_ensure_object_path(v.hash.clone())?
                    .to_string_lossy()
                    .to_string(),
                version_name.to_string(),
                Some(v.size),
                Some(v.hash),
            ));
        }

        downloader.add_tasks(tasks).await?;
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

        Ok(write(
            parent.join(format!("{version_name}.json")),
            serde_json::to_string(data)?,
        )?)
    }

    pub fn get_version(&self, version_name: impl Into<String>) -> Result<VersionStorage> {
        let name = version_name.into();
        if !self.version_exist(&name) {
            bail!("Can't find a valid version named '{name}'")
        }

        Ok(VersionStorage {
            root: self //It can be proved to be absolute path
                .join("versions")
                .join(&name)
                .to_string_lossy()
                .to_string(),
            name,
        })
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

    pub fn launch_version(
        &self,
        player_name: impl Into<String>,
        version_name: impl Into<String>,
        executable: impl Into<String>,
        extra_args: impl IntoIterator<Item = String>,
    ) -> Result<Child> {
        Ok(self
            .get_version(version_name)?
            .launch(player_name, &self, executable, extra_args)?)
    }
}
