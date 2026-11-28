use std::collections::HashMap;

use super::OriginPolicy;

/// Layered origin override registry supporting a global policy plus scoped overrides.
#[derive(Debug, Clone, Default)]
pub struct OriginPolicyMap {
    global: OriginPolicy,
    scoped: HashMap<String, OriginPolicy>,
}

impl OriginPolicyMap {
    /// Returns the global policy applied before any scoped overrides.
    pub fn global(&mut self) -> &mut OriginPolicy {
        &mut self.global
    }

    /// Returns a mutable scoped policy identified by `key`.
    pub fn scope(&mut self, key: impl Into<String>) -> &mut OriginPolicy {
        self.scoped.entry(key.into()).or_default()
    }

    /// Resolves the effective policy for `key` by layering scoped overrides on top of [`Self::global`].
    pub fn policy_for(&self, key: &str) -> OriginPolicy {
        match self.scoped.get(key) {
            Some(overrides) => self.global.merged_with(overrides),
            None => self.global.clone(),
        }
    }
}
