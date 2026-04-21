use std::collections::HashMap;

use super::{OperatingSystem, PistonMetaRuleArgumentRules, VersionJsonPlatform};

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

impl OperatingSystemExt for OperatingSystem {
    fn matches_platform(&self, platform: &VersionJsonPlatform) -> bool {
        if let Some(name) = &self.name
            && !matches_os_name(name, platform.os())
        {
            return false;
        }

        if let Some(arch) = &self.arch
            && arch != platform.arch()
        {
            return false;
        }

        true
    }
}

pub trait PistonMetaRuleExt {
    fn is_allowed(&self, context: &VersionJsonRuleContext) -> bool;
}

impl PistonMetaRuleExt for PistonMetaRuleArgumentRules {
    fn is_allowed(&self, context: &VersionJsonRuleContext) -> bool {
        let mut action = self.action == "allow";

        if let Some(os) = &self.os
            && !os.matches_platform(context.platform())
        {
            action = !action;
        }

        if let Some(features) = &self.features
            && !features_are_satisfied(features, context)
        {
            action = !action;
        }

        action
    }
}

pub trait PistonMetaRulesExt {
    fn are_allowed(&self, context: &VersionJsonRuleContext) -> bool;
}

impl PistonMetaRulesExt for [PistonMetaRuleArgumentRules] {
    fn are_allowed(&self, context: &VersionJsonRuleContext) -> bool {
        self.iter().all(|rule| rule.is_allowed(context))
    }
}

fn matches_os_name(rule_name: &str, platform_name: &str) -> bool {
    if platform_name == "macos" {
        return rule_name == "macos" || rule_name == "macosx" || rule_name == "osx";
    }

    rule_name == platform_name
}

fn features_are_satisfied(
    features: &HashMap<String, bool>,
    context: &VersionJsonRuleContext,
) -> bool {
    features
        .iter()
        .all(|(key, expected)| context.feature_enabled(key) == *expected)
}
