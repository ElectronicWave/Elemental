use anyhow::{Context, Result};
use dirs::{config_dir, config_local_dir, document_dir, home_dir};
use std::path::PathBuf;
use tokio::fs::create_dir_all;

const PRESERVED_DIR: &str = ".elemental";

#[derive(Debug, Clone)]
pub enum Scope {
    Document,
    Home,
    Config,
    ConfigLocal,
    Dot,
    Custom(PathBuf),
}

impl Scope {
    pub fn path(&self) -> Option<PathBuf> {
        match self {
            Scope::Document => document_dir(),
            Scope::Home => home_dir(),
            Scope::Config => config_dir(),
            Scope::ConfigLocal => config_local_dir(),
            Scope::Dot => Some(PathBuf::from(".")),
            // May need more pathbuf here
            Scope::Custom(p) => Some(p.clone()),
        }
    }

    pub async fn get_full_path(&self, id: &str, suffix: Option<String>) -> Result<PathBuf> {
        let mut path = self.path().context("There is no valid base path")?;
        // Make sure the preserved directory exists
        path.push(PRESERVED_DIR);
        if !path.exists() {
            create_dir_all(&path).await?;
        }

        path.push(id);
        if let Some(suf) = suffix {
            path.set_extension(suf);
        }

        Ok(path)
    }
}
