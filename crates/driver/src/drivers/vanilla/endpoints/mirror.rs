use super::{VanillaEndpoints, VanillaOrigin};
use crate::{error::Result, url::OriginPolicy};

impl VanillaEndpoints {
    pub fn mirror(
        launchermeta_origin: String,
        pistonmeta_origin: String,
        pistondata_origin: String,
        resources_origin: String,
        libraries_origin: String,
    ) -> Result<Self> {
        let policy = OriginPolicy::default()
            .with_override(VanillaOrigin::LauncherMeta, launchermeta_origin)?
            .with_override(VanillaOrigin::PistonMeta, pistonmeta_origin)?
            .with_override(VanillaOrigin::PistonData, pistondata_origin)?
            .with_override(VanillaOrigin::Resources, resources_origin)?
            .with_override(VanillaOrigin::Libraries, libraries_origin)?;
        Ok(Self::new(policy))
    }
}
