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

/// A decoded skin texture in row-major RGBA8 format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkinPixels<'a> {
    pub width: u16,
    pub height: u16,
    /// Row-major RGBA8 pixels, `width * height * 4` bytes.
    pub rgba: &'a [u8],
}

/// Unused arm pixels of a slim 64x64 skin.
const SLIM_ARM_GAPS: [(u16, u16, u16, u16); 4] = [
    (50, 16, 2, 4),
    (54, 20, 2, 12),
    (42, 48, 2, 4),
    (46, 52, 2, 12),
];

impl SkinModel {
    /// Infers the default model of an already-decoded skin texture.
    ///
    /// Supports the vanilla 64x32 and 64x64 textures. Returns `None` when the
    /// dimensions or pixel buffer are unsupported. This is only a heuristic;
    /// provider metadata and explicit user choices take precedence.
    pub fn infer(texture: SkinPixels<'_>) -> Option<Self> {
        let pixels = usize::from(texture.width).checked_mul(usize::from(texture.height))?;
        if texture.rgba.len() != pixels.checked_mul(4)? {
            return None;
        }

        match (texture.width, texture.height) {
            (64, 32) => Some(Self::Classic),
            (64, 64) => Some(infer_64x64(texture)),
            _ => None,
        }
    }
}

#[derive(Clone, Copy)]
struct GapSummary {
    transparent: bool,
    black: bool,
    white: bool,
}

fn infer_64x64(texture: SkinPixels<'_>) -> SkinModel {
    let gaps = SLIM_ARM_GAPS.map(|region| summarize_gap(texture, region));
    let has_transparency = gaps.iter().any(|gap| gap.transparent);
    let all_black = gaps.iter().all(|gap| gap.black);
    let all_white = gaps.iter().all(|gap| gap.white);

    if has_transparency || all_black || all_white {
        SkinModel::Slim
    } else {
        SkinModel::Classic
    }
}

fn summarize_gap(
    texture: SkinPixels<'_>,
    (x, y, width, height): (u16, u16, u16, u16),
) -> GapSummary {
    let stride = usize::from(texture.width) * 4;
    let mut summary = GapSummary {
        transparent: false,
        black: true,
        white: true,
    };

    for row in 0..usize::from(height) {
        for column in 0..usize::from(width) {
            let offset = (usize::from(y) + row) * stride + (usize::from(x) + column) * 4;
            let red = texture.rgba[offset];
            let green = texture.rgba[offset + 1];
            let blue = texture.rgba[offset + 2];
            let alpha = texture.rgba[offset + 3];

            summary.transparent |= alpha != u8::MAX;
            summary.black &= red == 0 && green == 0 && blue == 0 && alpha == u8::MAX;
            summary.white &=
                red == u8::MAX && green == u8::MAX && blue == u8::MAX && alpha == u8::MAX;
        }
    }

    summary
}

#[cfg(test)]
mod tests;

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
