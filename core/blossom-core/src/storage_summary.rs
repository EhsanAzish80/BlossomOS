use nix::sys::statvfs::statvfs;
use serde::Serialize;
use std::fmt;

pub const ROOT_FILESYSTEM_PATH: &str = "/";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageSummarySource {
    RootStatvfs,
}

impl StorageSummarySource {
    pub fn as_path(self) -> &'static str {
        match self {
            Self::RootStatvfs => ROOT_FILESYSTEM_PATH,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct StorageSummary {
    pub source: StorageSummarySource,
    pub resource_path: String,
    pub total_bytes: u64,
    pub available_bytes: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageSummaryError {
    StatFailed,
    ZeroFragmentSize,
    NumericOverflow,
    InvalidRelationship,
}

impl fmt::Display for StorageSummaryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::StatFailed => "root filesystem statistics could not be read",
            Self::ZeroFragmentSize => "root filesystem reported a zero fragment size",
            Self::NumericOverflow => "root filesystem statistics exceed the supported range",
            Self::InvalidRelationship => "root filesystem statistics are inconsistent",
        })
    }
}

impl std::error::Error for StorageSummaryError {}

pub trait StorageSummaryProvider {
    fn read_storage_summary(&mut self) -> Result<StorageSummary, StorageSummaryError>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct UnavailableStorageSummaryProvider;

impl StorageSummaryProvider for UnavailableStorageSummaryProvider {
    fn read_storage_summary(&mut self) -> Result<StorageSummary, StorageSummaryError> {
        Err(StorageSummaryError::StatFailed)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct RootStorageReader;

impl StorageSummaryProvider for RootStorageReader {
    fn read_storage_summary(&mut self) -> Result<StorageSummary, StorageSummaryError> {
        let statistics =
            statvfs(ROOT_FILESYSTEM_PATH).map_err(|_| StorageSummaryError::StatFailed)?;
        build_storage_summary(
            u64::from(statistics.blocks()),
            u64::from(statistics.blocks_available()),
            statistics.fragment_size(),
        )
    }
}

fn build_storage_summary(
    blocks: u64,
    blocks_available: u64,
    fragment_size: u64,
) -> Result<StorageSummary, StorageSummaryError> {
    if fragment_size == 0 {
        return Err(StorageSummaryError::ZeroFragmentSize);
    }
    if blocks == 0 || blocks_available > blocks {
        return Err(StorageSummaryError::InvalidRelationship);
    }
    let total_bytes = blocks
        .checked_mul(fragment_size)
        .ok_or(StorageSummaryError::NumericOverflow)?;
    let available_bytes = blocks_available
        .checked_mul(fragment_size)
        .ok_or(StorageSummaryError::NumericOverflow)?;
    Ok(StorageSummary {
        source: StorageSummarySource::RootStatvfs,
        resource_path: ROOT_FILESYSTEM_PATH.into(),
        total_bytes,
        available_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_fragment_counts_with_checked_arithmetic() {
        let summary = build_storage_summary(1_000, 250, 4096).expect("valid statistics");
        assert_eq!(summary.total_bytes, 4_096_000);
        assert_eq!(summary.available_bytes, 1_024_000);
        assert_eq!(summary.resource_path, ROOT_FILESYSTEM_PATH);
        assert_eq!(summary.source, StorageSummarySource::RootStatvfs);
    }

    #[test]
    fn rejects_zero_size_inconsistent_counts_and_overflow() {
        assert_eq!(
            build_storage_summary(1, 1, 0),
            Err(StorageSummaryError::ZeroFragmentSize)
        );
        assert_eq!(
            build_storage_summary(1, 2, 4096),
            Err(StorageSummaryError::InvalidRelationship)
        );
        assert_eq!(
            build_storage_summary(0, 0, 4096),
            Err(StorageSummaryError::InvalidRelationship)
        );
        assert_eq!(
            build_storage_summary(u64::MAX, 1, 2),
            Err(StorageSummaryError::NumericOverflow)
        );
    }

    #[test]
    fn reads_the_real_fixed_root_scope() {
        let summary = RootStorageReader
            .read_storage_summary()
            .expect("root filesystem should be observable");
        assert_eq!(summary.resource_path, ROOT_FILESYSTEM_PATH);
        assert!(summary.total_bytes > 0);
        assert!(summary.available_bytes <= summary.total_bytes);
    }
}
