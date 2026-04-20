use std::collections::HashMap;

use super::{MojangPlatform, OperatingSystem, PistonMetaRuleArgumentRules};

#[derive(Debug, Clone)]
pub struct MojangRuleContext {
    platform: MojangPlatform,
    features: HashMap<String, bool>,
}

impl MojangRuleContext {
    pub fn new(platform: MojangPlatform, features: HashMap<String, bool>) -> Self {
        Self { platform, features }
    }

    pub fn current() -> Self {
        Self::new(MojangPlatform::current(), HashMap::new())
    }

    pub fn platform(&self) -> &MojangPlatform {
        &self.platform
    }

    pub fn feature_enabled(&self, key: &str) -> bool {
        self.features.get(key).copied().unwrap_or(false)
    }
}

pub trait OperatingSystemExt {
    fn matches_platform(&self, platform: &MojangPlatform) -> bool;
}

impl OperatingSystemExt for OperatingSystem {
    fn matches_platform(&self, platform: &MojangPlatform) -> bool {
        if let Some(name) = &self.name {
            if !matches_os_name(name, platform.os()) {
                return false;
            }
        }

        if let Some(arch) = &self.arch {
            if arch != platform.arch() {
                return false;
            }
        }

        true
    }
}

pub trait PistonMetaRuleExt {
    fn is_allowed(&self, context: &MojangRuleContext) -> bool;
}

impl PistonMetaRuleExt for PistonMetaRuleArgumentRules {
    fn is_allowed(&self, context: &MojangRuleContext) -> bool {
        let mut action = self.action == "allow";

        if let Some(os) = &self.os {
            if !os.matches_platform(context.platform()) {
                action = !action;
            }
        }

        if let Some(features) = &self.features {
            if !features_are_satisfied(features, context) {
                action = !action;
            }
        }

        action
    }
}

pub trait PistonMetaRulesExt {
    fn are_allowed(&self, context: &MojangRuleContext) -> bool;
}

impl PistonMetaRulesExt for [PistonMetaRuleArgumentRules] {
    fn are_allowed(&self, context: &MojangRuleContext) -> bool {
        self.iter().all(|rule| rule.is_allowed(context))
    }
}

fn matches_os_name(rule_name: &str, platform_name: &str) -> bool {
    if platform_name == "macos" {
        return rule_name == "macos" || rule_name == "macosx" || rule_name == "osx";
    }

    rule_name == platform_name
}

fn features_are_satisfied(features: &HashMap<String, bool>, context: &MojangRuleContext) -> bool {
    features
        .iter()
        .all(|(key, expected)| context.feature_enabled(key) == *expected)
}
