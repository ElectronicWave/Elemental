use std::collections::HashMap;

use elemental_schema::mojang::piston::{OperatingSystem, PistonMetaRuleArgumentRules};

use super::{OperatingSystemExt, PistonMetaRuleExt, VersionJsonPlatform, VersionJsonRuleContext};

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

fn features_are_satisfied(
    features: &HashMap<String, bool>,
    context: &VersionJsonRuleContext,
) -> bool {
    features
        .iter()
        .all(|(key, expected)| context.feature_enabled(key) == *expected)
}

fn matches_os_name(rule_name: &str, platform_name: &str) -> bool {
    if platform_name == "macos" {
        return rule_name == "macos" || rule_name == "macosx" || rule_name == "osx";
    }

    rule_name == platform_name
}
