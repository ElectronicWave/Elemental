use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::OnceLock;

pub type OsReleaseVariables = HashMap<Cow<'static, str>, String>;
static OS_RELEASE: OnceLock<Result<OsReleaseVariables, ()>> = OnceLock::new();

pub fn get_os_release() -> Option<&'static OsReleaseVariables> {
    let result = OS_RELEASE.get_or_init(|| {
        rs_release::get_os_release().map_err(|e| {
            tracing::warn!("Failed to get OS release:{}", e);
        })
    });
    result.as_ref().ok()
}
