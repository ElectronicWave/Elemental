use crate::{
    error::Result,
    url::{Origin, OriginPolicy},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RiftOrigin {
    GitHubApi,
    GitHubReleases,
    SpongeMaven,
    MavenCentral,
}

#[derive(Debug, Clone)]
pub struct RiftEndpoints {
    origin_policy: OriginPolicy<RiftOrigin>,
}

impl Default for RiftEndpoints {
    fn default() -> Self {
        Self::official()
    }
}

impl Origin for RiftOrigin {
    fn canonical(self) -> &'static str {
        match self {
            Self::GitHubApi => "https://api.github.com",
            Self::GitHubReleases => "https://github.com",
            Self::SpongeMaven => "https://repo.spongepowered.org/maven",
            Self::MavenCentral => "https://repo.maven.apache.org/maven2",
        }
    }

    fn all() -> &'static [Self] {
        const ALL: &[RiftOrigin] = &[
            RiftOrigin::GitHubApi,
            RiftOrigin::GitHubReleases,
            RiftOrigin::SpongeMaven,
            RiftOrigin::MavenCentral,
        ];
        ALL
    }
}

impl RiftEndpoints {
    pub fn new(origin_policy: OriginPolicy<RiftOrigin>) -> Self {
        Self { origin_policy }
    }

    pub fn official() -> Self {
        Self::new(OriginPolicy::default())
    }

    pub fn releases_url(&self) -> Result<String> {
        self.origin_policy.resolve_segments(
            RiftOrigin::GitHubApi,
            ["repos", "DimensionalDevelopment", "Rift", "releases"],
        )
    }

    pub fn release_asset_url(&self, tag_name: &str, asset_name: &str) -> Result<String> {
        self.origin_policy.resolve_segments(
            RiftOrigin::GitHubReleases,
            [
                "DimensionalDevelopment",
                "Rift",
                "releases",
                "download",
                tag_name,
                asset_name,
            ],
        )
    }

    pub fn rewrite_upstream(&self, raw_url: &str) -> Result<String> {
        self.origin_policy.rewrite_known_origin_url_or_keep(raw_url)
    }

    pub fn sponge_maven_url(&self) -> Result<String> {
        self.origin_policy.resolve(RiftOrigin::SpongeMaven, "")
    }

    pub fn maven_central_url(&self) -> Result<String> {
        self.origin_policy.resolve(RiftOrigin::MavenCentral, "")
    }
}
