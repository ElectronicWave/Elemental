use log::warn;
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::OnceLock;

static OS_RELEASE: OnceLock<Option<HashMap<Cow<'static, str>, String>>> = OnceLock::new();

pub fn get_os_release() -> &'static Option<HashMap<Cow<'static, str>, String>> {
    OS_RELEASE.get_or_init(|| match rs_release::get_os_release() {
        Ok(os_release) => Some(os_release),
        Err(e) => {
            warn!("Failed to get OS release: {}", e);
            None
        }
    })
}
