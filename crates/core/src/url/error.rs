use std::error::Error as StdError;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum OriginError {
    #[error("{context}: {url}")]
    InvalidUrl {
        context: String,
        url: String,
        #[source]
        source: Box<dyn StdError + Send + Sync>,
    },

    #[error("{context}: {base_url} + {path}")]
    JoinUrl {
        context: String,
        base_url: String,
        path: String,
        #[source]
        source: Box<dyn StdError + Send + Sync>,
    },

    #[error("invalid relative url path: '{path}'")]
    InvalidRelativePath { path: String },

    #[error("URL cannot be used as a path base: {url}")]
    UrlNotPathBase { url: String },

    #[error("can't map URL to a known origin: {raw_url}")]
    UnknownOrigin { raw_url: String },

    #[error("URL matches a known origin host but not a configured base path: {raw_url}")]
    OriginBasePathMismatch { raw_url: String },

    #[error("{context} must use http or https: {url}")]
    UnsupportedUrlScheme { context: String, url: String },
}

impl OriginError {
    pub fn invalid_url(
        context: impl Into<String>,
        url: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        Self::InvalidUrl {
            context: context.into(),
            url: url.into(),
            source: Box::new(source),
        }
    }

    pub fn join_url(
        context: impl Into<String>,
        base_url: impl Into<String>,
        path: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        Self::JoinUrl {
            context: context.into(),
            base_url: base_url.into(),
            path: path.into(),
            source: Box::new(source),
        }
    }
}

pub type Result<T> = std::result::Result<T, OriginError>;
