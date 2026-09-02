use serde::{Deserialize, Serialize};
#[cfg(target_os = "linux")]
use sha2::{Digest, Sha256};
use std::fmt;

pub const MAX_SELECTED_PATH_BYTES: usize = 4096;
pub const MAX_FILE_CONTENT_BYTES: usize = 64 * 1024;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileIdentity {
    pub device: u64,
    pub inode: u64,
    pub size: u64,
    pub modified_seconds: i64,
    pub modified_nanoseconds: i64,
    pub changed_seconds: i64,
    pub changed_nanoseconds: i64,
}

impl FileIdentity {
    pub fn is_valid(&self) -> bool {
        self.device > 0
            && self.inode > 0
            && self.size <= MAX_FILE_CONTENT_BYTES as u64
            && (0..1_000_000_000).contains(&self.modified_nanoseconds)
            && (0..1_000_000_000).contains(&self.changed_nanoseconds)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileSelection {
    pub absolute_path: String,
    pub identity: FileIdentity,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FileContent {
    pub selection: FileSelection,
    pub content: String,
    pub source_bytes: usize,
    pub source_sha256: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FileReadError {
    UnsupportedPlatform,
    InvalidPath,
    OpenFailed,
    NotRegularFile,
    FileTooLarge,
    IdentityChanged,
    ReadFailed,
    InvalidUtf8,
}

impl fmt::Display for FileReadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::UnsupportedPlatform => "exact-file reads require Linux openat2",
            Self::InvalidPath => "the selected file path is not exact and absolute",
            Self::OpenFailed => "the selected file could not be opened without symlinks",
            Self::NotRegularFile => "the selected resource is not a regular file",
            Self::FileTooLarge => "the selected file exceeds the content bound",
            Self::IdentityChanged => "the selected file identity changed",
            Self::ReadFailed => "the selected file could not be read",
            Self::InvalidUtf8 => "the selected file is not valid UTF-8 text",
        })
    }
}

impl std::error::Error for FileReadError {}

pub trait FileContentProvider {
    fn selection(&self) -> &FileSelection;
    fn read_selected_file(
        &mut self,
        expected: &FileSelection,
    ) -> Result<FileContent, FileReadError>;
}

#[derive(Clone, Debug)]
pub struct UnavailableFileContentProvider {
    selection: FileSelection,
}

impl Default for UnavailableFileContentProvider {
    fn default() -> Self {
        Self::new(FileSelection {
            absolute_path: "/unavailable".into(),
            identity: FileIdentity {
                device: 0,
                inode: 0,
                size: 0,
                modified_seconds: 0,
                modified_nanoseconds: 0,
                changed_seconds: 0,
                changed_nanoseconds: 0,
            },
        })
    }
}

impl UnavailableFileContentProvider {
    pub fn new(selection: FileSelection) -> Self {
        Self { selection }
    }
}

impl FileContentProvider for UnavailableFileContentProvider {
    fn selection(&self) -> &FileSelection {
        &self.selection
    }
    fn read_selected_file(&mut self, _: &FileSelection) -> Result<FileContent, FileReadError> {
        Err(FileReadError::UnsupportedPlatform)
    }
}

pub fn validate_selected_path(path: &str) -> Result<(), FileReadError> {
    if path.is_empty()
        || path.len() > MAX_SELECTED_PATH_BYTES
        || !path.starts_with('/')
        || path == "/"
        || path.ends_with('/')
        || path.contains('\0')
        || path.chars().any(char::is_control)
    {
        return Err(FileReadError::InvalidPath);
    }
    if path
        .split('/')
        .skip(1)
        .any(|component| component.is_empty() || component == "." || component == "..")
    {
        return Err(FileReadError::InvalidPath);
    }
    Ok(())
}

#[cfg(target_os = "linux")]
#[derive(Debug)]
pub struct Openat2FileReader {
    selection: FileSelection,
    file: std::fs::File,
}

#[cfg(not(target_os = "linux"))]
#[derive(Clone, Copy, Debug, Default)]
pub struct Openat2FileReader;

#[cfg(not(target_os = "linux"))]
impl Openat2FileReader {
    pub fn select(_: &str) -> Result<Self, FileReadError> {
        Err(FileReadError::UnsupportedPlatform)
    }
}

#[cfg(not(target_os = "linux"))]
impl FileContentProvider for Openat2FileReader {
    fn selection(&self) -> &FileSelection {
        panic!("Linux-only reader cannot be constructed")
    }
    fn read_selected_file(&mut self, _: &FileSelection) -> Result<FileContent, FileReadError> {
        Err(FileReadError::UnsupportedPlatform)
    }
}

#[cfg(target_os = "linux")]
impl Openat2FileReader {
    pub fn select(path: &str) -> Result<Self, FileReadError> {
        use nix::fcntl::{OFlag, OpenHow, ResolveFlag, open, openat2};
        use nix::sys::stat::Mode;
        use std::fs::File;

        validate_selected_path(path)?;
        let root = open(
            "/",
            OFlag::O_RDONLY | OFlag::O_DIRECTORY | OFlag::O_CLOEXEC,
            Mode::empty(),
        )
        .map_err(|_| FileReadError::OpenFailed)?;
        let relative = path.strip_prefix('/').ok_or(FileReadError::InvalidPath)?;
        let how = OpenHow::new()
            .flags(OFlag::O_RDONLY | OFlag::O_CLOEXEC | OFlag::O_NOCTTY | OFlag::O_NONBLOCK)
            .resolve(
                ResolveFlag::RESOLVE_BENEATH
                    | ResolveFlag::RESOLVE_NO_SYMLINKS
                    | ResolveFlag::RESOLVE_NO_MAGICLINKS,
            );
        let descriptor = openat2(&root, relative, how).map_err(|_| FileReadError::OpenFailed)?;
        let file = File::from(descriptor);
        let identity = identity_from_file(&file)?;
        if identity.size > MAX_FILE_CONTENT_BYTES as u64 {
            return Err(FileReadError::FileTooLarge);
        }
        Ok(Self {
            selection: FileSelection {
                absolute_path: path.into(),
                identity,
            },
            file,
        })
    }
}

#[cfg(target_os = "linux")]
impl FileContentProvider for Openat2FileReader {
    fn selection(&self) -> &FileSelection {
        &self.selection
    }

    fn read_selected_file(
        &mut self,
        expected: &FileSelection,
    ) -> Result<FileContent, FileReadError> {
        use std::io::{Read, Seek, SeekFrom};
        if expected != &self.selection || identity_from_file(&self.file)? != expected.identity {
            return Err(FileReadError::IdentityChanged);
        }
        self.file
            .seek(SeekFrom::Start(0))
            .map_err(|_| FileReadError::ReadFailed)?;
        let mut bytes = Vec::new();
        self.file
            .by_ref()
            .take((MAX_FILE_CONTENT_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|_| FileReadError::ReadFailed)?;
        if bytes.len() > MAX_FILE_CONTENT_BYTES {
            return Err(FileReadError::FileTooLarge);
        }
        if identity_from_file(&self.file)? != expected.identity {
            return Err(FileReadError::IdentityChanged);
        }
        let content = String::from_utf8(bytes.clone()).map_err(|_| FileReadError::InvalidUtf8)?;
        Ok(FileContent {
            selection: expected.clone(),
            content,
            source_bytes: bytes.len(),
            source_sha256: digest(&bytes),
        })
    }
}

#[cfg(target_os = "linux")]
fn identity_from_file(file: &std::fs::File) -> Result<FileIdentity, FileReadError> {
    use std::os::unix::fs::{FileTypeExt, MetadataExt};
    let metadata = file.metadata().map_err(|_| FileReadError::ReadFailed)?;
    let kind = metadata.file_type();
    if !kind.is_file()
        || kind.is_fifo()
        || kind.is_socket()
        || kind.is_block_device()
        || kind.is_char_device()
    {
        return Err(FileReadError::NotRegularFile);
    }
    Ok(FileIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
        size: metadata.size(),
        modified_seconds: metadata.mtime(),
        modified_nanoseconds: metadata.mtime_nsec(),
        changed_seconds: metadata.ctime(),
        changed_nanoseconds: metadata.ctime_nsec(),
    })
}

#[cfg(target_os = "linux")]
fn digest(bytes: &[u8]) -> String {
    let hash = Sha256::digest(bytes);
    hash.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_exact_absolute_paths() {
        assert_eq!(validate_selected_path("/home/user/note.txt"), Ok(()));
        for invalid in [
            "",
            "/",
            "relative",
            "/tmp/../secret",
            "/tmp/./file",
            "/tmp//file",
            "/tmp/file/",
            "/tmp/bad\nname",
        ] {
            assert_eq!(
                validate_selected_path(invalid),
                Err(FileReadError::InvalidPath),
                "{invalid:?}"
            );
        }
    }

    #[cfg(target_os = "linux")]
    mod linux {
        use super::*;
        use std::fs::{self, File};
        use std::io::Write;
        use std::os::unix::fs::symlink;
        use std::path::PathBuf;

        struct Root(PathBuf);
        impl Root {
            fn new(label: &str) -> Self {
                let path = std::env::temp_dir().join(format!(
                    "blossom-file-read-{label}-{}-{}",
                    std::process::id(),
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .expect("clock")
                        .as_nanos()
                ));
                fs::create_dir(&path).expect("fixture root");
                Self(path)
            }
            fn child(&self, name: &str) -> PathBuf {
                self.0.join(name)
            }
        }
        impl Drop for Root {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.0);
            }
        }

        #[test]
        fn reads_exact_utf8_from_retained_descriptor_after_path_replacement() {
            let root = Root::new("replacement");
            let path = root.child("selected.txt");
            fs::write(&path, "approved content").expect("selected file");
            let mut reader =
                Openat2FileReader::select(path.to_str().expect("path")).expect("selection");
            let selection = reader.selection().clone();
            fs::rename(&path, root.child("moved.txt")).expect("replace path");
            fs::write(&path, "attacker content").expect("replacement");
            match reader.read_selected_file(&selection) {
                Ok(result) => assert_eq!(result.content, "approved content"),
                Err(error) => assert_eq!(error, FileReadError::IdentityChanged),
            }
        }

        #[test]
        fn rejects_final_and_intermediate_symlinks() {
            let root = Root::new("symlinks");
            let real = root.child("real");
            fs::create_dir(&real).expect("real directory");
            fs::write(real.join("file.txt"), "secret").expect("real file");
            symlink(real.join("file.txt"), root.child("final.txt")).expect("final symlink");
            symlink(&real, root.child("linked-dir")).expect("directory symlink");
            assert!(matches!(
                Openat2FileReader::select(root.child("final.txt").to_str().expect("path")),
                Err(FileReadError::OpenFailed)
            ));
            assert!(matches!(
                Openat2FileReader::select(
                    root.child("linked-dir/file.txt").to_str().expect("path")
                ),
                Err(FileReadError::OpenFailed)
            ));
        }

        #[test]
        fn rejects_special_oversized_invalid_utf8_and_mutated_files() {
            let root = Root::new("invalid");
            let fifo = root.child("pipe");
            nix::unistd::mkfifo(&fifo, nix::sys::stat::Mode::S_IRUSR).expect("FIFO");
            assert!(matches!(
                Openat2FileReader::select(fifo.to_str().expect("path")),
                Err(FileReadError::NotRegularFile)
            ));

            let large = root.child("large.txt");
            fs::write(&large, vec![b'a'; MAX_FILE_CONTENT_BYTES + 1]).expect("large file");
            assert!(matches!(
                Openat2FileReader::select(large.to_str().expect("path")),
                Err(FileReadError::FileTooLarge)
            ));

            let binary = root.child("binary.txt");
            fs::write(&binary, [0xff, 0xfe]).expect("binary");
            let mut binary_reader = Openat2FileReader::select(binary.to_str().expect("path"))
                .expect("binary selection");
            let binary_selection = binary_reader.selection().clone();
            assert_eq!(
                binary_reader.read_selected_file(&binary_selection),
                Err(FileReadError::InvalidUtf8)
            );

            let mutable = root.child("mutable.txt");
            fs::write(&mutable, "before").expect("mutable");
            let mut reader =
                Openat2FileReader::select(mutable.to_str().expect("path")).expect("selection");
            let selection = reader.selection().clone();
            File::options()
                .write(true)
                .truncate(true)
                .open(&mutable)
                .expect("open mutable")
                .write_all(b"changed value")
                .expect("mutate");
            assert_eq!(
                reader.read_selected_file(&selection),
                Err(FileReadError::IdentityChanged)
            );
        }
    }
}
