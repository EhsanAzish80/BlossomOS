use crate::{AuditError, HelperAudit, HelperAuditEvent, audit_chain_hash};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

const AUDIT_FILE: &str = "audit.jsonl";
const MAX_AUDIT_BYTES: u64 = 1024 * 1024;
const MAX_AUDIT_RECORDS: u64 = 4096;
const DIRECTORY_MODE: u32 = 0o700;
const FILE_MODE: u32 = 0o600;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AuditRecord {
    sequence: u64,
    previous_sha256: String,
    event: HelperAuditEvent,
    record_sha256: String,
}

/// A synced, hash-chained audit log in a pre-created trusted runtime directory.
#[derive(Debug)]
pub struct FileAudit {
    directory: PathBuf,
    owner_uid: u32,
    file: File,
    sequence: u64,
    last_sha256: String,
    bytes: u64,
    poisoned: bool,
}

impl FileAudit {
    pub fn open_root_owned(directory: impl AsRef<Path>) -> Result<Self, AuditError> {
        Self::open_owned_by(directory, 0)
    }

    fn open_owned_by(directory: impl AsRef<Path>, owner_uid: u32) -> Result<Self, AuditError> {
        let directory = directory.as_ref().to_path_buf();
        validate_directory(&directory, owner_uid)?;
        let path = directory.join(AUDIT_FILE);
        let mut options = OpenOptions::new();
        options
            .read(true)
            .append(true)
            .create(true)
            .mode(FILE_MODE)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
        let mut file = options.open(&path).map_err(|_| AuditError::Unavailable)?;
        validate_file(&file, owner_uid)?;
        let (sequence, last_sha256, bytes) = read_and_validate(&mut file)?;
        if bytes == 0 {
            file.sync_all().map_err(|_| AuditError::Unavailable)?;
            sync_directory(&directory)?;
        }
        Ok(Self {
            directory,
            owner_uid,
            file,
            sequence,
            last_sha256,
            bytes,
            poisoned: false,
        })
    }
}

impl HelperAudit for FileAudit {
    fn record(&mut self, event: HelperAuditEvent) -> Result<(), AuditError> {
        if self.poisoned || self.sequence >= MAX_AUDIT_RECORDS {
            return Err(AuditError::Unavailable);
        }
        validate_directory(&self.directory, self.owner_uid)?;
        validate_file(&self.file, self.owner_uid)?;
        let sequence = self.sequence + 1;
        let record_sha256 =
            audit_chain_hash(&self.last_sha256, &event).map_err(|_| AuditError::Unavailable)?;
        let record = AuditRecord {
            sequence,
            previous_sha256: self.last_sha256.clone(),
            event,
            record_sha256: record_sha256.clone(),
        };
        let mut bytes = serde_json::to_vec(&record).map_err(|_| AuditError::Unavailable)?;
        bytes.push(b'\n');
        if self
            .bytes
            .checked_add(bytes.len() as u64)
            .is_none_or(|total| total > MAX_AUDIT_BYTES)
        {
            return Err(AuditError::Unavailable);
        }
        if self
            .file
            .write_all(&bytes)
            .and_then(|_| self.file.sync_all())
            .is_err()
        {
            self.poisoned = true;
            return Err(AuditError::Unavailable);
        }
        self.sequence = sequence;
        self.last_sha256 = record_sha256;
        self.bytes += bytes.len() as u64;
        Ok(())
    }
}

fn read_and_validate(file: &mut File) -> Result<(u64, String, u64), AuditError> {
    let length = file.metadata().map_err(|_| AuditError::Unavailable)?.len();
    if length > MAX_AUDIT_BYTES {
        return Err(AuditError::Unavailable);
    }
    let mut bytes = Vec::with_capacity(length as usize);
    file.take(MAX_AUDIT_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| AuditError::Unavailable)?;
    if bytes.len() as u64 != length || (!bytes.is_empty() && !bytes.ends_with(b"\n")) {
        return Err(AuditError::Unavailable);
    }
    let mut sequence = 0u64;
    let mut previous = "0".repeat(64);
    let content = &bytes[..bytes.len().saturating_sub(1)];
    for line in content
        .split(|byte| *byte == b'\n')
        .filter(|_| !content.is_empty())
    {
        sequence = sequence.checked_add(1).ok_or(AuditError::Unavailable)?;
        if sequence > MAX_AUDIT_RECORDS {
            return Err(AuditError::Unavailable);
        }
        let record: AuditRecord =
            serde_json::from_slice(line).map_err(|_| AuditError::Unavailable)?;
        let expected =
            audit_chain_hash(&previous, &record.event).map_err(|_| AuditError::Unavailable)?;
        if record.sequence != sequence
            || record.previous_sha256 != previous
            || record.record_sha256 != expected
        {
            return Err(AuditError::Unavailable);
        }
        previous = record.record_sha256;
    }
    Ok((sequence, previous, length))
}

fn validate_directory(path: &Path, owner_uid: u32) -> Result<(), AuditError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| AuditError::Unavailable)?;
    if !metadata.file_type().is_dir()
        || metadata.uid() != owner_uid
        || metadata.permissions().mode() & 0o777 != DIRECTORY_MODE
    {
        return Err(AuditError::Unavailable);
    }
    for entry in fs::read_dir(path).map_err(|_| AuditError::Unavailable)? {
        let entry = entry.map_err(|_| AuditError::Unavailable)?;
        if entry.file_name() != AUDIT_FILE {
            return Err(AuditError::Unavailable);
        }
    }
    Ok(())
}

fn validate_file(file: &File, owner_uid: u32) -> Result<(), AuditError> {
    let metadata = file.metadata().map_err(|_| AuditError::Unavailable)?;
    if !metadata.file_type().is_file()
        || metadata.uid() != owner_uid
        || metadata.permissions().mode() & 0o777 != FILE_MODE
    {
        return Err(AuditError::Unavailable);
    }
    Ok(())
}

fn sync_directory(path: &Path) -> Result<(), AuditError> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|_| AuditError::Unavailable)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static NEXT_TEST_DIRECTORY: AtomicU64 = AtomicU64::new(0);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "blossom-audit-{}-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos(),
                NEXT_TEST_DIRECTORY.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(DIRECTORY_MODE)).unwrap();
            Self(path)
        }

        fn owner_uid(&self) -> u32 {
            fs::symlink_metadata(&self.0).unwrap().uid()
        }

        fn audit(&self) -> FileAudit {
            FileAudit::open_owned_by(&self.0, self.owner_uid()).unwrap()
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn event(correlation: &str) -> HelperAuditEvent {
        HelperAuditEvent::JobMarkedSubmitted {
            correlation_id: correlation.into(),
        }
    }

    #[test]
    fn persists_and_recovers_a_valid_chain() {
        let directory = TestDirectory::new();
        let mut audit = directory.audit();
        audit.record(event("request-1")).unwrap();
        audit.record(event("request-2")).unwrap();
        drop(audit);
        let audit = directory.audit();
        assert_eq!(audit.sequence, 2);
        assert_ne!(audit.last_sha256, "0".repeat(64));
    }

    #[test]
    fn rejects_tampering_truncation_unknown_files_and_modes() {
        let directory = TestDirectory::new();
        directory.audit().record(event("request-1")).unwrap();
        let path = directory.0.join(AUDIT_FILE);
        let mut bytes = fs::read(&path).unwrap();
        bytes[20] ^= 1;
        fs::write(&path, bytes).unwrap();
        assert!(FileAudit::open_owned_by(&directory.0, directory.owner_uid()).is_err());

        fs::write(&path, b"{}").unwrap();
        assert!(FileAudit::open_owned_by(&directory.0, directory.owner_uid()).is_err());

        fs::remove_file(&path).unwrap();
        fs::write(directory.0.join("foreign"), b"x").unwrap();
        assert!(FileAudit::open_owned_by(&directory.0, directory.owner_uid()).is_err());
        fs::remove_file(directory.0.join("foreign")).unwrap();

        fs::set_permissions(&directory.0, fs::Permissions::from_mode(0o755)).unwrap();
        assert!(FileAudit::open_owned_by(&directory.0, directory.owner_uid()).is_err());
    }

    #[test]
    fn rejects_symlinked_directory_and_audit_file() {
        let directory = TestDirectory::new();
        let link = directory.0.with_extension("link");
        symlink(&directory.0, &link).unwrap();
        assert!(FileAudit::open_owned_by(&link, directory.owner_uid()).is_err());
        fs::remove_file(&link).unwrap();

        let target = directory.0.join("target");
        fs::write(&target, b"").unwrap();
        fs::set_permissions(&target, fs::Permissions::from_mode(FILE_MODE)).unwrap();
        symlink(&target, directory.0.join(AUDIT_FILE)).unwrap();
        assert!(FileAudit::open_owned_by(&directory.0, directory.owner_uid()).is_err());
    }
}
