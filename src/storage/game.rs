use std::fs::{create_dir_all, write};
use std::io::{Error, ErrorKind, Result};
use std::path::{Path, PathBuf};

use tokio_util::sync::CancellationToken;

use crate::model::mojang::{
    MojangBaseUrl, PistonMetaAssetIndexObjects, PistonMetaLibrariesDownloadsArtifact,
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
        if let Err(err) = create_dir_all(&root) {
            Err(err)
        } else {
            Ok(Self { root })
        }
    }

    pub fn get_ensure_object_indexes_path(&self, version_id: String) -> Result<String> {
        let parent = self.join("assets").join("indexes");

        if let Err(err) = create_dir_all(parent.clone()) {
            Err(err)
        } else {
            Ok(parent
                .join(format!("{version_id}.json"))
                .to_string_lossy()
                .to_string())
        }
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

        if let Err(err) = create_dir_all(parent.clone()) {
            Err(err)
        } else {
            Ok(parent.join(hash).to_string_lossy().to_string())
        }
    }

    pub fn get_natives_path(&self) -> String {
        todo!()
    }

    pub fn get_ensure_library_path(
        &self,
        library: PistonMetaLibrariesDownloadsArtifact,
    ) -> Result<String> {
        let path = self.join("libraries").join(&library.path);
        let path_parent = path.parent();

        if let None = path_parent {
            return Err(Error::new(ErrorKind::Other, "No such directory"));
        }

        if let Err(err) = create_dir_all(path_parent.unwrap()) {
            Err(err)
        } else {
            Ok(path.to_string_lossy().to_string())
        }
    }

    pub fn join<P: AsRef<Path>>(&self, path: P) -> PathBuf {
        Path::new(&self.root).join(path)
    }

    pub fn download_version(&self) {
        todo!()
    }

    pub fn download_objects(
        &self,
        data: PistonMetaAssetIndexObjects,
        baseurl: MojangBaseUrl,
        callback: Option<fn(status: bool, url: String)>,
    ) -> Result<CancellationToken> {
        let token = CancellationToken::new();
        let mut tasks = vec![];

        for (_, v) in data.objects {
            tasks.push((
                baseurl.get_object_url(v.hash.clone()),
                self.get_ensure_object_path(v.hash)?,
            ));
        }

        ElementalDownloader::shared().new_tasks(tasks, token.clone(), callback);

        Ok(token)
    }

    pub fn download_pistonmeta_all(&self) {
        todo!()
    }
    pub fn validate_version() {
        todo!()
    }
    pub fn get_versions() {
        todo!()
    }
}
