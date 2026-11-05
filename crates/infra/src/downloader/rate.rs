use std::num::NonZeroUsize;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ByteRate(NonZeroUsize);

impl ByteRate {
    const KIB: NonZeroUsize = NonZeroUsize::new(1024).expect("1024 must be non-zero");

    pub fn bytes_per_sec(value: NonZeroUsize) -> Self {
        Self(value)
    }

    pub fn kib_per_sec(value: NonZeroUsize) -> Self {
        Self::bytes_per_sec(
            value
                .checked_mul(Self::KIB)
                .expect("KiB/s conversion overflowed when building ByteRate"),
        )
    }

    pub fn mib_per_sec(value: NonZeroUsize) -> Self {
        Self::kib_per_sec(
            value
                .checked_mul(Self::KIB)
                .expect("MiB/s conversion overflowed when building ByteRate"),
        )
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
