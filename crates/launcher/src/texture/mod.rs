mod index;
mod store;

pub(crate) use index::{TextureAsset, TextureId, TextureIndex, TextureSelection};
pub(crate) use store::TextureStore;

use crate::error::{Error, Result};

const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";

pub(crate) fn validate_name(name: &str) -> Result<()> {
    if name.trim().is_empty() {
        return Err(Error::TextureNameEmpty);
    }
    Ok(())
}
