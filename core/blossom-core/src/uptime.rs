use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fmt;
use std::fmt::Write as _;
use std::fs::{File, OpenOptions};
use std::io::{self, Read};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

pub const PROC_UPTIME_PATH: &str = "/proc/uptime";
pub const MAX_PROC_UPTIME_BYTES: usize = 128;
const MAX_FRACTIONAL_DIGITS: usize = 9;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SystemUptime {
    pub seconds: u64,
    pub nanoseconds: u32,
    pub source_path: String,
    pub source_sha256: String,
    pub source_bytes: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UptimeError {
    Missing,
    OpenFailed,
    NonRegularFile,
    ReadFailed,
    OversizedInput,
    NulByte,
    InvalidUtf8,
    MalformedInput,
    NumericOverflow,
}

impl fmt::Display for UptimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Missing => "/proc/uptime is missing",
            Self::OpenFailed => "/proc/uptime could not be opened",
            Self::NonRegularFile => "/proc/uptime final target is not a regular file",
            Self::ReadFailed => "/proc/uptime could not be read",
            Self::OversizedInput => "/proc/uptime exceeds the input limit",
            Self::NulByte => "/proc/uptime contains a NUL byte",
            Self::InvalidUtf8 => "/proc/uptime is not valid UTF-8",
            Self::MalformedInput => "/proc/uptime has an invalid format",
            Self::NumericOverflow => "/proc/uptime contains a value outside the supported range",
        })
    }
}

impl std::error::Error for UptimeError {}

pub trait UptimeProvider {
    fn read_uptime(&mut self) -> Result<SystemUptime, UptimeError>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct UnavailableUptimeProvider;

impl UptimeProvider for UnavailableUptimeProvider {
    fn read_uptime(&mut self) -> Result<SystemUptime, UptimeError> {
        Err(UptimeError::OpenFailed)
    }
}

#[derive(Clone, Debug)]
pub struct ProcUptimeReader {
    path: PathBuf,
}

impl Default for ProcUptimeReader {
    fn default() -> Self {
        Self {
            path: PathBuf::from(PROC_UPTIME_PATH),
        }
    }
}

impl ProcUptimeReader {
    #[cfg(test)]
    fn for_test(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

impl UptimeProvider for ProcUptimeReader {
    fn read_uptime(&mut self) -> Result<SystemUptime, UptimeError> {
        let mut file = open_read_only_nonblocking(&self.path).map_err(map_open_error)?;
        let metadata = file.metadata().map_err(|_| UptimeError::ReadFailed)?;
        if !metadata.file_type().is_file() {
            return Err(UptimeError::NonRegularFile);
        }
        let mut bytes = Vec::with_capacity(MAX_PROC_UPTIME_BYTES);
        file.by_ref()
            .take((MAX_PROC_UPTIME_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|_| UptimeError::ReadFailed)?;
        if bytes.len() > MAX_PROC_UPTIME_BYTES {
            return Err(UptimeError::OversizedInput);
        }
        parse_proc_uptime(&bytes)
    }
}

fn open_read_only_nonblocking(path: &Path) -> io::Result<File> {
    OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_CLOEXEC | libc::O_NONBLOCK)
        .open(path)
}

fn map_open_error(error: io::Error) -> UptimeError {
    if error.kind() == io::ErrorKind::NotFound {
        UptimeError::Missing
    } else {
        UptimeError::OpenFailed
    }
}

pub fn parse_proc_uptime(bytes: &[u8]) -> Result<SystemUptime, UptimeError> {
    if bytes.len() > MAX_PROC_UPTIME_BYTES {
        return Err(UptimeError::OversizedInput);
    }
    if bytes.contains(&0) {
        return Err(UptimeError::NulByte);
    }
    let text = std::str::from_utf8(bytes).map_err(|_| UptimeError::InvalidUtf8)?;
    if text
        .chars()
        .any(|character| character.is_control() && !character.is_ascii_whitespace())
    {
        return Err(UptimeError::MalformedInput);
    }
    let fields = text.split_ascii_whitespace().collect::<Vec<_>>();
    if fields.len() != 2 {
        return Err(UptimeError::MalformedInput);
    }
    let (seconds, nanoseconds) = parse_decimal_seconds(fields[0])?;
    let _aggregate_idle = parse_decimal_seconds(fields[1])?;
    Ok(SystemUptime {
        seconds,
        nanoseconds,
        source_path: PROC_UPTIME_PATH.into(),
        source_sha256: digest(bytes),
        source_bytes: bytes.len(),
    })
}

fn parse_decimal_seconds(value: &str) -> Result<(u64, u32), UptimeError> {
    let mut parts = value.split('.');
    let whole = parts.next().ok_or(UptimeError::MalformedInput)?;
    let fraction = parts.next();
    if parts.next().is_some()
        || whole.is_empty()
        || !whole.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(UptimeError::MalformedInput);
    }
    let seconds = whole
        .parse::<u64>()
        .map_err(|_| UptimeError::NumericOverflow)?;
    let nanoseconds = match fraction {
        None => 0,
        Some(value)
            if !value.is_empty()
                && value.len() <= MAX_FRACTIONAL_DIGITS
                && value.bytes().all(|byte| byte.is_ascii_digit()) =>
        {
            let parsed = value
                .parse::<u32>()
                .map_err(|_| UptimeError::NumericOverflow)?;
            parsed * 10_u32.pow((MAX_FRACTIONAL_DIGITS - value.len()) as u32)
        }
        Some(_) => return Err(UptimeError::MalformedInput),
    };
    Ok((seconds, nanoseconds))
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
                std::env::temp_dir().join(format!("blossom-uptime-{}-{id}", std::process::id()));
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

    #[test]
    fn parses_exact_duration_and_provenance_without_exposing_idle() {
        let bytes = b"12345.67 89012.34\n";
        let uptime = parse_proc_uptime(bytes).expect("valid proc uptime fixture");
        assert_eq!(uptime.seconds, 12_345);
        assert_eq!(uptime.nanoseconds, 670_000_000);
        assert_eq!(uptime.source_bytes, bytes.len());
        assert_eq!(uptime.source_sha256.len(), 64);
        let encoded = serde_json::to_string(&uptime).expect("serializable result");
        assert!(!encoded.contains("89012"));
    }

    #[test]
    fn accepts_integer_and_bounded_fractional_values() {
        assert_eq!(parse_decimal_seconds("9").unwrap(), (9, 0));
        assert_eq!(parse_decimal_seconds("9.1").unwrap(), (9, 100_000_000));
        assert_eq!(
            parse_decimal_seconds("9.123456789").unwrap(),
            (9, 123_456_789)
        );
    }

    #[test]
    fn rejects_malformed_and_unbounded_values() {
        for bytes in [
            &b""[..],
            &b"1.0"[..],
            &b"1.0 2.0 extra"[..],
            &b"-1.0 2.0"[..],
            &b"1e3 2.0"[..],
            &b"NaN 2.0"[..],
            &b"1. 2.0"[..],
            &b"1.1234567890 2.0"[..],
            &b"1.0 2.0\0"[..],
        ] {
            assert!(parse_proc_uptime(bytes).is_err(), "accepted {bytes:?}");
        }
        assert_eq!(
            parse_proc_uptime(&[0xff, b' ', b'1']),
            Err(UptimeError::InvalidUtf8)
        );
        assert_eq!(
            parse_proc_uptime(&[b'1'; MAX_PROC_UPTIME_BYTES + 1]),
            Err(UptimeError::OversizedInput)
        );
        assert_eq!(
            parse_proc_uptime(b"18446744073709551616.0 1.0"),
            Err(UptimeError::NumericOverflow)
        );
    }

    #[test]
    fn reads_once_through_a_regular_final_target() {
        let directory = TestDirectory::new();
        let target = directory.path("uptime-target");
        let link = directory.path("uptime-link");
        fs::write(&target, b"42.25 99.50\n").expect("write fixture");
        symlink(&target, &link).expect("create symlink");
        let result = ProcUptimeReader::for_test(link)
            .read_uptime()
            .expect("regular symlink target is accepted");
        assert_eq!((result.seconds, result.nanoseconds), (42, 250_000_000));
        assert_eq!(result.source_path, PROC_UPTIME_PATH);
    }

    #[test]
    fn rejects_missing_special_and_oversized_sources() {
        let directory = TestDirectory::new();
        assert_eq!(
            ProcUptimeReader::for_test(directory.path("missing")).read_uptime(),
            Err(UptimeError::Missing)
        );
        assert_eq!(
            ProcUptimeReader::for_test(&directory.0).read_uptime(),
            Err(UptimeError::NonRegularFile)
        );
        let oversized = directory.path("oversized");
        fs::write(&oversized, vec![b'1'; MAX_PROC_UPTIME_BYTES + 1]).unwrap();
        assert_eq!(
            ProcUptimeReader::for_test(oversized).read_uptime(),
            Err(UptimeError::OversizedInput)
        );
    }
}
