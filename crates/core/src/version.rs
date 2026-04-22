use std::{convert::Infallible, fmt::Display, marker::PhantomData, str::FromStr};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version<Tag> {
    raw: String,
    tag: PhantomData<Tag>,
}

impl<Tag> Version<Tag> {
    pub fn new(raw: String) -> Self {
        Self {
            raw,
            tag: PhantomData,
        }
    }

    pub fn as_str(&self) -> &str {
        self.raw.as_str()
    }

    pub fn into_inner(self) -> String {
        self.raw
    }
}

impl<Tag> AsRef<str> for Version<Tag> {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl<Tag> Display for Version<Tag> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl<Tag> From<String> for Version<Tag> {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl<Tag> From<&str> for Version<Tag> {
    fn from(value: &str) -> Self {
        Self::new(value.to_owned())
    }
}

impl<Tag> FromStr for Version<Tag> {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::from(s))
    }
}
