use crate::approval::ApprovalError;
use crate::executor::{ExecutionResult, ExecutorError};
use crate::os_identity::{OsIdentity, OsIdentityError};
use crate::policy::{Capability, PolicyDecision};
use crate::request::ToolRequest;
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
}
