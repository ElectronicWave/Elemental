use crate::error::{Error, Result};

pub(crate) const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";

pub(crate) fn validate_name(name: &str) -> Result<()> {
    if name.trim().is_empty() {
        return Err(Error::TextureNameEmpty);
    }
    Ok(())
}
