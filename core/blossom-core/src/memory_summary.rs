use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt;
use std::fmt::Write as _;
use std::fs::{File, OpenOptions};
use std::io::{self, Read};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

pub const PROC_MEMINFO_PATH: &str = "/proc/meminfo";
pub const MAX_PROC_MEMINFO_BYTES: usize = 64 * 1024;
pub const MAX_PROC_MEMINFO_LINES: usize = 512;
const MAX_KEY_BYTES: usize = 64;
const KIBIBYTE_BYTES: u64 = 1024;
const REQUIRED_KEYS: [&str; 4] = ["MemTotal", "MemAvailable", "SwapTotal", "SwapFree"];

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MemorySummary {
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub swap_total_bytes: u64,
    pub swap_free_bytes: u64,
    pub source_path: String,
    pub source_sha256: String,
    pub source_bytes: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MemorySummaryError {
    Missing,
    OpenFailed,
    NonRegularFile,
    ReadFailed,
    OversizedInput,
    TooManyLines,
    NulByte,
    InvalidUtf8,
    MalformedInput,
    MissingRequiredField,
    DuplicateRequiredField,
    InvalidUnit,
    NumericOverflow,
    InvalidRelationship,
}

impl fmt::Display for MemorySummaryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Missing => "/proc/meminfo is missing",
            Self::OpenFailed => "/proc/meminfo could not be opened",
            Self::NonRegularFile => "/proc/meminfo final target is not a regular file",
            Self::ReadFailed => "/proc/meminfo could not be read",
            Self::OversizedInput => "/proc/meminfo exceeds the input limit",
            Self::TooManyLines => "/proc/meminfo exceeds the line limit",
            Self::NulByte => "/proc/meminfo contains a NUL byte",
            Self::InvalidUtf8 => "/proc/meminfo is not valid UTF-8",
            Self::MalformedInput => "/proc/meminfo has an invalid format",
            Self::MissingRequiredField => "/proc/meminfo is missing a required summary field",
            Self::DuplicateRequiredField => "/proc/meminfo repeats a required summary field",
            Self::InvalidUnit => "/proc/meminfo uses an invalid unit for a required field",
            Self::NumericOverflow => "/proc/meminfo contains a value outside the supported range",
            Self::InvalidRelationship => "/proc/meminfo contains inconsistent summary values",
        })
    }
}

impl std::error::Error for MemorySummaryError {}

pub trait MemorySummaryProvider {
    fn read_memory_summary(&mut self) -> Result<MemorySummary, MemorySummaryError>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct UnavailableMemorySummaryProvider;

impl MemorySummaryProvider for UnavailableMemorySummaryProvider {
    fn read_memory_summary(&mut self) -> Result<MemorySummary, MemorySummaryError> {
        Err(MemorySummaryError::OpenFailed)
    }
}

#[derive(Clone, Debug)]
pub struct ProcMeminfoReader {
    path: PathBuf,
}

impl Default for ProcMeminfoReader {
    fn default() -> Self {
        Self {
            path: PathBuf::from(PROC_MEMINFO_PATH),
        }
    }
}

impl ProcMeminfoReader {
    #[cfg(test)]
    fn for_test(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

impl MemorySummaryProvider for ProcMeminfoReader {
    fn read_memory_summary(&mut self) -> Result<MemorySummary, MemorySummaryError> {
        let mut file = open_read_only_nonblocking(&self.path).map_err(map_open_error)?;
        let metadata = file
            .metadata()
            .map_err(|_| MemorySummaryError::ReadFailed)?;
        if !metadata.file_type().is_file() {
            return Err(MemorySummaryError::NonRegularFile);
        }
        let mut bytes = Vec::with_capacity(MAX_PROC_MEMINFO_BYTES);
        file.by_ref()
            .take((MAX_PROC_MEMINFO_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|_| MemorySummaryError::ReadFailed)?;
        if bytes.len() > MAX_PROC_MEMINFO_BYTES {
            return Err(MemorySummaryError::OversizedInput);
        }
        parse_proc_meminfo(&bytes)
    }
}

fn open_read_only_nonblocking(path: &Path) -> io::Result<File> {
    OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_CLOEXEC | libc::O_NONBLOCK)
        .open(path)
}

fn map_open_error(error: io::Error) -> MemorySummaryError {
    if error.kind() == io::ErrorKind::NotFound {
        MemorySummaryError::Missing
    } else {
        MemorySummaryError::OpenFailed
    }
}

pub fn parse_proc_meminfo(bytes: &[u8]) -> Result<MemorySummary, MemorySummaryError> {
    if bytes.len() > MAX_PROC_MEMINFO_BYTES {
        return Err(MemorySummaryError::OversizedInput);
    }
    if bytes.contains(&0) {
        return Err(MemorySummaryError::NulByte);
    }
    let text = std::str::from_utf8(bytes).map_err(|_| MemorySummaryError::InvalidUtf8)?;
    if text.split('\n').count() > MAX_PROC_MEMINFO_LINES {
        return Err(MemorySummaryError::TooManyLines);
    }
    let mut values = BTreeMap::new();
    for raw_line in text.lines() {
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        if line.is_empty() {
            return Err(MemorySummaryError::MalformedInput);
        }
        let (key, raw_value) = line
            .split_once(':')
            .ok_or(MemorySummaryError::MalformedInput)?;
        if key.is_empty()
            || key.len() > MAX_KEY_BYTES
            || !key
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'(' | b')'))
        {
            return Err(MemorySummaryError::MalformedInput);
        }
        let fields = raw_value.split_ascii_whitespace().collect::<Vec<_>>();
        if fields.is_empty()
            || fields.len() > 2
            || !fields[0].bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err(MemorySummaryError::MalformedInput);
        }
        if REQUIRED_KEYS.contains(&key) {
            if fields.get(1).copied() != Some("kB") {
                return Err(MemorySummaryError::InvalidUnit);
            }
            let kibibytes = fields[0]
                .parse::<u64>()
                .map_err(|_| MemorySummaryError::NumericOverflow)?;
            let value = kibibytes
                .checked_mul(KIBIBYTE_BYTES)
                .ok_or(MemorySummaryError::NumericOverflow)?;
            if values.insert(key, value).is_some() {
                return Err(MemorySummaryError::DuplicateRequiredField);
            }
        }
    }

    let value = |key| {
        values
            .get(key)
            .copied()
            .ok_or(MemorySummaryError::MissingRequiredField)
    };
    let total_bytes = value("MemTotal")?;
    let available_bytes = value("MemAvailable")?;
    let swap_total_bytes = value("SwapTotal")?;
    let swap_free_bytes = value("SwapFree")?;
    if available_bytes > total_bytes || swap_free_bytes > swap_total_bytes {
        return Err(MemorySummaryError::InvalidRelationship);
    }
    Ok(MemorySummary {
        total_bytes,
        available_bytes,
        swap_total_bytes,
        swap_free_bytes,
        source_path: PROC_MEMINFO_PATH.into(),
        source_sha256: digest(bytes),
        source_bytes: bytes.len(),
    })
}

fn digest(bytes: &[u8]) -> String {
    let hash = Sha256::digest(bytes);
    let mut encoded = String::with_capacity(hash.len() * 2);
    for byte in hash {
        write!(&mut encoded, "{byte:02x}").expect("writing to a String cannot fail");
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::symlink;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEST_DIRECTORY: AtomicU64 = AtomicU64::new(1);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let id = NEXT_TEST_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path =
                std::env::temp_dir().join(format!("blossom-meminfo-{}-{id}", std::process::id()));
            fs::create_dir(&path).expect("create test directory");
            Self(path)
        }

        fn path(&self, name: &str) -> PathBuf {
            self.0.join(name)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn fixture() -> &'static [u8] {
        b"MemTotal:       32858820 kB\nMemFree:        21001236 kB\nMemAvailable:   27214312 kB\nBuffers:          581092 kB\nHugePages_Total:       0\nSwapTotal:       4194304 kB\nSwapFree:        3145728 kB\n"
    }

    #[test]
    fn parses_allowlisted_summary_and_exact_provenance() {
        let summary = parse_proc_meminfo(fixture()).expect("valid meminfo fixture");
        assert_eq!(summary.total_bytes, 32_858_820 * 1024);
        assert_eq!(summary.available_bytes, 27_214_312 * 1024);
        assert_eq!(summary.swap_total_bytes, 4_194_304 * 1024);
        assert_eq!(summary.swap_free_bytes, 3_145_728 * 1024);
        assert_eq!(summary.source_bytes, fixture().len());
        assert_eq!(summary.source_sha256.len(), 64);
        let encoded = serde_json::to_string(&summary).expect("serializable result");
        assert!(!encoded.contains("MemFree"));
        assert!(!encoded.contains("HugePages"));
    }

    #[test]
    fn rejects_missing_duplicate_units_relationships_and_overflow() {
        assert_eq!(
            parse_proc_meminfo(b"MemTotal: 1 kB\n"),
            Err(MemorySummaryError::MissingRequiredField)
        );
        let duplicate = [fixture(), b"MemTotal: 1 kB\n"].concat();
        assert_eq!(
            parse_proc_meminfo(&duplicate),
            Err(MemorySummaryError::DuplicateRequiredField)
        );
        assert_eq!(
            parse_proc_meminfo(
                b"MemTotal: 10 MB\nMemAvailable: 9 kB\nSwapTotal: 0 kB\nSwapFree: 0 kB\n"
            ),
            Err(MemorySummaryError::InvalidUnit)
        );
        assert_eq!(
            parse_proc_meminfo(
                b"MemTotal: 10 kB\nMemAvailable: 11 kB\nSwapTotal: 1 kB\nSwapFree: 0 kB\n"
            ),
            Err(MemorySummaryError::InvalidRelationship)
        );
        assert_eq!(
            parse_proc_meminfo(b"MemTotal: 18446744073709551615 kB\nMemAvailable: 1 kB\nSwapTotal: 0 kB\nSwapFree: 0 kB\n"),
            Err(MemorySummaryError::NumericOverflow)
        );
    }

    #[test]
    fn rejects_malformed_nul_utf8_and_bounds() {
        for bytes in [
            &b""[..],
            &b"MemTotal 1 kB\n"[..],
            &b"Bad Key: 1 kB\n"[..],
            &b"MemTotal: -1 kB\n"[..],
            &b"MemTotal: 1 kB extra\n"[..],
            &b"MemTotal: 1 kB\0\n"[..],
        ] {
            assert!(parse_proc_meminfo(bytes).is_err(), "accepted {bytes:?}");
        }
        assert_eq!(
            parse_proc_meminfo(&[0xff]),
            Err(MemorySummaryError::InvalidUtf8)
        );
        assert_eq!(
            parse_proc_meminfo(&[b'1'; MAX_PROC_MEMINFO_BYTES + 1]),
            Err(MemorySummaryError::OversizedInput)
        );
        assert_eq!(
            parse_proc_meminfo("\n".repeat(MAX_PROC_MEMINFO_LINES).as_bytes()),
            Err(MemorySummaryError::TooManyLines)
        );
    }

    #[test]
    fn reads_once_through_regular_symlink_target() {
        let directory = TestDirectory::new();
        let target = directory.path("meminfo-target");
        let link = directory.path("meminfo-link");
        fs::write(&target, fixture()).expect("write fixture");
        symlink(&target, &link).expect("create symlink");
        let summary = ProcMeminfoReader::for_test(link)
            .read_memory_summary()
            .expect("regular symlink target is accepted");
        assert_eq!(summary.source_path, PROC_MEMINFO_PATH);
    }

    #[test]
    fn rejects_missing_special_and_oversized_sources() {
        let directory = TestDirectory::new();
        assert_eq!(
            ProcMeminfoReader::for_test(directory.path("missing")).read_memory_summary(),
            Err(MemorySummaryError::Missing)
        );
        assert_eq!(
            ProcMeminfoReader::for_test(&directory.0).read_memory_summary(),
            Err(MemorySummaryError::NonRegularFile)
        );
        let device = directory.path("device");
        symlink("/dev/null", &device).expect("create character-device symlink");
        assert_eq!(
            ProcMeminfoReader::for_test(device).read_memory_summary(),
            Err(MemorySummaryError::NonRegularFile)
        );
        let oversized = directory.path("oversized");
        fs::write(&oversized, vec![b'1'; MAX_PROC_MEMINFO_BYTES + 1]).unwrap();
        assert_eq!(
            ProcMeminfoReader::for_test(oversized).read_memory_summary(),
            Err(MemorySummaryError::OversizedInput)
        );
    }
}
