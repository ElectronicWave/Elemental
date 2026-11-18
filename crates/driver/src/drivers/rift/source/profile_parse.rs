use elemental_schema::fabric::ProfileJson;

use crate::error::{Error, Result};

pub(super) fn parse_profile_json(raw_profile: &str) -> Result<ProfileJson> {
    serde_json::from_str(raw_profile).map_err(|source| Error::DecodeRiftProfile { source })
}
