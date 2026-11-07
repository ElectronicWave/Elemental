use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum MinecraftVersionParseError {
    #[error("Minecraft version is empty")]
    Empty,

    #[error("invalid Minecraft version '{raw}': {reason}")]
    InvalidFormat { raw: String, reason: &'static str },

    #[error("invalid numeric component '{component}' in Minecraft version '{raw}'")]
    InvalidNumber { raw: String, component: String },

    #[error("unsupported stage '{stage}' in Minecraft version '{raw}'")]
    UnsupportedStage { raw: String, stage: String },

    #[error("unsupported year '{year}' in Minecraft version '{raw}'")]
    UnsupportedYear { raw: String, year: u32 },
}
