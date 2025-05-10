use std::fs::create_dir_all;
use std::io::{Error, ErrorKind, Result};
use std::path::{Path, PathBuf};

use tokio_util::sync::CancellationToken;

use crate::model::mojang::{
    MojangBaseUrl, PistonMetaAssetIndexObjects, PistonMetaLibrariesDownloadsArtifact,
};
use crate::online::downloader::ElementalDownloader;

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
    ) -> CancellationToken {
        let token = CancellationToken::new();
        ElementalDownloader::shared().new_tasks(
            data.objects
                .into_iter()
                .map(|(_, v)| {
                    (
                        baseurl.get_object_url(v.hash.clone()),
                        self.get_ensure_object_path(v.hash).unwrap(),
                    )
                }) // TODO Remove unwrap here
                .collect(),
            token.clone(),
            Some(|status, url| println!("{url}: {status}",)),
        );

        token
    }

    pub fn download_pistonmeta_all(&self) {
        todo!()
    }
    pub fn validate_version() {
        todo!()
    }
}

#[cfg(test)]
mod test_storage {
    use crate::model::mojang::MojangBaseUrl;

    use super::GameStorage;
    #[tokio::test]
    async fn test_storage() {
        use crate::online::mojang::MojangService;
        let service = MojangService::default();
        let launchmeta = service.launchmeta().await.unwrap();
        let pistonmeta = service
            .pistonmeta(launchmeta.versions.first().unwrap().url.clone())
            .await
            .unwrap();
        let objs = service
            .pistonmeta_assetindex_objects(pistonmeta.asset_index.url.clone())
            .await
            .unwrap();
        let storage = GameStorage::new_ensure_dir(".minecraft").unwrap();
        storage.download_objects(objs, MojangBaseUrl::default());
        loop {}
    }
}
