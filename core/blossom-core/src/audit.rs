use crate::approval::ApprovalError;
use crate::executor::{ExecutionResult, ExecutorError};
use crate::memory_summary::{MemorySummary, MemorySummaryError};
use crate::os_identity::{OsIdentity, OsIdentityError};
use crate::policy::{Capability, PolicyDecision};
use crate::process_list::{ProcessList, ProcessListError};
use crate::process_self::{ProcessSelf, ProcessSelfError};
use crate::request::ToolRequest;
use crate::storage_summary::{StorageSummary, StorageSummaryError};
use crate::uptime::{SystemUptime, UptimeError};
use crate::verification::Verification;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fmt::Write;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AuditEvent {
    RequestRejected {
        category: String,
    },
    RequestAccepted {
        request_id: String,
        tool: String,
    },
    PolicyEvaluated {
        request_id: String,
        capability: Capability,
        decision: PolicyDecision,
    },
    ApprovalIssued {
        request_id: String,
    },
    ApprovalRejected {
        request_id: String,
        error: ApprovalError,
    },
    ApprovalConsumed {
        request_id: String,
    },
    ApprovalDenied {
        request_id: String,
    },
    ApprovalCancelled {
        request_id: String,
    },
    ExecutionStarted {
        request_id: String,
        program: String,
    },
    ExecutionFinished {
        request_id: String,
        exit_code: Option<i32>,
        stdout_bytes: usize,
        stderr_bytes: usize,
        stdout_sha256: String,
        stderr_sha256: String,
        timed_out: bool,
        output_truncated: bool,
    },
    ExecutionFailed {
        request_id: String,
        error: ExecutorError,
    },
    NativeReadStarted {
        request_id: String,
        resource: String,
    },
    OsIdentityReadFinished {
        request_id: String,
        source_path: String,
        source_sha256: String,
        source_bytes: usize,
    },
    UptimeReadFinished {
        request_id: String,
        source_path: String,
        source_sha256: String,
        source_bytes: usize,
    },
    MemorySummaryReadFinished {
        request_id: String,
        source_path: String,
        source_sha256: String,
        source_bytes: usize,
    },
    StorageSummaryReadFinished {
        request_id: String,
        resource_path: String,
        source: String,
    },
    ProcessSelfReadFinished {
        request_id: String,
        source: String,
    },
    ProcessListReadFinished {
        request_id: String,
        source: String,
        returned_entries: usize,
        skipped_entries: u32,
        truncated: bool,
    },
    NativeReadFailed {
        request_id: String,
        resource: String,
        error: OsIdentityError,
    },
    UptimeReadFailed {
        request_id: String,
        resource: String,
        error: UptimeError,
    },
    MemorySummaryReadFailed {
        request_id: String,
        resource: String,
        error: MemorySummaryError,
    },
    StorageSummaryReadFailed {
        request_id: String,
        resource: String,
        error: StorageSummaryError,
    },
    ProcessSelfReadFailed {
        request_id: String,
        resource: String,
        error: ProcessSelfError,
    },
    ProcessListReadFailed {
        request_id: String,
        resource: String,
        error: ProcessListError,
    },
    VerificationFinished {
        request_id: String,
        verification: Verification,
    },
    Denied {
        request_id: String,
    },
}

impl AuditEvent {
    pub fn execution_finished(request: &ToolRequest, result: &ExecutionResult) -> Self {
        Self::ExecutionFinished {
            request_id: request.request_id().as_str().into(),
            exit_code: result.exit_code,
            stdout_bytes: result.stdout.len(),
            stderr_bytes: result.stderr.len(),
            stdout_sha256: digest(&result.stdout),
            stderr_sha256: digest(&result.stderr),
            timed_out: result.timed_out,
            output_truncated: result.output_truncated,
        }
    }

    pub fn os_identity_finished(request: &ToolRequest, identity: &OsIdentity) -> Self {
        Self::OsIdentityReadFinished {
            request_id: request.request_id().as_str().into(),
            source_path: identity.source_path.clone(),
            source_sha256: identity.source_sha256.clone(),
            source_bytes: identity.source_bytes,
        }
    }

    pub fn uptime_finished(request: &ToolRequest, uptime: &SystemUptime) -> Self {
        Self::UptimeReadFinished {
            request_id: request.request_id().as_str().into(),
            source_path: uptime.source_path.clone(),
            source_sha256: uptime.source_sha256.clone(),
            source_bytes: uptime.source_bytes,
        }
    }

    pub fn memory_summary_finished(request: &ToolRequest, summary: &MemorySummary) -> Self {
        Self::MemorySummaryReadFinished {
            request_id: request.request_id().as_str().into(),
            source_path: summary.source_path.clone(),
            source_sha256: summary.source_sha256.clone(),
            source_bytes: summary.source_bytes,
        }
    }

    pub fn storage_summary_finished(request: &ToolRequest, summary: &StorageSummary) -> Self {
        Self::StorageSummaryReadFinished {
            request_id: request.request_id().as_str().into(),
            resource_path: summary.resource_path.clone(),
            source: "statvfs".into(),
        }
    }

    pub fn process_self_finished(request: &ToolRequest, _identity: &ProcessSelf) -> Self {
        Self::ProcessSelfReadFinished {
            request_id: request.request_id().as_str().into(),
            source: "native_process_identity".into(),
        }
    }

    pub fn process_list_finished(request: &ToolRequest, list: &ProcessList) -> Self {
        Self::ProcessListReadFinished {
            request_id: request.request_id().as_str().into(),
            source: "proc_status_same_effective_user".into(),
            returned_entries: list.processes.len(),
            skipped_entries: list.skipped_entries,
            truncated: list.truncated,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AuditRecord {
    pub sequence: u64,
    pub previous_hash: String,
    pub event: AuditEvent,
    pub record_hash: String,
}

#[derive(Clone, Debug, Default)]
pub struct AuditLog {
    records: Vec<AuditRecord>,
}

impl AuditLog {
    pub fn append(&mut self, event: AuditEvent) {
        let sequence = self.records.len() as u64 + 1;
        let previous_hash = self
            .records
            .last()
            .map_or_else(|| "0".repeat(64), |record| record.record_hash.clone());
        let record_hash = hash_record(sequence, &previous_hash, &event);
        self.records.push(AuditRecord {
            sequence,
            previous_hash,
            event,
            record_hash,
        });
    }

    pub fn records(&self) -> &[AuditRecord] {
        &self.records
    }

    pub fn verify_chain(&self) -> bool {
        let mut expected_previous = "0".repeat(64);
        for (index, record) in self.records.iter().enumerate() {
            if record.sequence != index as u64 + 1 || record.previous_hash != expected_previous {
                return false;
            }
            if record.record_hash
                != hash_record(record.sequence, &record.previous_hash, &record.event)
            {
                return false;
            }
            expected_previous = record.record_hash.clone();
        }
        true
    }
}

fn hash_record(sequence: u64, previous_hash: &str, event: &AuditEvent) -> String {
    #[derive(Serialize)]
    struct HashMaterial<'a> {
        sequence: u64,
        previous_hash: &'a str,
        event: &'a AuditEvent,
    }
    let encoded = serde_json::to_vec(&HashMaterial {
        sequence,
        previous_hash,
        event,
    })
    .expect("audit events contain only serializable internal types");
    digest(&encoded)
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

    #[test]
    fn chains_records_and_redacts_output_content() {
        let mut audit = AuditLog::default();
        audit.append(AuditEvent::Denied {
            request_id: "req-1".into(),
        });
        let request = ToolRequest::parse_json(
            r#"{"request_id":"req-2","tool":"system.uname","arguments":{}}"#,
        )
        .expect("valid request");
        audit.append(AuditEvent::execution_finished(
            &request,
            &ExecutionResult {
                exit_code: Some(0),
                stdout: b"secret-output".to_vec(),
                stderr: Vec::new(),
                timed_out: false,
                output_truncated: false,
            },
        ));
        assert!(audit.verify_chain());
        let encoded = serde_json::to_string(audit.records()).expect("serializable audit records");
        assert!(!encoded.contains("secret-output"));
    }

    #[test]
    fn os_identity_audit_records_provenance_not_identity_values() {
        let request = ToolRequest::parse_json(
            r#"{"request_id":"req-os","tool":"system.os.identity","arguments":{}}"#,
        )
        .expect("valid OS identity request");
        let identity = OsIdentity {
            source: crate::os_identity::OsReleaseSource::EtcOsRelease,
            source_path: "/etc/os-release".into(),
            source_sha256: "a".repeat(64),
            source_bytes: 42,
            id: Some("private-id-value".into()),
            name: Some("private-name-value".into()),
            pretty_name: None,
            version_id: None,
            version_codename: None,
            build_id: None,
            variant_id: None,
        };
        let mut audit = AuditLog::default();
        audit.append(AuditEvent::os_identity_finished(&request, &identity));
        let encoded = serde_json::to_string(audit.records()).expect("serializable audit records");
        assert!(encoded.contains("/etc/os-release"));
        assert!(encoded.contains(&"a".repeat(64)));
        assert!(!encoded.contains("private-id-value"));
        assert!(!encoded.contains("private-name-value"));
    }

    #[test]
    fn uptime_audit_records_provenance_not_duration_or_idle_values() {
        let request = ToolRequest::parse_json(
            r#"{"request_id":"req-up","tool":"system.uptime","arguments":{}}"#,
        )
        .expect("valid uptime request");
        let uptime = SystemUptime {
            seconds: 12_345,
            nanoseconds: 670_000_000,
            source_path: "/proc/uptime".into(),
            source_sha256: "b".repeat(64),
            source_bytes: 20,
        };
        let mut audit = AuditLog::default();
        audit.append(AuditEvent::uptime_finished(&request, &uptime));
        let encoded = serde_json::to_string(audit.records()).expect("serializable audit records");
        assert!(encoded.contains("/proc/uptime"));
        assert!(!encoded.contains("12345"));
        assert!(!encoded.contains("670000000"));
    }

    #[test]
    fn memory_audit_records_provenance_not_memory_values() {
        let request = ToolRequest::parse_json(
            r#"{"request_id":"req-memory","tool":"system.memory.summary","arguments":{}}"#,
        )
        .expect("valid memory summary request");
        let summary = MemorySummary {
            total_bytes: 17_179_869_184,
            available_bytes: 8_589_934_592,
            swap_total_bytes: 4_294_967_296,
            swap_free_bytes: 2_147_483_648,
            source_path: "/proc/meminfo".into(),
            source_sha256: "c".repeat(64),
            source_bytes: 128,
        };
        let mut audit = AuditLog::default();
        audit.append(AuditEvent::memory_summary_finished(&request, &summary));
        let encoded = serde_json::to_string(audit.records()).expect("serializable audit records");
        assert!(encoded.contains("/proc/meminfo"));
        assert!(!encoded.contains("17179869184"));
        assert!(!encoded.contains("8589934592"));
    }

    #[test]
    fn storage_audit_records_scope_not_capacity_values() {
        let request = ToolRequest::parse_json(
            r#"{"request_id":"req-storage","tool":"system.storage.summary","arguments":{}}"#,
        )
        .expect("valid storage summary request");
        let summary = StorageSummary {
            source: crate::storage_summary::StorageSummarySource::RootStatvfs,
            resource_path: "/".into(),
            total_bytes: 987_654_321,
            available_bytes: 123_456_789,
        };
        let mut audit = AuditLog::default();
        audit.append(AuditEvent::storage_summary_finished(&request, &summary));
        let encoded = serde_json::to_string(audit.records()).expect("serializable audit records");
        assert!(encoded.contains("statvfs"));
        assert!(!encoded.contains("987654321"));
        assert!(!encoded.contains("123456789"));
    }

    #[test]
    fn process_self_audit_omits_process_and_user_identifiers() {
        let request = ToolRequest::parse_json(
            r#"{"request_id":"req-self","tool":"process.self","arguments":{}}"#,
        )
        .expect("valid process self request");
        let identity = ProcessSelf {
            source: crate::process_self::ProcessSelfSource::NativeProcessIdentity,
            process_id: 987_654,
            parent_process_id: 876_543,
            effective_user_id: 765_432,
            effective_group_id: 654_321,
        };
        let mut audit = AuditLog::default();
        audit.append(AuditEvent::process_self_finished(&request, &identity));
        let encoded = serde_json::to_string(audit.records()).expect("serializable audit records");
        assert!(encoded.contains("native_process_identity"));
        assert!(!encoded.contains("987654"));
        assert!(!encoded.contains("876543"));
        assert!(!encoded.contains("765432"));
        assert!(!encoded.contains("654321"));
    }
}
