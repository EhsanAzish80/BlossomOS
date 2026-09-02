use crate::{ClaimOutcome, IdempotencyJournal, JournalError, JournalKey, JournalState};
use blossom_core::privileged::BluetoothRestartResult;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

const MAX_ENTRIES: usize = 256;
const MAX_JOURNAL_BYTES: u64 = 1024 * 1024;
const MAX_ENTRY_BYTES: u64 = 16 * 1024;
const DIRECTORY_MODE: u32 = 0o700;
const FILE_MODE: u32 = 0o600;

/// A boot-scoped journal rooted in a pre-created, root-owned runtime directory.
///
/// The systemd unit is responsible for creating and removing the directory.
/// This type never follows a journal-file symlink and never creates the parent.
#[derive(Debug)]
pub struct FileJournal {
    directory: PathBuf,
    owner_uid: u32,
}

impl FileJournal {
    pub fn open_root_owned(directory: impl AsRef<Path>) -> Result<Self, JournalError> {
        Self::open_owned_by(directory, 0)
    }

    fn open_owned_by(directory: impl AsRef<Path>, owner_uid: u32) -> Result<Self, JournalError> {
        let directory = directory.as_ref().to_path_buf();
        validate_directory(&directory, owner_uid)?;
        let journal = Self {
            directory,
            owner_uid,
        };
        journal.usage()?;
        Ok(journal)
    }

    fn path_for(&self, key: &JournalKey) -> Result<PathBuf, JournalError> {
        if key.idempotency_key.len() != 32
            || !key
                .idempotency_key
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        {
            return Err(JournalError::InvalidTransition);
        }
        Ok(self
            .directory
            .join(format!("{}-{}.json", key.caller_uid, key.idempotency_key)))
    }

    fn load(&self, path: &Path) -> Result<Option<JournalState>, JournalError> {
        let mut options = OpenOptions::new();
        options
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
        let file = match options.open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(JournalError::Unavailable),
        };
        validate_file(&file, self.owner_uid)?;
        let length = file
            .metadata()
            .map_err(|_| JournalError::Unavailable)?
            .len();
        if length == 0 || length > MAX_ENTRY_BYTES {
            return Err(JournalError::Unavailable);
        }
        let mut bytes = Vec::with_capacity(length as usize);
        file.take(MAX_ENTRY_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| JournalError::Unavailable)?;
        if bytes.len() as u64 != length {
            return Err(JournalError::Unavailable);
        }
        serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|_| JournalError::Unavailable)
    }

    fn create_claim(&self, path: &Path, state: &JournalState) -> Result<(), JournalError> {
        let bytes = encode(state)?;
        let (entries, used_bytes) = self.usage()?;
        if entries >= MAX_ENTRIES
            || used_bytes
                .checked_add(bytes.len() as u64)
                .is_none_or(|total| total > MAX_JOURNAL_BYTES)
        {
            return Err(JournalError::Unavailable);
        }
        let mut options = OpenOptions::new();
        options
            .write(true)
            .create_new(true)
            .mode(FILE_MODE)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
        let mut file = options.open(path).map_err(|_| JournalError::Unavailable)?;
        write_and_sync(&mut file, &bytes)?;
        sync_directory(&self.directory)
    }

    fn replace(&self, path: &Path, state: &JournalState) -> Result<(), JournalError> {
        let bytes = encode(state)?;
        let (entries, used_bytes) = self.usage()?;
        let old_bytes = fs::symlink_metadata(path)
            .map_err(|_| JournalError::Unavailable)?
            .len();
        if entries > MAX_ENTRIES
            || used_bytes
                .checked_sub(old_bytes)
                .and_then(|remaining| remaining.checked_add(bytes.len() as u64))
                .is_none_or(|total| total > MAX_JOURNAL_BYTES)
        {
            return Err(JournalError::Unavailable);
        }
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or(JournalError::InvalidTransition)?;
        let temporary = self.directory.join(format!(".{file_name}.tmp"));
        let mut options = OpenOptions::new();
        options
            .write(true)
            .create_new(true)
            .mode(FILE_MODE)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
        let mut file = options
            .open(&temporary)
            .map_err(|_| JournalError::Unavailable)?;
        if let Err(error) = write_and_sync(&mut file, &bytes) {
            let _ = fs::remove_file(&temporary);
            return Err(error);
        }
        drop(file);
        fs::rename(&temporary, path).map_err(|_| JournalError::Unavailable)?;
        sync_directory(&self.directory)
    }

    fn usage(&self) -> Result<(usize, u64), JournalError> {
        validate_directory(&self.directory, self.owner_uid)?;
        let mut entries = 0usize;
        let mut bytes = 0u64;
        for item in fs::read_dir(&self.directory).map_err(|_| JournalError::Unavailable)? {
            let item = item.map_err(|_| JournalError::Unavailable)?;
            let name = item.file_name();
            let name = name.to_str().ok_or(JournalError::Unavailable)?;
            if !valid_entry_name(name) {
                return Err(JournalError::Unavailable);
            }
            let file_type = item.file_type().map_err(|_| JournalError::Unavailable)?;
            let metadata =
                fs::symlink_metadata(item.path()).map_err(|_| JournalError::Unavailable)?;
            if !file_type.is_file()
                || metadata.uid() != self.owner_uid
                || metadata.permissions().mode() & 0o777 != FILE_MODE
                || metadata.len() > MAX_ENTRY_BYTES
            {
                return Err(JournalError::Unavailable);
            }
            entries = entries.checked_add(1).ok_or(JournalError::Unavailable)?;
            bytes = bytes
                .checked_add(metadata.len())
                .ok_or(JournalError::Unavailable)?;
        }
        if entries > MAX_ENTRIES || bytes > MAX_JOURNAL_BYTES {
            return Err(JournalError::Unavailable);
        }
        Ok((entries, bytes))
    }
}

impl IdempotencyJournal for FileJournal {
    fn claim(&mut self, key: &JournalKey, digest: &str) -> Result<ClaimOutcome, JournalError> {
        let path = self.path_for(key)?;
        if let Some(state) = self.load(&path)? {
            return if state.digest() == digest {
                Ok(ClaimOutcome::Existing(state))
            } else {
                Ok(ClaimOutcome::DigestMismatch)
            };
        }
        let state = JournalState::Claimed {
            request_sha256: digest.into(),
        };
        self.create_claim(&path, &state)?;
        Ok(ClaimOutcome::New)
    }

    fn mark_submitted(&mut self, key: &JournalKey, digest: &str) -> Result<(), JournalError> {
        let path = self.path_for(key)?;
        match self.load(&path)? {
            Some(JournalState::Claimed { request_sha256 }) if request_sha256 == digest => {
                self.replace(&path, &JournalState::Submitted { request_sha256 })
            }
            _ => Err(JournalError::InvalidTransition),
        }
    }

    fn complete(
        &mut self,
        key: &JournalKey,
        digest: &str,
        result: &BluetoothRestartResult,
    ) -> Result<(), JournalError> {
        let path = self.path_for(key)?;
        match self.load(&path)? {
            Some(state)
                if state.digest() == digest && !matches!(state, JournalState::Completed { .. }) =>
            {
                self.replace(
                    &path,
                    &JournalState::Completed {
                        request_sha256: digest.into(),
                        result: Box::new(result.clone()),
                    },
                )
            }
            _ => Err(JournalError::InvalidTransition),
        }
    }
}

fn validate_directory(path: &Path, owner_uid: u32) -> Result<(), JournalError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| JournalError::Unavailable)?;
    if !metadata.file_type().is_dir()
        || metadata.uid() != owner_uid
        || metadata.permissions().mode() & 0o777 != DIRECTORY_MODE
    {
        return Err(JournalError::Unavailable);
    }
    Ok(())
}

fn validate_file(file: &File, owner_uid: u32) -> Result<(), JournalError> {
    let metadata = file.metadata().map_err(|_| JournalError::Unavailable)?;
    if !metadata.file_type().is_file()
        || metadata.uid() != owner_uid
        || metadata.permissions().mode() & 0o777 != FILE_MODE
    {
        return Err(JournalError::Unavailable);
    }
    Ok(())
}

fn encode(state: &JournalState) -> Result<Vec<u8>, JournalError> {
    let bytes = serde_json::to_vec(state).map_err(|_| JournalError::Unavailable)?;
    if bytes.is_empty() || bytes.len() as u64 > MAX_ENTRY_BYTES {
        return Err(JournalError::Unavailable);
    }
    Ok(bytes)
}

fn write_and_sync(file: &mut File, bytes: &[u8]) -> Result<(), JournalError> {
    file.write_all(bytes)
        .map_err(|_| JournalError::Unavailable)?;
    file.sync_all().map_err(|_| JournalError::Unavailable)
}

fn sync_directory(path: &Path) -> Result<(), JournalError> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|_| JournalError::Unavailable)
}

fn valid_entry_name(name: &str) -> bool {
    let Some((uid, rest)) = name.split_once('-') else {
        return false;
    };
    let Some(key) = rest.strip_suffix(".json") else {
        return false;
    };
    uid.parse::<u32>().is_ok()
        && key.len() == 32
        && key
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use blossom_core::privileged::{
        BluetoothRestartFailure, BluetoothRestartOutcome, BluetoothRestartRequest,
        PRIVILEGED_PROTOCOL_VERSION,
    };
    use std::os::unix::fs::symlink;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "blossom-journal-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            fs::create_dir(&path).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(DIRECTORY_MODE)).unwrap();
            Self(path)
        }

        fn journal(&self) -> FileJournal {
            FileJournal::open_owned_by(&self.0, self.owner_uid()).unwrap()
        }

        fn owner_uid(&self) -> u32 {
            fs::symlink_metadata(&self.0).unwrap().uid()
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn key() -> JournalKey {
        JournalKey {
            caller_uid: 1000,
            idempotency_key: "0".repeat(32),
        }
    }

    fn request() -> BluetoothRestartRequest {
        BluetoothRestartRequest {
            version: PRIVILEGED_PROTOCOL_VERSION,
            correlation_id: "request-1".into(),
            idempotency_key: key().idempotency_key,
            interactive: true,
        }
    }

    fn failure_result(digest: &str) -> BluetoothRestartResult {
        BluetoothRestartResult {
            version: PRIVILEGED_PROTOCOL_VERSION,
            correlation_id: "request-1".into(),
            authenticated_uid: 1000,
            request_sha256: digest.into(),
            replayed: false,
            outcome: BluetoothRestartOutcome::Failed {
                error: BluetoothRestartFailure::Denied,
                job_submitted: false,
            },
        }
    }

    #[test]
    fn persists_each_transition_and_recovers_completed_result() {
        let directory = TestDirectory::new();
        let digest = request().normalized_digest(1000).unwrap();
        directory.journal().claim(&key(), &digest).unwrap();
        assert!(matches!(
            directory.journal().claim(&key(), &digest).unwrap(),
            ClaimOutcome::Existing(JournalState::Claimed { .. })
        ));
        directory.journal().mark_submitted(&key(), &digest).unwrap();
        assert!(matches!(
            directory.journal().claim(&key(), &digest).unwrap(),
            ClaimOutcome::Existing(JournalState::Submitted { .. })
        ));
        let result = failure_result(&digest);
        directory
            .journal()
            .complete(&key(), &digest, &result)
            .unwrap();
        assert!(matches!(
            directory.journal().claim(&key(), &digest).unwrap(),
            ClaimOutcome::Existing(JournalState::Completed { .. })
        ));
    }

    #[test]
    fn rejects_digest_reuse_invalid_transitions_and_malformed_keys() {
        let directory = TestDirectory::new();
        let digest = request().normalized_digest(1000).unwrap();
        let mut journal = directory.journal();
        journal.claim(&key(), &digest).unwrap();
        assert_eq!(
            journal.claim(&key(), &"1".repeat(64)).unwrap(),
            ClaimOutcome::DigestMismatch
        );
        assert_eq!(
            journal.complete(&key(), &"1".repeat(64), &failure_result(&digest)),
            Err(JournalError::InvalidTransition)
        );
        let mut malformed = key();
        malformed.idempotency_key = "../escape".into();
        assert_eq!(
            journal.claim(&malformed, &digest),
            Err(JournalError::InvalidTransition)
        );
    }

    #[test]
    fn rejects_symlinked_directory_file_corruption_and_unsafe_modes() {
        let directory = TestDirectory::new();
        let link = directory.0.with_extension("link");
        symlink(&directory.0, &link).unwrap();
        assert!(FileJournal::open_owned_by(&link, directory.owner_uid()).is_err());
        fs::remove_file(&link).unwrap();

        fs::set_permissions(&directory.0, fs::Permissions::from_mode(0o755)).unwrap();
        assert!(FileJournal::open_owned_by(&directory.0, directory.owner_uid()).is_err());
        fs::set_permissions(&directory.0, fs::Permissions::from_mode(DIRECTORY_MODE)).unwrap();

        let path = directory.journal().path_for(&key()).unwrap();
        fs::write(&path, b"not-json").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(FILE_MODE)).unwrap();
        assert!(directory.journal().claim(&key(), &"1".repeat(64)).is_err());
        fs::remove_file(&path).unwrap();

        let mut journal = directory.journal();
        let target = directory.0.join("target");
        fs::write(&target, b"{}").unwrap();
        fs::set_permissions(&target, fs::Permissions::from_mode(FILE_MODE)).unwrap();
        symlink(&target, &path).unwrap();
        assert!(journal.claim(&key(), &"1".repeat(64)).is_err());
    }

    #[test]
    fn rejects_unknown_files_and_stale_atomic_replacements() {
        let directory = TestDirectory::new();
        fs::write(directory.0.join("foreign"), b"x").unwrap();
        assert!(FileJournal::open_owned_by(&directory.0, directory.owner_uid()).is_err());
    }

    #[test]
    fn rejects_a_write_that_would_cross_the_total_byte_bound() {
        let directory = TestDirectory::new();
        for index in 0..255u32 {
            let path = directory.0.join(format!("1000-{index:032x}.json"));
            fs::write(&path, vec![b' '; 4_112]).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(FILE_MODE)).unwrap();
        }
        let mut journal = directory.journal();
        let mut new_key = key();
        new_key.idempotency_key = "f".repeat(32);
        assert_eq!(
            journal.claim(&new_key, &"1".repeat(64)),
            Err(JournalError::Unavailable)
        );
    }
}
