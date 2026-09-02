use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt;
use std::fmt::Write as _;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

pub const MAX_OS_RELEASE_BYTES: usize = 64 * 1024;
pub const MAX_OS_RELEASE_LINES: usize = 256;
pub const MAX_OS_RELEASE_KEYS: usize = 128;
pub const MAX_OS_RELEASE_KEY_BYTES: usize = 64;
pub const MAX_OS_RELEASE_VALUE_BYTES: usize = 4096;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OsReleaseSource {
    EtcOsRelease,
    UsrLibOsRelease,
}

impl OsReleaseSource {
    pub fn as_path(self) -> &'static str {
        match self {
            Self::EtcOsRelease => "/etc/os-release",
            Self::UsrLibOsRelease => "/usr/lib/os-release",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct OsIdentity {
    pub source: OsReleaseSource,
    pub source_path: String,
    pub source_sha256: String,
    pub source_bytes: usize,
    pub id: Option<String>,
    pub name: Option<String>,
    pub pretty_name: Option<String>,
    pub version_id: Option<String>,
    pub version_codename: Option<String>,
    pub build_id: Option<String>,
    pub variant_id: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OsIdentityError {
    Missing,
    OpenFailed,
    NonRegularFile,
    ReadFailed,
    OversizedInput,
    TooManyLines,
    TooManyKeys,
    KeyTooLong,
    ValueTooLong,
    NulByte,
    InvalidUtf8,
    MalformedAssignment,
    InvalidKey,
    InvalidValue,
}

impl fmt::Display for OsIdentityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Missing => "os-release file is missing",
            Self::OpenFailed => "os-release file could not be opened",
            Self::NonRegularFile => "os-release final target is not a regular file",
            Self::ReadFailed => "os-release file could not be read",
            Self::OversizedInput => "os-release file exceeds the input limit",
            Self::TooManyLines => "os-release file exceeds the line limit",
            Self::TooManyKeys => "os-release file exceeds the key limit",
            Self::KeyTooLong => "os-release key exceeds the limit",
            Self::ValueTooLong => "os-release value exceeds the limit",
            Self::NulByte => "os-release contains a NUL byte",
            Self::InvalidUtf8 => "os-release is not valid UTF-8",
            Self::MalformedAssignment => "os-release contains a malformed assignment",
            Self::InvalidKey => "os-release contains an invalid key",
            Self::InvalidValue => "os-release contains an invalid value",
        })
    }
}

impl std::error::Error for OsIdentityError {}

pub trait OsIdentityProvider {
    fn read_os_identity(&mut self) -> Result<OsIdentity, OsIdentityError>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct UnavailableOsIdentityProvider;

impl OsIdentityProvider for UnavailableOsIdentityProvider {
    fn read_os_identity(&mut self) -> Result<OsIdentity, OsIdentityError> {
        Err(OsIdentityError::OpenFailed)
    }
}

#[derive(Clone, Debug)]
pub struct OsReleaseReader {
    etc_path: PathBuf,
    usr_lib_path: PathBuf,
}

impl Default for OsReleaseReader {
    fn default() -> Self {
        Self::new("/etc/os-release", "/usr/lib/os-release")
    }
}

impl OsReleaseReader {
    fn new(etc_path: impl Into<PathBuf>, usr_lib_path: impl Into<PathBuf>) -> Self {
        Self {
            etc_path: etc_path.into(),
            usr_lib_path: usr_lib_path.into(),
        }
    }

    fn open_selected(&self) -> Result<(File, OsReleaseSource), OsIdentityError> {
        match open_read_only_nonblocking(&self.etc_path) {
            Ok(file) => Ok((file, OsReleaseSource::EtcOsRelease)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                match fs::symlink_metadata(&self.etc_path) {
                    Ok(_) => Err(OsIdentityError::OpenFailed),
                    Err(metadata_error) if metadata_error.kind() == io::ErrorKind::NotFound => {
                        open_read_only_nonblocking(&self.usr_lib_path)
                            .map(|file| (file, OsReleaseSource::UsrLibOsRelease))
                            .map_err(map_open_error)
                    }
                    Err(_) => Err(OsIdentityError::OpenFailed),
                }
            }
            Err(error) => Err(map_open_error(error)),
        }
    }
}

impl OsIdentityProvider for OsReleaseReader {
    fn read_os_identity(&mut self) -> Result<OsIdentity, OsIdentityError> {
        let (mut file, source) = self.open_selected()?;
        let metadata = file.metadata().map_err(|_| OsIdentityError::ReadFailed)?;
        if !metadata.file_type().is_file() {
            return Err(OsIdentityError::NonRegularFile);
        }
        let mut bytes =
            Vec::with_capacity(metadata.len().min(MAX_OS_RELEASE_BYTES as u64) as usize);
        file.by_ref()
            .take((MAX_OS_RELEASE_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|_| OsIdentityError::ReadFailed)?;
        if bytes.len() > MAX_OS_RELEASE_BYTES {
            return Err(OsIdentityError::OversizedInput);
        }
        parse_os_release(source, &bytes)
    }
}

fn open_read_only_nonblocking(path: &Path) -> io::Result<File> {
    OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_CLOEXEC | libc::O_NONBLOCK)
        .open(path)
}

fn map_open_error(error: io::Error) -> OsIdentityError {
    if error.kind() == io::ErrorKind::NotFound {
        OsIdentityError::Missing
    } else {
        OsIdentityError::OpenFailed
    }
}

pub fn parse_os_release(
    source: OsReleaseSource,
    bytes: &[u8],
) -> Result<OsIdentity, OsIdentityError> {
    if bytes.len() > MAX_OS_RELEASE_BYTES {
        return Err(OsIdentityError::OversizedInput);
    }
    if bytes.contains(&0) {
        return Err(OsIdentityError::NulByte);
    }
    let text = std::str::from_utf8(bytes).map_err(|_| OsIdentityError::InvalidUtf8)?;
    let line_count = text.split('\n').count();
    if line_count > MAX_OS_RELEASE_LINES {
        return Err(OsIdentityError::TooManyLines);
    }

    let mut fields = BTreeMap::new();
    let mut key_count = 0_usize;
    for raw_line in text.split('\n') {
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, raw_value) = line
            .split_once('=')
            .ok_or(OsIdentityError::MalformedAssignment)?;
        key_count += 1;
        if key_count > MAX_OS_RELEASE_KEYS {
            return Err(OsIdentityError::TooManyKeys);
        }
        if key.len() > MAX_OS_RELEASE_KEY_BYTES {
            return Err(OsIdentityError::KeyTooLong);
        }
        if !valid_key(key) {
            return Err(OsIdentityError::InvalidKey);
        }
        let value = parse_value(raw_value)?;
        if value.len() > MAX_OS_RELEASE_VALUE_BYTES {
            return Err(OsIdentityError::ValueTooLong);
        }
        if allowed_key(key) {
            fields.insert(key, value);
        }
    }

    Ok(OsIdentity {
        source,
        source_path: source.as_path().into(),
        source_sha256: digest(bytes),
        source_bytes: bytes.len(),
        id: fields.remove("ID"),
        name: fields.remove("NAME"),
        pretty_name: fields.remove("PRETTY_NAME"),
        version_id: fields.remove("VERSION_ID"),
        version_codename: fields.remove("VERSION_CODENAME"),
        build_id: fields.remove("BUILD_ID"),
        variant_id: fields.remove("VARIANT_ID"),
    })
}

fn valid_key(key: &str) -> bool {
    let mut bytes = key.bytes();
    matches!(bytes.next(), Some(first) if first.is_ascii_uppercase() || first == b'_')
        && bytes.all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
}

fn allowed_key(key: &str) -> bool {
    matches!(
        key,
        "ID" | "NAME"
            | "PRETTY_NAME"
            | "VERSION_ID"
            | "VERSION_CODENAME"
            | "BUILD_ID"
            | "VARIANT_ID"
    )
}

fn parse_value(raw: &str) -> Result<String, OsIdentityError> {
    if raw.len() > MAX_OS_RELEASE_VALUE_BYTES.saturating_mul(2) {
        return Err(OsIdentityError::ValueTooLong);
    }
    match raw.as_bytes().first().copied() {
        Some(b'\'') | Some(b'"') => parse_quoted_value(raw),
        _ => parse_unquoted_value(raw),
    }
}

fn parse_quoted_value(raw: &str) -> Result<String, OsIdentityError> {
    let quote = raw.as_bytes()[0];
    if raw.len() < 2 || raw.as_bytes().last().copied() != Some(quote) {
        return Err(OsIdentityError::InvalidValue);
    }
    let content = &raw[1..raw.len() - 1];
    decode_escapes(content, Some(quote))
}

fn parse_unquoted_value(raw: &str) -> Result<String, OsIdentityError> {
    if raw
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        Ok(raw.into())
    } else {
        Err(OsIdentityError::InvalidValue)
    }
}

fn decode_escapes(content: &str, delimiter: Option<u8>) -> Result<String, OsIdentityError> {
    let mut output = String::with_capacity(content.len());
    let mut bytes = content.as_bytes().iter().copied();
    while let Some(byte) = bytes.next() {
        if byte == b'\\' {
            let escaped = bytes.next().ok_or(OsIdentityError::InvalidValue)?;
            if !matches!(escaped, b'$' | b'\'' | b'"' | b'\\' | b'`') {
                return Err(OsIdentityError::InvalidValue);
            }
            output.push(escaped as char);
            continue;
        }
        if matches!(byte, b'$' | b'\'' | b'"' | b'`') && Some(byte) != delimiter {
            return Err(OsIdentityError::InvalidValue);
        }
        if byte == delimiter.unwrap_or_default() {
            return Err(OsIdentityError::InvalidValue);
        }
        if byte.is_ascii_control() {
            return Err(OsIdentityError::InvalidValue);
        }
        if byte.is_ascii() {
            output.push(byte as char);
        } else {
            return decode_escaped_unicode(content);
        }
    }
    Ok(output)
}

fn decode_escaped_unicode(content: &str) -> Result<String, OsIdentityError> {
    let mut output = String::with_capacity(content.len());
    let mut escaped = false;
    for character in content.chars() {
        if escaped {
            if !matches!(character, '$' | '\'' | '"' | '\\' | '`') {
                return Err(OsIdentityError::InvalidValue);
            }
            output.push(character);
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character.is_control() || matches!(character, '$' | '\'' | '"' | '`') {
            return Err(OsIdentityError::InvalidValue);
        } else {
            output.push(character);
        }
    }
    if escaped {
        return Err(OsIdentityError::InvalidValue);
    }
    Ok(output)
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
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEMP: AtomicU64 = AtomicU64::new(1);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let id = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir()
                .join(format!("blossom-os-identity-{}-{id}", std::process::id()));
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
    fn parses_arch_fixture_and_ignores_unknown_fields() {
        let bytes = b"NAME=\"Arch Linux\"\nPRETTY_NAME=\"Arch Linux\"\nID=arch\nBUILD_ID=rolling\nANSI_COLOR=\"38;2;23;147;209\"\nHOME_URL=\"https://archlinux.org/\"\n";
        let identity = parse_os_release(OsReleaseSource::EtcOsRelease, bytes)
            .expect("Arch fixture should parse");
        assert_eq!(identity.id.as_deref(), Some("arch"));
        assert_eq!(identity.name.as_deref(), Some("Arch Linux"));
        assert_eq!(identity.pretty_name.as_deref(), Some("Arch Linux"));
        assert_eq!(identity.build_id.as_deref(), Some("rolling"));
        assert_eq!(identity.source_path, "/etc/os-release");
        assert_eq!(identity.source_bytes, bytes.len());
        assert_eq!(identity.source_sha256.len(), 64);
    }

    #[test]
    fn parses_quotes_escapes_unicode_and_later_duplicate_values() {
        let bytes = "NAME='First Name'\nNAME=\"Blossom \\\"Linux\\\"\"\nPRETTY_NAME=\"Blossom \\$ Edition\"\nVERSION_CODENAME=\"çiçek\"\n".as_bytes();
        let identity = parse_os_release(OsReleaseSource::EtcOsRelease, bytes)
            .expect("quoted fixture should parse");
        assert_eq!(identity.name.as_deref(), Some("Blossom \"Linux\""));
        assert_eq!(identity.pretty_name.as_deref(), Some("Blossom $ Edition"));
        assert_eq!(identity.version_codename.as_deref(), Some("çiçek"));
    }

    #[test]
    fn rejects_malformed_nul_utf8_and_unescaped_shell_syntax() {
        assert_eq!(
            parse_os_release(OsReleaseSource::EtcOsRelease, b"NAME\n"),
            Err(OsIdentityError::MalformedAssignment)
        );
        assert_eq!(
            parse_os_release(OsReleaseSource::EtcOsRelease, b"NAME=bad\0value\n"),
            Err(OsIdentityError::NulByte)
        );
        assert_eq!(
            parse_os_release(OsReleaseSource::EtcOsRelease, b"NAME=\xff\n"),
            Err(OsIdentityError::InvalidUtf8)
        );
        assert_eq!(
            parse_os_release(OsReleaseSource::EtcOsRelease, b"NAME=$HOME\n"),
            Err(OsIdentityError::InvalidValue)
        );
        assert_eq!(
            parse_os_release(OsReleaseSource::EtcOsRelease, b"NAME=\"one\"\"two\"\n"),
            Err(OsIdentityError::InvalidValue)
        );
    }

    #[test]
    fn enforces_input_line_key_and_value_bounds() {
        assert_eq!(
            parse_os_release(
                OsReleaseSource::EtcOsRelease,
                &vec![b'a'; MAX_OS_RELEASE_BYTES + 1]
            ),
            Err(OsIdentityError::OversizedInput)
        );
        let too_many_lines = "\n".repeat(MAX_OS_RELEASE_LINES);
        assert_eq!(
            parse_os_release(OsReleaseSource::EtcOsRelease, too_many_lines.as_bytes()),
            Err(OsIdentityError::TooManyLines)
        );
        let too_many_keys = (0..=MAX_OS_RELEASE_KEYS)
            .map(|index| format!("KEY_{index}=value\n"))
            .collect::<String>();
        assert_eq!(
            parse_os_release(OsReleaseSource::EtcOsRelease, too_many_keys.as_bytes()),
            Err(OsIdentityError::TooManyKeys)
        );
        let long_key = format!("{}=value\n", "A".repeat(MAX_OS_RELEASE_KEY_BYTES + 1));
        assert_eq!(
            parse_os_release(OsReleaseSource::EtcOsRelease, long_key.as_bytes()),
            Err(OsIdentityError::KeyTooLong)
        );
        let long_value = format!("NAME=\"{}\"\n", "a".repeat(MAX_OS_RELEASE_VALUE_BYTES + 1));
        assert_eq!(
            parse_os_release(OsReleaseSource::EtcOsRelease, long_value.as_bytes()),
            Err(OsIdentityError::ValueTooLong)
        );
    }

    #[test]
    fn etc_takes_precedence_and_sources_are_never_merged() {
        let directory = TestDirectory::new();
        let etc = directory.path("etc-os-release");
        let usr = directory.path("usr-os-release");
        fs::write(&etc, "ID=etc\n").expect("write etc fixture");
        fs::write(&usr, "NAME=Usr\n").expect("write usr fixture");
        let identity = OsReleaseReader::new(&etc, &usr)
            .read_os_identity()
            .expect("etc fixture should win");
        assert_eq!(identity.source, OsReleaseSource::EtcOsRelease);
        assert_eq!(identity.id.as_deref(), Some("etc"));
        assert_eq!(identity.name, None);
    }

    #[test]
    fn malformed_etc_is_rejected_instead_of_using_valid_fallback() {
        let directory = TestDirectory::new();
        let etc = directory.path("etc-os-release");
        let usr = directory.path("usr-os-release");
        fs::write(&etc, "not-an-assignment\n").expect("write malformed primary fixture");
        fs::write(&usr, "ID=fallback\n").expect("write valid fallback fixture");
        assert_eq!(
            OsReleaseReader::new(&etc, &usr).read_os_identity(),
            Err(OsIdentityError::MalformedAssignment)
        );
    }

    #[test]
    fn falls_back_only_when_etc_is_absent() {
        let directory = TestDirectory::new();
        let etc = directory.path("missing-etc");
        let usr = directory.path("usr-os-release");
        fs::write(&usr, "ID=arch\n").expect("write usr fixture");
        let identity = OsReleaseReader::new(&etc, &usr)
            .read_os_identity()
            .expect("usr fixture should be fallback");
        assert_eq!(identity.source, OsReleaseSource::UsrLibOsRelease);
        assert_eq!(identity.id.as_deref(), Some("arch"));
    }

    #[cfg(unix)]
    #[test]
    fn accepts_symlink_to_regular_file_and_rejects_dangling_symlink() {
        use std::os::unix::fs::symlink;

        let directory = TestDirectory::new();
        let target = directory.path("target");
        let etc = directory.path("etc-os-release");
        let usr = directory.path("usr-os-release");
        fs::write(&target, "ID=arch\n").expect("write target");
        symlink("target", &etc).expect("create relative symlink");
        let identity = OsReleaseReader::new(&etc, &usr)
            .read_os_identity()
            .expect("regular symlink target should parse");
        assert_eq!(identity.id.as_deref(), Some("arch"));

        fs::remove_file(&etc).expect("remove symlink");
        symlink("absent", &etc).expect("create dangling symlink");
        fs::write(&usr, "ID=fallback\n").expect("write fallback");
        assert_eq!(
            OsReleaseReader::new(&etc, &usr).read_os_identity(),
            Err(OsIdentityError::OpenFailed)
        );
    }

    #[test]
    fn rejects_non_regular_final_target() {
        let directory = TestDirectory::new();
        let etc = directory.path("directory-target");
        let usr = directory.path("usr-os-release");
        fs::create_dir(&etc).expect("create directory special target");
        assert_eq!(
            OsReleaseReader::new(&etc, &usr).read_os_identity(),
            Err(OsIdentityError::NonRegularFile)
        );
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlink_to_character_device_without_blocking() {
        use std::os::unix::fs::symlink;

        let directory = TestDirectory::new();
        let etc = directory.path("device-target");
        let usr = directory.path("usr-os-release");
        symlink("/dev/null", &etc).expect("create character-device symlink");
        assert_eq!(
            OsReleaseReader::new(&etc, &usr).read_os_identity(),
            Err(OsIdentityError::NonRegularFile)
        );
    }

    #[test]
    fn bounds_bytes_while_reading_from_the_open_descriptor() {
        let directory = TestDirectory::new();
        let etc = directory.path("etc-os-release");
        let usr = directory.path("usr-os-release");
        fs::write(&etc, vec![b'a'; MAX_OS_RELEASE_BYTES + 1]).expect("write oversized fixture");
        assert_eq!(
            OsReleaseReader::new(&etc, &usr).read_os_identity(),
            Err(OsIdentityError::OversizedInput)
        );
    }

    #[test]
    fn reports_both_sources_missing() {
        let directory = TestDirectory::new();
        assert_eq!(
            OsReleaseReader::new(directory.path("etc"), directory.path("usr")).read_os_identity(),
            Err(OsIdentityError::Missing)
        );
    }
}
