use std::num::NonZeroUsize;

use crate::error::{Error, Result};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ByteRate(NonZeroUsize);

impl ByteRate {
    const KIB: NonZeroUsize = NonZeroUsize::new(1024).expect("1024 must be non-zero");

    pub fn bytes_per_sec(value: NonZeroUsize) -> Self {
        Self(value)
    }

    pub fn kib_per_sec(value: NonZeroUsize) -> Result<Self> {
        value
            .checked_mul(Self::KIB)
            .map(Self::bytes_per_sec)
            .ok_or(Error::ByteRateOverflow { unit: "KiB/s" })
    }

    pub fn mib_per_sec(value: NonZeroUsize) -> Result<Self> {
        value
            .checked_mul(Self::KIB)
            .and_then(|value| value.checked_mul(Self::KIB))
            .map(Self::bytes_per_sec)
            .ok_or(Error::ByteRateOverflow { unit: "MiB/s" })
    }

    pub fn as_bytes_per_sec(self) -> usize {
        self.0.get()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DownloadRateLimit {
    #[default]
    Unlimited,
    Limited(ByteRate),
}
