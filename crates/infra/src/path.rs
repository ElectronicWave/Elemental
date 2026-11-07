use std::{
    fmt,
    path::{Path, PathBuf},
};

use crate::error::{Error, Result};

/// A relative path that is safe to append to a stable local storage root and
/// to append to a URL path.
///
/// Remote metadata is untrusted: absolute paths, drive prefixes, parent
/// traversal and URL delimiters must never reach `Path::join` or `Url::join`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SafeRelativePath(String);

impl SafeRelativePath {
    pub fn parse(raw: impl Into<String>) -> Result<Self> {
        let raw = raw.into();
        validate(&raw)?;
        Ok(Self(raw))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub fn as_path(&self) -> &Path {
        Path::new(self.0.as_str())
    }

    pub fn join_to(&self, root: &Path) -> PathBuf {
        root.join(self.as_path())
    }
}

impl fmt::Display for SafeRelativePath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl AsRef<Path> for SafeRelativePath {
    fn as_ref(&self) -> &Path {
        self.as_path()
    }
}

impl AsRef<str> for SafeRelativePath {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

fn validate(raw: &str) -> Result<()> {
    let reject = |reason: &'static str| {
        Err(Error::UnsafeRelativePath {
            path: raw.to_owned(),
            reason,
        })
    };

    if raw.is_empty() {
        return reject("path is empty");
    }
    if raw.starts_with('/') {
        return reject("path is absolute");
    }
    if raw.contains('\\') {
        return reject("path contains a backslash");
    }
    if raw.contains('%') {
        return reject("path contains a percent escape");
    }
    if raw.contains(':') {
        return reject("path contains a drive or scheme separator");
    }
    if raw.contains('?') || raw.contains('#') {
        return reject("path contains a URL delimiter");
    }
    if raw.chars().any(char::is_control) {
        return reject("path contains a control character");
    }

    for segment in raw.split('/') {
        if segment.is_empty() {
            return reject("path contains an empty segment");
        }
        if segment == "." || segment == ".." {
            return reject("path contains a current or parent directory segment");
        }
        if segment.ends_with(' ') || segment.ends_with('.') {
            return reject("path segment ends with a Windows-trimmed character");
        }
        if is_windows_reserved_name(segment) {
            return reject("path contains a Windows reserved device name");
        }
    }

    Ok(())
}

fn is_windows_reserved_name(segment: &str) -> bool {
    let stem = segment
        .split_once('.')
        .map_or(segment, |(stem, _extension)| stem)
        .trim()
        .to_ascii_uppercase();

    matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL" | "CLOCK$")
        || stem
            .strip_prefix("COM")
            .or_else(|| stem.strip_prefix("LPT"))
            .is_some_and(|suffix| suffix.len() == 1 && suffix.as_bytes()[0].is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::SafeRelativePath;

    #[test]
    fn accepts_plain_relative_paths() {
        for raw in ["a.jar", "org/example/a-1.0.jar", "a.b-c_d/e"] {
            assert!(SafeRelativePath::parse(raw).is_ok(), "{raw}");
        }
    }

    #[test]
    fn rejects_escaping_paths() {
        for raw in [
            "",
            "/tmp/a.jar",
            "C:\\a.jar",
            "a\\b.jar",
            "..",
            "../a.jar",
            "a/../../b.jar",
            "a//b.jar",
            "a/./b.jar",
            "//host/path",
            "a?x=1",
            "a#fragment",
            "a:b",
            "a%2e%2e/b",
            "a./b",
            "a /b",
            "CON",
            "nul.txt",
            "COM1.jar",
        ] {
            assert!(SafeRelativePath::parse(raw).is_err(), "{raw}");
        }
    }
}
