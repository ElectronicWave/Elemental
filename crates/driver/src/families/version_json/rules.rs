use std::collections::HashMap;

use elemental_schema::mojang::piston::PistonMetaRuleArgumentRules;

use super::VersionJsonPlatform;

mod argument;

#[derive(Debug, Clone)]
pub struct VersionJsonRuleContext {
    platform: VersionJsonPlatform,
    features: HashMap<String, bool>,
}

impl VersionJsonRuleContext {
    pub fn new(platform: VersionJsonPlatform, features: HashMap<String, bool>) -> Self {
        Self { platform, features }
    }

    pub fn current() -> Self {
        Self::new(VersionJsonPlatform::current(), HashMap::new())
    }

    pub fn platform(&self) -> &VersionJsonPlatform {
        &self.platform
    }

    pub fn feature_enabled(&self, key: &str) -> bool {
        self.features.get(key).copied().unwrap_or(false)
    }
}

pub trait OperatingSystemExt {
    fn matches_platform(&self, platform: &VersionJsonPlatform) -> bool;
}

pub trait PistonMetaRuleExt {
    fn is_allowed(&self, context: &VersionJsonRuleContext) -> bool;
}

pub trait PistonMetaRulesExt {
    fn are_allowed(&self, context: &VersionJsonRuleContext) -> bool;
}

impl PistonMetaRulesExt for [PistonMetaRuleArgumentRules] {
    fn are_allowed(&self, context: &VersionJsonRuleContext) -> bool {
        self.iter().all(|rule| rule.is_allowed(context))
    }
}
