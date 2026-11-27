use async_trait::async_trait;
use elemental_driver::families::version_json::MetadataPatch;

use crate::{
    addon::{
        Addon, AddonCompatibility, AddonContext, AddonDescriptor, AddonPrepareOutput,
        AddonPrepareRequest,
    },
    error::Result,
};

/// Addon wrapping a user-provided declarative [`MetadataPatch`].
#[derive(Debug, Clone)]
pub struct CustomPatchAddon {
    descriptor: AddonDescriptor,
    version: Option<String>,
    patch: MetadataPatch,
    allowed_drivers: Option<Vec<&'static str>>,
}

impl CustomPatchAddon {
    pub fn new(
        id: impl Into<std::borrow::Cow<'static, str>>,
        name: impl Into<std::borrow::Cow<'static, str>>,
        patch: MetadataPatch,
    ) -> Self {
        Self {
            descriptor: AddonDescriptor::new(id, name),
            version: None,
            patch,
            allowed_drivers: None,
        }
    }

    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    pub fn with_allowed_drivers(mut self, drivers: Vec<&'static str>) -> Self {
        self.allowed_drivers = Some(drivers);
        self
    }
}

#[async_trait]
impl Addon for CustomPatchAddon {
    fn descriptor(&self) -> AddonDescriptor {
        self.descriptor.clone()
    }

    fn check_compatibility(&self, ctx: &AddonContext<'_>) -> AddonCompatibility {
        if let Some(allowed) = &self.allowed_drivers
            && !allowed.contains(&ctx.driver.id)
        {
            return AddonCompatibility::Incompatible {
                reason: format!(
                    "driver '{}' is not in allowed drivers {:?}",
                    ctx.driver.id, allowed
                ),
            };
        }
        AddonCompatibility::Compatible
    }

    async fn prepare(&self, request: &AddonPrepareRequest<'_>) -> Result<AddonPrepareOutput> {
        Ok(AddonPrepareOutput {
            version: request
                .addon_version
                .map(str::to_owned)
                .or_else(|| self.version.clone()),
            metadata_patch: Some(self.patch.clone()),
            managed_files: Vec::new(),
        })
    }
}
