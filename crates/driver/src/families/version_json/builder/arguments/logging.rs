use std::collections::HashMap;

use elemental_infra::SafeRelativePath;
use elemental_schema::mojang::piston::PistonMetaData;

use crate::{
    error::Result,
    families::version_json::{VersionJsonRootLayout, VersionJsonRootResource},
};

use super::super::VersionJsonLaunchBuilder;
use super::common::resolve_root_resource;

pub(super) fn logging_arguments<L, VL>(
    builder: &VersionJsonLaunchBuilder<L, VL>,
    metadata: &PistonMetaData,
) -> Result<Option<Vec<String>>>
where
    L: VersionJsonRootLayout,
    VL: crate::families::version_json::VersionJsonInstanceLayout,
{
    let Some(client) = metadata
        .logging
        .as_ref()
        .and_then(|logging| logging.client.as_ref())
    else {
        return Ok(None);
    };

    let log_config_path = resolve_root_resource(
        builder,
        VersionJsonRootResource::AssetLogConfigs(Some(SafeRelativePath::parse(
            client.file.id.as_str(),
        )?)),
    )?;
    if !log_config_path.exists() {
        return Ok(None);
    }

    Ok(Some(builder.inner.apply_with(
        vec![client.argument.clone()],
        &HashMap::from([(
            "path".to_owned(),
            log_config_path.to_string_lossy().to_string(),
        )]),
    )?))
}
