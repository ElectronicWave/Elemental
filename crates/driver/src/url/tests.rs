use super::{Origin, OriginPolicy};

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

#[test]
fn resolve_preserves_base_path_without_trailing_slash() {
    let resolved = OriginPolicy::default()
        .resolve(TestOrigin::Base, "org/example/a.jar")
        .expect("base path should resolve");

    assert_eq!(resolved, "https://example.com/releases/org/example/a.jar");
}

#[test]
fn origin_suffix_requires_a_path_boundary() {
    let policy: OriginPolicy<TestOrigin> = OriginPolicy::default();
    let rewritten = policy
        .try_rewrite_origin_url("https://example.com/releases-v2/a.jar")
        .expect("url should parse");

    assert!(rewritten.is_none());
}

#[test]
fn origin_suffix_requires_a_boundary_for_trailing_slash_overrides() {
    let policy = OriginPolicy::default()
        .with_override(TestOrigin::Base, "https://example.com/releases/".to_owned())
        .expect("origin override should parse");
    let rewritten = policy
        .try_rewrite_origin_url("https://example.com/releases-v2/a.jar")
        .expect("url should parse");

    assert!(rewritten.is_none());
}
