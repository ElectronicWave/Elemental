use super::{Origin, OriginError, OriginPolicy, OriginPolicyMap};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum TestOrigin {
    Base,
}

impl Origin for TestOrigin {
    fn canonical(self) -> &'static str {
        "https://example.com/releases"
    }

    fn all() -> &'static [Self] {
        &[Self::Base]
    }
}

impl AsRef<str> for TestOrigin {
    fn as_ref(&self) -> &str {
        self.canonical()
    }
}

#[test]
fn resolve_preserves_base_path_without_trailing_slash() {
    let resolved = OriginPolicy::default()
        .resolve(TestOrigin::Base, "org/example/a.jar")
        .expect("base path should resolve");

    assert_eq!(resolved, "https://example.com/releases/org/example/a.jar");
}

#[test]
fn resolve_rejects_parent_traversal() {
    let err = OriginPolicy::default()
        .resolve(TestOrigin::Base, "../secret")
        .unwrap_err();
    assert!(matches!(err, OriginError::InvalidRelativePath { .. }));
}

#[test]
fn origin_suffix_requires_a_path_boundary() {
    let policy = OriginPolicy::default();
    let rewritten = policy
        .try_rewrite_origin_url(
            &[TestOrigin::Base.canonical()],
            "https://example.com/releases-v2/a.jar",
        )
        .expect("url should parse");

    assert!(rewritten.is_none());
}

#[test]
fn origin_suffix_requires_a_boundary_for_trailing_slash_overrides() {
    let policy = OriginPolicy::default()
        .with_override(TestOrigin::Base, "https://example.com/releases/".to_owned())
        .expect("origin override should parse");
    let rewritten = policy
        .try_rewrite_origin_url(
            &[TestOrigin::Base.canonical()],
            "https://example.com/releases-v2/a.jar",
        )
        .expect("url should parse");

    assert!(rewritten.is_none());
}

#[test]
fn origin_policy_map_layers_scope_over_global() {
    let mut map = OriginPolicyMap::default();
    map.global()
        .override_origin(
            TestOrigin::Base,
            "https://mirror.example.org/global".to_owned(),
        )
        .unwrap();
    map.scope("custom")
        .override_origin(
            TestOrigin::Base,
            "https://mirror.example.org/custom".to_owned(),
        )
        .unwrap();

    assert_eq!(
        map.policy_for("other")
            .resolve(TestOrigin::Base, "a.jar")
            .unwrap(),
        "https://mirror.example.org/global/a.jar"
    );
    assert_eq!(
        map.policy_for("custom")
            .resolve(TestOrigin::Base, "a.jar")
            .unwrap(),
        "https://mirror.example.org/custom/a.jar"
    );
}
