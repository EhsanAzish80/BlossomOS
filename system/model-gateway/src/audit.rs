#![cfg_attr(
    not(all(target_os = "linux", feature = "production-private-inference")),
    allow(
        dead_code,
        reason = "persistent gateway audit is target-Linux and package-feature gated"
    )
)]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt::Write as _;
use std::fs::{self, DirBuilder, File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

const AUDIT_FILE: &str = "audit.jsonl";
const DIRECTORY_MODE: u32 = 0o700;
const FILE_MODE: u32 = 0o600;
const MAX_AUDIT_BYTES: u64 = 1024 * 1024;
const MAX_AUDIT_RECORDS: u64 = 4096;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum GatewayAuditOutcome {
    CompletedText,
    CompletedProposals,
    Cancelled,
    ProviderFailed,
    ProtocolRejected,
    DeliveryFailed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum GatewayAdmissionOutcome {
    Authorized,
    CredentialsUnavailable,
    Ineligible,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum GatewayAuditEvent {
    ProcessStarted {
        boot_id_sha256: String,
        instance_sha256: String,
        profile_sha256: String,
    },
    ClientAdmission {
        instance_sha256: String,
        client_uid_sha256: Option<String>,
        outcome: GatewayAdmissionOutcome,
    },
    ConnectionRejected {
        instance_sha256: String,
        client_uid_sha256: String,
        outcome: GatewayAuditOutcome,
    },
    RequestStarted {
        instance_sha256: String,
        client_uid_sha256: String,
        request_id_sha256: String,
        provider: String,
        model_sha256: String,
        deadline_ms: u64,
    },
    RequestTerminal {
        instance_sha256: String,
        request_id_sha256: String,
        outcome: GatewayAuditOutcome,
        elapsed_ms: u64,
        output_bytes: usize,
        proposed_intents: usize,
        prompt_tokens: Option<u64>,
        generated_tokens: Option<u64>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct GatewayAuditRecord {
    sequence: u64,
    previous_sha256: String,
    event: GatewayAuditEvent,
    record_sha256: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct GatewayAuditError;

pub(crate) trait GatewayAudit {
    fn record(&mut self, event: GatewayAuditEvent) -> Result<(), GatewayAuditError>;
}

pub(crate) struct FileGatewayAudit {
    directory: PathBuf,
    owner_uid: u32,
    file: File,
    sequence: u64,
    previous_sha256: String,
    bytes: u64,
    poisoned: bool,
}

impl FileGatewayAudit {
    pub(crate) fn create(directory: &Path, owner_uid: u32) -> Result<Self, GatewayAuditError> {
        let mut builder = DirBuilder::new();
        builder.mode(DIRECTORY_MODE);
        builder.create(directory).map_err(|_| GatewayAuditError)?;
        validate_directory(directory, owner_uid)?;
        let path = directory.join(AUDIT_FILE);
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(FILE_MODE)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(path)
            .map_err(|_| GatewayAuditError)?;
        validate_file(&file, owner_uid)?;
        file.sync_all().map_err(|_| GatewayAuditError)?;
        sync_directory(directory)?;
        Ok(Self {
            directory: directory.to_path_buf(),
            owner_uid,
            file,
            sequence: 0,
            previous_sha256: "0".repeat(64),
            bytes: 0,
            poisoned: false,
        })
    }
}

impl GatewayAudit for FileGatewayAudit {
    fn record(&mut self, event: GatewayAuditEvent) -> Result<(), GatewayAuditError> {
        if self.poisoned || self.sequence >= MAX_AUDIT_RECORDS {
            return Err(GatewayAuditError);
        }
        validate_directory(&self.directory, self.owner_uid)?;
        validate_file(&self.file, self.owner_uid)?;
        let sequence = self.sequence.checked_add(1).ok_or(GatewayAuditError)?;
        let record_sha256 = chain_digest(&self.previous_sha256, &event)?;
        let record = GatewayAuditRecord {
            sequence,
            previous_sha256: self.previous_sha256.clone(),
            event,
            record_sha256: record_sha256.clone(),
        };
        let mut encoded = serde_json::to_vec(&record).map_err(|_| GatewayAuditError)?;
        encoded.push(b'\n');
        if self
            .bytes
            .checked_add(encoded.len() as u64)
            .is_none_or(|total| total > MAX_AUDIT_BYTES)
        {
            return Err(GatewayAuditError);
        }
        if self
            .file
            .write_all(&encoded)
            .and_then(|_| self.file.sync_all())
            .is_err()
        {
            self.poisoned = true;
            return Err(GatewayAuditError);
        }
        self.sequence = sequence;
        self.previous_sha256 = record_sha256;
        self.bytes += encoded.len() as u64;
        Ok(())
    }
}

pub(crate) fn domain_digest(domain: &str, instance_nonce: &str, value: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(domain.as_bytes());
    digest.update([0]);
    digest.update(instance_nonce.as_bytes());
    digest.update([0]);
    digest.update(value);
    hex(&digest.finalize())
}

fn chain_digest(
    previous_sha256: &str,
    event: &GatewayAuditEvent,
) -> Result<String, GatewayAuditError> {
    let mut digest = Sha256::new();
    digest.update(b"blossom.gateway.audit.v1\0");
    digest.update(previous_sha256.as_bytes());
    digest.update([0]);
    digest.update(serde_json::to_vec(event).map_err(|_| GatewayAuditError)?);
    Ok(hex(&digest.finalize()))
}

fn hex(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut encoded, "{byte:02x}").expect("writing to a String cannot fail");
    }
    encoded
}

fn validate_directory(path: &Path, owner_uid: u32) -> Result<(), GatewayAuditError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| GatewayAuditError)?;
    if !metadata.file_type().is_dir()
        || metadata.uid() != owner_uid
        || metadata.permissions().mode() & 0o777 != DIRECTORY_MODE
    {
        return Err(GatewayAuditError);
    }
    for entry in fs::read_dir(path).map_err(|_| GatewayAuditError)? {
        let entry = entry.map_err(|_| GatewayAuditError)?;
        if entry.file_name() != AUDIT_FILE {
            return Err(GatewayAuditError);
        }
    }
    Ok(())
}

fn validate_file(file: &File, owner_uid: u32) -> Result<(), GatewayAuditError> {
    let metadata = file.metadata().map_err(|_| GatewayAuditError)?;
    if !metadata.file_type().is_file()
        || metadata.uid() != owner_uid
        || metadata.permissions().mode() & 0o777 != FILE_MODE
    {
        return Err(GatewayAuditError);
    }
    Ok(())
}

fn sync_directory(path: &Path) -> Result<(), GatewayAuditError> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|_| GatewayAuditError)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "blossom-gateway-audit-{}-{}",
                std::process::id(),
                std::thread::current().name().unwrap_or("unnamed")
            ));
            let _ = fs::remove_dir_all(&path);
            Self(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn started() -> GatewayAuditEvent {
        GatewayAuditEvent::ProcessStarted {
            boot_id_sha256: "a".repeat(64),
            instance_sha256: "b".repeat(64),
            profile_sha256: "c".repeat(64),
        }
    }

    #[test]
    fn writes_synced_hash_chained_content_free_records() {
        let directory = TestDirectory::new();
        let uid = nix::unistd::geteuid().as_raw();
        let mut audit = FileGatewayAudit::create(&directory.0, uid).unwrap();
        audit.record(started()).unwrap();
        audit
            .record(GatewayAuditEvent::RequestTerminal {
                instance_sha256: "b".repeat(64),
                request_id_sha256: "d".repeat(64),
                outcome: GatewayAuditOutcome::CompletedText,
                elapsed_ms: 4,
                output_bytes: 12,
                proposed_intents: 0,
                prompt_tokens: Some(3),
                generated_tokens: Some(2),
            })
            .unwrap();
        let bytes = fs::read(directory.0.join(AUDIT_FILE)).unwrap();
        let text = std::str::from_utf8(&bytes).unwrap();
        assert_eq!(text.lines().count(), 2);
        assert!(!text.contains("private fixture"));
        assert!(!text.contains("generated secret"));
        assert!(!text.contains("blossom-ai"));
        let records: Vec<GatewayAuditRecord> = text
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(records[0].previous_sha256, "0".repeat(64));
        assert_eq!(records[1].previous_sha256, records[0].record_sha256);
        assert_eq!(
            records[1].record_sha256,
            chain_digest(&records[1].previous_sha256, &records[1].event).unwrap()
        );
    }

    #[test]
    fn rejects_stale_symlinked_or_expanded_audit_state() {
        let uid = nix::unistd::geteuid().as_raw();
        let stale = TestDirectory::new();
        DirBuilder::new()
            .mode(DIRECTORY_MODE)
            .create(&stale.0)
            .unwrap();
        assert!(FileGatewayAudit::create(&stale.0, uid).is_err());

        let expanded = TestDirectory::new();
        let mut audit = FileGatewayAudit::create(&expanded.0, uid).unwrap();
        fs::write(expanded.0.join("unexpected"), b"data").unwrap();
        assert!(audit.record(started()).is_err());
    }

    #[test]
    fn domain_digests_are_scoped_to_instance_and_field() {
        let value = 1000_u32.to_be_bytes();
        let first = domain_digest("client_uid", "nonce-a", &value);
        assert_ne!(first, domain_digest("request_id", "nonce-a", &value));
        assert_ne!(first, domain_digest("client_uid", "nonce-b", &value));
        assert!(!first.contains("1000"));
    }
}
