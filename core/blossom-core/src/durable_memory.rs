use crate::Capability;
use serde::{Deserialize, Serialize};
use std::fmt;

pub const MEMORY_SCHEMA_VERSION: u16 = 1;
pub const MAX_DURABLE_NOTE_BYTES: usize = 4 * 1024;
pub const MAX_MEMORY_RECORD_ID_BYTES: usize = 64;
pub const MAX_ENCRYPTED_MEMORY_STORE_BYTES: usize = 1024 * 1024;
pub const MEMORY_DIRECTORY_MODE: u32 = 0o700;
pub const MEMORY_FILE_MODE: u32 = 0o600;
const STORE_MAGIC: &[u8; 8] = b"BLSMEM01";
const STORE_AAD: &[u8] = b"blossom.memory.store.v1";
const STORE_NONCE_BYTES: usize = 24;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryClass {
    SessionContext,
    TemporaryMemory,
    DurableMemory,
    ProjectKnowledge,
    SystemHistory,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryOperation {
    Create,
    Inspect,
    Edit,
    Delete,
    Export,
    Disable,
    SetRetention,
    Recall,
    Enable,
}

impl MemoryOperation {
    pub fn capability(self) -> Capability {
        match self {
            Self::Create => Capability::MemoryDurableCreate,
            Self::Inspect => Capability::MemoryDurableInspect,
            Self::Edit => Capability::MemoryDurableEdit,
            Self::Delete => Capability::MemoryDurableDelete,
            Self::Export => Capability::MemoryDurableExport,
            Self::Disable => Capability::MemoryDurableDisable,
            Self::SetRetention => Capability::MemoryDurableSetRetention,
            Self::Recall => Capability::MemoryDurableRecall,
            Self::Enable => Capability::MemoryDurableEnable,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryPurpose {
    UserPreference,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryScope {
    User,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryRetention {
    UntilDeleted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryConsumer {
    UserControls,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryInputProvenance {
    UserAuthored,
    ModelAuthored,
    ToolOutput,
    ContextObservation,
    ThirdPartyContent,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
pub struct DurableNoteDraft {
    pub schema_version: u16,
    pub class: MemoryClass,
    pub operation: MemoryOperation,
    pub purpose: MemoryPurpose,
    pub scope: MemoryScope,
    pub retention: MemoryRetention,
    pub consumers: Vec<MemoryConsumer>,
    pub provenance: MemoryInputProvenance,
    pub value: String,
}

impl DurableNoteDraft {
    pub fn explicit_user_note(value: String) -> Self {
        Self {
            schema_version: MEMORY_SCHEMA_VERSION,
            class: MemoryClass::DurableMemory,
            operation: MemoryOperation::Create,
            purpose: MemoryPurpose::UserPreference,
            scope: MemoryScope::User,
            retention: MemoryRetention::UntilDeleted,
            consumers: vec![MemoryConsumer::UserControls],
            provenance: MemoryInputProvenance::UserAuthored,
            value,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DurableMemoryLifecycle {
    Proposed,
    Approved,
    Committed,
    Deleted,
}

impl DurableMemoryLifecycle {
    pub fn transition(self, next: Self) -> Result<Self, MemoryValidationError> {
        match (self, next) {
            (Self::Proposed, Self::Approved)
            | (Self::Approved, Self::Committed)
            | (Self::Committed, Self::Deleted) => Ok(next),
            _ => Err(MemoryValidationError::InvalidLifecycleTransition),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryValidationError {
    UnsupportedSchema,
    WrongClass,
    WrongOperation,
    UnapprovedProvenance,
    EmptyValue,
    ValueTooLarge,
    InvalidConsumers,
    InvalidRecordId,
    InvalidLifecycleTransition,
}

impl fmt::Display for MemoryValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::UnsupportedSchema => "memory schema is unsupported",
            Self::WrongClass => "memory class is not the fixed durable class",
            Self::WrongOperation => "memory operation is not the fixed create operation",
            Self::UnapprovedProvenance => "memory value is not explicitly user-authored",
            Self::EmptyValue => "memory value is empty",
            Self::ValueTooLarge => "memory value exceeds the fixed size limit",
            Self::InvalidConsumers => "memory consumers are not the fixed first-slice set",
            Self::InvalidRecordId => "memory record identifier is invalid",
            Self::InvalidLifecycleTransition => "memory lifecycle transition is invalid",
        })
    }
}

impl std::error::Error for MemoryValidationError {}

pub fn validate_durable_note_draft(draft: &DurableNoteDraft) -> Result<(), MemoryValidationError> {
    if draft.schema_version != MEMORY_SCHEMA_VERSION {
        return Err(MemoryValidationError::UnsupportedSchema);
    }
    if draft.class != MemoryClass::DurableMemory {
        return Err(MemoryValidationError::WrongClass);
    }
    if draft.operation != MemoryOperation::Create {
        return Err(MemoryValidationError::WrongOperation);
    }
    if draft.provenance != MemoryInputProvenance::UserAuthored {
        return Err(MemoryValidationError::UnapprovedProvenance);
    }
    if draft.value.is_empty() {
        return Err(MemoryValidationError::EmptyValue);
    }
    if draft.value.len() > MAX_DURABLE_NOTE_BYTES {
        return Err(MemoryValidationError::ValueTooLarge);
    }
    if draft.consumers.as_slice() != [MemoryConsumer::UserControls] {
        return Err(MemoryValidationError::InvalidConsumers);
    }
    Ok(())
}

pub fn validate_memory_record_id(value: &str) -> Result<(), MemoryValidationError> {
    let valid = !value.is_empty()
        && value.len() <= MAX_MEMORY_RECORD_ID_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'));
    if valid {
        Ok(())
    } else {
        Err(MemoryValidationError::InvalidRecordId)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DurableMemoryRecord {
    pub record_id: String,
    pub schema_version: u16,
    pub purpose: MemoryPurpose,
    pub scope: MemoryScope,
    pub retention: MemoryRetention,
    pub consumers: Vec<MemoryConsumer>,
    pub created_at_unix_ms: u64,
    pub last_user_edit_at_unix_ms: u64,
    pub value: String,
}

impl DurableMemoryRecord {
    pub fn validate(&self) -> Result<(), MemoryValidationError> {
        validate_memory_record_id(&self.record_id)?;
        if self.schema_version != MEMORY_SCHEMA_VERSION {
            return Err(MemoryValidationError::UnsupportedSchema);
        }
        if self.consumers.as_slice() != [MemoryConsumer::UserControls] {
            return Err(MemoryValidationError::InvalidConsumers);
        }
        if self.value.is_empty() {
            return Err(MemoryValidationError::EmptyValue);
        }
        if self.value.len() > MAX_DURABLE_NOTE_BYTES {
            return Err(MemoryValidationError::ValueTooLarge);
        }
        if self.last_user_edit_at_unix_ms < self.created_at_unix_ms {
            return Err(MemoryValidationError::InvalidLifecycleTransition);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DurableMemoryStoreEnvelope {
    schema_version: u16,
    records: Vec<DurableMemoryRecord>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryStoreError {
    UnsupportedPlatform,
    InvalidPath,
    UnsafePermissions,
    KeyUnavailable,
    RandomnessUnavailable,
    StoreUnavailable,
    StoreMissing,
    StoreTooLarge,
    InvalidRecord,
    DuplicateRecord,
    EncryptionFailed,
    AuthenticationFailed,
    InvalidEnvelope,
    WriteFailed,
    DurabilityFailed,
}

impl fmt::Display for MemoryStoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::UnsupportedPlatform => "encrypted memory store requires a supported Unix host",
            Self::InvalidPath => "memory store path is invalid",
            Self::UnsafePermissions => "memory store permissions are unsafe",
            Self::KeyUnavailable => "memory store key is unavailable",
            Self::RandomnessUnavailable => "secure operating-system randomness is unavailable",
            Self::StoreUnavailable => "memory store is unavailable",
            Self::StoreMissing => "memory store has not been created",
            Self::StoreTooLarge => "memory store exceeds the fixed size limit",
            Self::InvalidRecord => "memory store contains an invalid record",
            Self::DuplicateRecord => "memory store contains a duplicate record",
            Self::EncryptionFailed => "memory store encryption failed",
            Self::AuthenticationFailed => "memory store authentication failed",
            Self::InvalidEnvelope => "memory store envelope is invalid",
            Self::WriteFailed => "memory store write failed",
            Self::DurabilityFailed => "memory store durability could not be verified",
        })
    }
}

impl std::error::Error for MemoryStoreError {}

#[cfg(unix)]
#[derive(Clone, Debug)]
pub struct EncryptedMemoryStore {
    data_directory: std::path::PathBuf,
    key_directory: std::path::PathBuf,
}

#[cfg(unix)]
impl EncryptedMemoryStore {
    pub fn initialize_inactive(
        data_directory: &std::path::Path,
        key_directory: &std::path::Path,
    ) -> Result<Self, MemoryStoreError> {
        if !data_directory.is_absolute()
            || !key_directory.is_absolute()
            || data_directory == key_directory
        {
            return Err(MemoryStoreError::InvalidPath);
        }
        create_private_directory(data_directory)?;
        create_private_directory(key_directory)?;
        let store = Self {
            data_directory: data_directory.to_path_buf(),
            key_directory: key_directory.to_path_buf(),
        };
        store.load_or_create_key()?;
        Ok(store)
    }

    pub fn load(&self) -> Result<Vec<DurableMemoryRecord>, MemoryStoreError> {
        use chacha20poly1305::aead::{Aead, Payload};
        use chacha20poly1305::{KeyInit, XChaCha20Poly1305, XNonce};
        use std::io::Read;

        let key = self.load_or_create_key()?;
        let path = self.data_directory.join("records.v1");
        let mut file = match open_private_regular(&path, MemoryStoreError::StoreUnavailable) {
            Err(MemoryStoreError::StoreUnavailable) if !path.exists() => {
                return Err(MemoryStoreError::StoreMissing);
            }
            result => result?,
        };
        let length = file
            .metadata()
            .map_err(|_| MemoryStoreError::StoreUnavailable)?
            .len() as usize;
        if length > MAX_ENCRYPTED_MEMORY_STORE_BYTES
            || length < STORE_MAGIC.len() + STORE_NONCE_BYTES
        {
            return Err(MemoryStoreError::StoreTooLarge);
        }
        let mut encoded = Vec::with_capacity(length);
        file.read_to_end(&mut encoded)
            .map_err(|_| MemoryStoreError::StoreUnavailable)?;
        if encoded.len() != length || !encoded.starts_with(STORE_MAGIC) {
            return Err(MemoryStoreError::InvalidEnvelope);
        }
        let nonce_start = STORE_MAGIC.len();
        let ciphertext_start = nonce_start + STORE_NONCE_BYTES;
        let nonce = XNonce::from_slice(&encoded[nonce_start..ciphertext_start]);
        let cipher = XChaCha20Poly1305::new(chacha20poly1305::Key::from_slice(&key[..]));
        let plaintext = zeroize::Zeroizing::new(
            cipher
                .decrypt(
                    nonce,
                    Payload {
                        msg: &encoded[ciphertext_start..],
                        aad: STORE_AAD,
                    },
                )
                .map_err(|_| MemoryStoreError::AuthenticationFailed)?,
        );
        let envelope: DurableMemoryStoreEnvelope =
            serde_json::from_slice(&plaintext).map_err(|_| MemoryStoreError::InvalidEnvelope)?;
        validate_store_envelope(&envelope)?;
        Ok(envelope.records)
    }

    pub fn replace_all(&self, records: &[DurableMemoryRecord]) -> Result<(), MemoryStoreError> {
        use chacha20poly1305::aead::{Aead, Payload};
        use chacha20poly1305::{KeyInit, XChaCha20Poly1305, XNonce};
        use std::fs::OpenOptions;
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;

        let envelope = DurableMemoryStoreEnvelope {
            schema_version: MEMORY_SCHEMA_VERSION,
            records: records.to_vec(),
        };
        validate_store_envelope(&envelope)?;
        let plaintext = zeroize::Zeroizing::new(
            serde_json::to_vec(&envelope).map_err(|_| MemoryStoreError::InvalidEnvelope)?,
        );
        if plaintext.len() > MAX_ENCRYPTED_MEMORY_STORE_BYTES / 2 {
            return Err(MemoryStoreError::StoreTooLarge);
        }
        let key = self.load_or_create_key()?;
        let mut nonce_bytes = [0u8; STORE_NONCE_BYTES];
        getrandom::fill(&mut nonce_bytes).map_err(|_| MemoryStoreError::RandomnessUnavailable)?;
        let cipher = XChaCha20Poly1305::new(chacha20poly1305::Key::from_slice(&key[..]));
        let ciphertext = cipher
            .encrypt(
                XNonce::from_slice(&nonce_bytes),
                Payload {
                    msg: &plaintext,
                    aad: STORE_AAD,
                },
            )
            .map_err(|_| MemoryStoreError::EncryptionFailed)?;
        let total_length = STORE_MAGIC.len() + nonce_bytes.len() + ciphertext.len();
        if total_length > MAX_ENCRYPTED_MEMORY_STORE_BYTES {
            return Err(MemoryStoreError::StoreTooLarge);
        }

        let mut suffix = [0u8; 16];
        getrandom::fill(&mut suffix).map_err(|_| MemoryStoreError::RandomnessUnavailable)?;
        let temporary_name = format!(
            ".records.v1.tmp-{}",
            suffix
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        );
        let temporary_path = self.data_directory.join(temporary_name);
        let final_path = self.data_directory.join("records.v1");
        let result = (|| {
            let mut temporary = OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(MEMORY_FILE_MODE)
                .open(&temporary_path)
                .map_err(|_| MemoryStoreError::WriteFailed)?;
            temporary
                .write_all(STORE_MAGIC)
                .and_then(|_| temporary.write_all(&nonce_bytes))
                .and_then(|_| temporary.write_all(&ciphertext))
                .map_err(|_| MemoryStoreError::WriteFailed)?;
            temporary
                .sync_all()
                .map_err(|_| MemoryStoreError::DurabilityFailed)?;
            std::fs::rename(&temporary_path, &final_path)
                .map_err(|_| MemoryStoreError::WriteFailed)?;
            std::fs::File::open(&self.data_directory)
                .and_then(|directory| directory.sync_all())
                .map_err(|_| MemoryStoreError::DurabilityFailed)
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&temporary_path);
        }
        result
    }

    fn load_or_create_key(&self) -> Result<zeroize::Zeroizing<[u8; 32]>, MemoryStoreError> {
        use std::fs::OpenOptions;
        use std::io::{Read, Write};
        use std::os::unix::fs::OpenOptionsExt;

        let path = self.key_directory.join("memory.v1.key");
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(MEMORY_FILE_MODE)
            .open(&path)
        {
            Ok(mut file) => {
                let mut key = [0u8; 32];
                let result = (|| {
                    getrandom::fill(&mut key)
                        .map_err(|_| MemoryStoreError::RandomnessUnavailable)?;
                    file.write_all(&key)
                        .and_then(|_| file.sync_all())
                        .map_err(|_| MemoryStoreError::KeyUnavailable)?;
                    std::fs::File::open(&self.key_directory)
                        .and_then(|directory| directory.sync_all())
                        .map_err(|_| MemoryStoreError::DurabilityFailed)?;
                    Ok(zeroize::Zeroizing::new(key))
                })();
                if result.is_err() {
                    drop(file);
                    let _ = std::fs::remove_file(&path);
                }
                result
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                let mut file = open_private_regular(&path, MemoryStoreError::KeyUnavailable)?;
                let mut key = [0u8; 32];
                file.read_exact(&mut key)
                    .map_err(|_| MemoryStoreError::KeyUnavailable)?;
                let mut trailing = [0u8; 1];
                if file
                    .read(&mut trailing)
                    .map_err(|_| MemoryStoreError::KeyUnavailable)?
                    != 0
                {
                    return Err(MemoryStoreError::KeyUnavailable);
                }
                Ok(zeroize::Zeroizing::new(key))
            }
            Err(_) => Err(MemoryStoreError::KeyUnavailable),
        }
    }
}

#[cfg(unix)]
fn create_private_directory(path: &std::path::Path) -> Result<(), MemoryStoreError> {
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};

    if !path.exists() {
        let mut builder = std::fs::DirBuilder::new();
        builder.mode(MEMORY_DIRECTORY_MODE);
        builder
            .create(path)
            .map_err(|_| MemoryStoreError::StoreUnavailable)?;
    }
    let metadata =
        std::fs::symlink_metadata(path).map_err(|_| MemoryStoreError::StoreUnavailable)?;
    if !metadata.file_type().is_dir()
        || metadata.file_type().is_symlink()
        || metadata.permissions().mode() & 0o777 != MEMORY_DIRECTORY_MODE
    {
        return Err(MemoryStoreError::UnsafePermissions);
    }
    Ok(())
}

#[cfg(unix)]
fn open_private_regular(
    path: &std::path::Path,
    unavailable: MemoryStoreError,
) -> Result<std::fs::File, MemoryStoreError> {
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

    let file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW)
        .open(path)
        .map_err(|_| unavailable)?;
    let metadata = file.metadata().map_err(|_| unavailable)?;
    if !metadata.file_type().is_file() || metadata.permissions().mode() & 0o777 != MEMORY_FILE_MODE
    {
        return Err(MemoryStoreError::UnsafePermissions);
    }
    Ok(file)
}

fn validate_store_envelope(envelope: &DurableMemoryStoreEnvelope) -> Result<(), MemoryStoreError> {
    if envelope.schema_version != MEMORY_SCHEMA_VERSION {
        return Err(MemoryStoreError::InvalidEnvelope);
    }
    let mut identifiers = std::collections::HashSet::new();
    for record in &envelope.records {
        record
            .validate()
            .map_err(|_| MemoryStoreError::InvalidRecord)?;
        if !identifiers.insert(record.record_id.as_str()) {
            return Err(MemoryStoreError::DuplicateRecord);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PolicyDecision, PolicyEngine};

    #[test]
    fn memory_classes_are_closed_and_stably_serialized() {
        let classes = [
            MemoryClass::SessionContext,
            MemoryClass::TemporaryMemory,
            MemoryClass::DurableMemory,
            MemoryClass::ProjectKnowledge,
            MemoryClass::SystemHistory,
        ];
        assert_eq!(classes.len(), 5);
        assert_eq!(
            serde_json::to_string(&classes).expect("classes serialize"),
            r#"["session_context","temporary_memory","durable_memory","project_knowledge","system_history"]"#
        );
    }

    #[test]
    fn every_memory_operation_is_default_deny() {
        let operations = [
            MemoryOperation::Create,
            MemoryOperation::Inspect,
            MemoryOperation::Edit,
            MemoryOperation::Delete,
            MemoryOperation::Export,
            MemoryOperation::Disable,
            MemoryOperation::SetRetention,
            MemoryOperation::Recall,
            MemoryOperation::Enable,
        ];
        for operation in operations {
            assert_eq!(
                PolicyEngine::default().evaluate_capability(operation.capability()),
                PolicyDecision::Deny
            );
        }
    }

    #[test]
    fn explicit_user_note_has_the_only_valid_first_slice_shape() {
        let draft = DurableNoteDraft::explicit_user_note("Use compact answers".into());
        assert_eq!(validate_durable_note_draft(&draft), Ok(()));
        assert_eq!(
            draft.operation.capability().as_str(),
            "memory.durable:create"
        );
        assert_eq!(MAX_DURABLE_NOTE_BYTES, 4_096);
    }

    #[test]
    fn rejects_non_user_sources_empty_and_oversized_values() {
        for provenance in [
            MemoryInputProvenance::ModelAuthored,
            MemoryInputProvenance::ToolOutput,
            MemoryInputProvenance::ContextObservation,
            MemoryInputProvenance::ThirdPartyContent,
        ] {
            let mut draft = DurableNoteDraft::explicit_user_note("value".into());
            draft.provenance = provenance;
            assert_eq!(
                validate_durable_note_draft(&draft),
                Err(MemoryValidationError::UnapprovedProvenance)
            );
        }

        let empty = DurableNoteDraft::explicit_user_note(String::new());
        assert_eq!(
            validate_durable_note_draft(&empty),
            Err(MemoryValidationError::EmptyValue)
        );

        let oversized = DurableNoteDraft::explicit_user_note("x".repeat(4_097));
        assert_eq!(
            validate_durable_note_draft(&oversized),
            Err(MemoryValidationError::ValueTooLarge)
        );
    }

    #[test]
    fn rejects_schema_class_operation_and_consumer_widening() {
        let mut draft = DurableNoteDraft::explicit_user_note("value".into());
        draft.schema_version = 2;
        assert_eq!(
            validate_durable_note_draft(&draft),
            Err(MemoryValidationError::UnsupportedSchema)
        );

        draft = DurableNoteDraft::explicit_user_note("value".into());
        draft.class = MemoryClass::TemporaryMemory;
        assert_eq!(
            validate_durable_note_draft(&draft),
            Err(MemoryValidationError::WrongClass)
        );

        draft = DurableNoteDraft::explicit_user_note("value".into());
        draft.operation = MemoryOperation::Edit;
        assert_eq!(
            validate_durable_note_draft(&draft),
            Err(MemoryValidationError::WrongOperation)
        );

        draft = DurableNoteDraft::explicit_user_note("value".into());
        draft.consumers.clear();
        assert_eq!(
            validate_durable_note_draft(&draft),
            Err(MemoryValidationError::InvalidConsumers)
        );
    }

    #[test]
    fn record_ids_are_bounded_opaque_and_non_path_like() {
        assert_eq!(validate_memory_record_id("mem-01_test"), Ok(()));
        for invalid in ["", "../note", "note/value", "note value"] {
            assert_eq!(
                validate_memory_record_id(invalid),
                Err(MemoryValidationError::InvalidRecordId)
            );
        }
        assert_eq!(
            validate_memory_record_id(&"x".repeat(65)),
            Err(MemoryValidationError::InvalidRecordId)
        );
    }

    #[test]
    fn lifecycle_is_monotonic_and_cannot_skip_approval() {
        assert_eq!(
            DurableMemoryLifecycle::Proposed.transition(DurableMemoryLifecycle::Approved),
            Ok(DurableMemoryLifecycle::Approved)
        );
        assert_eq!(
            DurableMemoryLifecycle::Approved.transition(DurableMemoryLifecycle::Committed),
            Ok(DurableMemoryLifecycle::Committed)
        );
        assert_eq!(
            DurableMemoryLifecycle::Committed.transition(DurableMemoryLifecycle::Deleted),
            Ok(DurableMemoryLifecycle::Deleted)
        );
        assert_eq!(
            DurableMemoryLifecycle::Proposed.transition(DurableMemoryLifecycle::Committed),
            Err(MemoryValidationError::InvalidLifecycleTransition)
        );
        assert_eq!(
            DurableMemoryLifecycle::Deleted.transition(DurableMemoryLifecycle::Committed),
            Err(MemoryValidationError::InvalidLifecycleTransition)
        );
    }

    #[cfg(unix)]
    fn test_store(label: &str) -> (std::path::PathBuf, EncryptedMemoryStore) {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(1);
        let root = std::env::temp_dir().join(format!(
            "blossom-memory-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let data = root.join("data");
        let keys = root.join("keys");
        std::fs::create_dir(&root).expect("test root created");
        let store = EncryptedMemoryStore::initialize_inactive(&data, &keys)
            .expect("inactive store initialized");
        (root, store)
    }

    #[cfg(unix)]
    fn record(id: &str, value: &str) -> DurableMemoryRecord {
        DurableMemoryRecord {
            record_id: id.into(),
            schema_version: MEMORY_SCHEMA_VERSION,
            purpose: MemoryPurpose::UserPreference,
            scope: MemoryScope::User,
            retention: MemoryRetention::UntilDeleted,
            consumers: vec![MemoryConsumer::UserControls],
            created_at_unix_ms: 10,
            last_user_edit_at_unix_ms: 10,
            value: value.into(),
        }
    }

    #[cfg(unix)]
    #[test]
    fn encrypted_store_round_trips_without_plaintext_on_disk() {
        let (root, store) = test_store("round-trip");
        let expected = vec![record("mem-1", "private preference marker")];
        store.replace_all(&expected).expect("store replaced");
        assert_eq!(store.load().expect("store loaded"), expected);

        let encoded = std::fs::read(root.join("data/records.v1")).expect("store readable");
        assert!(encoded.starts_with(STORE_MAGIC));
        assert!(
            !encoded
                .windows(b"private preference marker".len())
                .any(|window| { window == b"private preference marker" })
        );
        assert_eq!(
            std::fs::read(root.join("keys/memory.v1.key"))
                .expect("key readable")
                .len(),
            32
        );
        std::fs::remove_dir_all(root).expect("test store removed");
    }

    #[cfg(unix)]
    #[test]
    fn ciphertext_corruption_and_wrong_key_fail_authentication() {
        use std::io::{Seek, SeekFrom, Write};

        let (root, store) = test_store("corruption");
        store
            .replace_all(&[record("mem-1", "value")])
            .expect("store replaced");
        let path = root.join("data/records.v1");
        let mut file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .expect("store opened");
        file.seek(SeekFrom::End(-1)).expect("seeked");
        let mut final_byte = [0u8; 1];
        std::io::Read::read_exact(&mut file, &mut final_byte).expect("byte read");
        file.seek(SeekFrom::End(-1)).expect("seeked again");
        file.write_all(&[final_byte[0] ^ 0xff]).expect("corrupted");
        assert_eq!(store.load(), Err(MemoryStoreError::AuthenticationFailed));

        store
            .replace_all(&[record("mem-1", "value")])
            .expect("store restored");
        std::fs::write(root.join("keys/memory.v1.key"), [7u8; 32]).expect("key replaced");
        assert_eq!(store.load(), Err(MemoryStoreError::AuthenticationFailed));
        std::fs::remove_dir_all(root).expect("test store removed");
    }

    #[cfg(unix)]
    #[test]
    fn store_rejects_unsafe_permissions_symlinks_and_duplicate_records() {
        use std::os::unix::fs::{PermissionsExt, symlink};

        let (root, store) = test_store("unsafe");
        let duplicate = record("same", "one");
        assert_eq!(
            store.replace_all(&[duplicate.clone(), duplicate]),
            Err(MemoryStoreError::DuplicateRecord)
        );

        std::fs::set_permissions(
            root.join("keys/memory.v1.key"),
            std::fs::Permissions::from_mode(0o644),
        )
        .expect("permissions changed");
        assert_eq!(store.load(), Err(MemoryStoreError::UnsafePermissions));
        std::fs::remove_dir_all(&root).expect("first test store removed");

        let root = std::env::temp_dir().join(format!("blossom-memory-link-{}", std::process::id()));
        let target = root.join("target");
        std::fs::create_dir(&root).expect("root created");
        std::fs::create_dir(&target).expect("target created");
        symlink(&target, root.join("data")).expect("link created");
        assert!(matches!(
            EncryptedMemoryStore::initialize_inactive(&root.join("data"), &root.join("keys")),
            Err(MemoryStoreError::UnsafePermissions)
        ));
        std::fs::remove_dir_all(root).expect("link test removed");
    }

    #[cfg(unix)]
    #[test]
    fn atomic_replace_leaves_no_temporary_plaintext_or_ciphertext_files() {
        let (root, store) = test_store("atomic");
        store
            .replace_all(&[record("mem-1", "first")])
            .expect("first write");
        store
            .replace_all(&[record("mem-1", "second")])
            .expect("replacement");
        assert_eq!(store.load().expect("loaded")[0].value, "second");
        let names: Vec<_> = std::fs::read_dir(root.join("data"))
            .expect("data listed")
            .map(|entry| entry.expect("entry").file_name())
            .collect();
        assert_eq!(names, vec![std::ffi::OsString::from("records.v1")]);
        std::fs::remove_dir_all(root).expect("test store removed");
    }
}
