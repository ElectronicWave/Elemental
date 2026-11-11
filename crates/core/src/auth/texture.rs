use serde::{Deserialize, Serialize};

/// Texture kinds that can be read from or changed on a Minecraft profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextureType {
    Skin,
    Cape,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SkinModel {
    #[default]
    Classic,
    Slim,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Skin {
    pub url: String,
    pub model: SkinModel,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cape {
    /// Selectable cape identifier when the provider exposes a cape inventory.
    pub id: Option<String>,
    pub alias: Option<String>,
    pub url: String,
    pub active: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileTextures {
    pub skin: Option<Skin>,
    /// Capes available for selection. Yggdrasil reports only the active texture.
    pub capes: Vec<Cape>,
    /// Texture kinds this account accepts through upload.
    pub uploadable: Vec<TextureType>,
}

impl ProfileTextures {
    pub fn active_cape(&self) -> Option<&Cape> {
        self.capes.iter().find(|cape| cape.active)
    }
}
