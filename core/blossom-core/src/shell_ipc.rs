use crate::request::{RequestError, RequestId};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt::{self, Write};

pub const SHELL_PROTOCOL_VERSION: u16 = 1;
pub const SHELL_BUS_NAME: &str = "org.blossomos.Shell1";
pub const SHELL_OBJECT_PATH: &str = "/org/blossomos/Shell1";
pub const SHELL_INTERFACE: &str = "org.blossomos.Shell1";
pub const MAX_SHELL_MESSAGE_BYTES: usize = 4_096;
pub const MAX_ACTIVITY_BATCH: u16 = 64;

const DIGEST_HEX_BYTES: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShellDecision {
    ApproveOnce,
    Deny,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShellClientRequest {
    StartSystemUname,
    SubmitDecision {
        request_id: RequestId,
        preview_sha256: String,
        decision: ShellDecision,
    },
    CancelPending {
        request_id: RequestId,
        preview_sha256: String,
    },
    ReadActivity {
        after_sequence: Option<u64>,
        limit: u16,
    },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum ShellClientWire {
    StartSystemUname {
        version: u16,
    },
    SubmitDecision {
        version: u16,
        request_id: String,
        preview_sha256: String,
        decision: ShellDecision,
    },
    CancelPending {
        version: u16,
        request_id: String,
        preview_sha256: String,
    },
    ReadActivity {
        version: u16,
        after_sequence: Option<u64>,
        limit: u16,
    },
}

pub fn decode_shell_client_request(input: &[u8]) -> Result<ShellClientRequest, ShellProtocolError> {
    if input.len() > MAX_SHELL_MESSAGE_BYTES {
        return Err(ShellProtocolError::MessageTooLarge);
    }
    let wire: ShellClientWire =
        serde_json::from_slice(input).map_err(|_| ShellProtocolError::MalformedMessage)?;
    let version = match &wire {
        ShellClientWire::StartSystemUname { version }
        | ShellClientWire::SubmitDecision { version, .. }
        | ShellClientWire::CancelPending { version, .. }
        | ShellClientWire::ReadActivity { version, .. } => *version,
    };
    if version != SHELL_PROTOCOL_VERSION {
        return Err(ShellProtocolError::UnsupportedVersion);
    }

    match wire {
        ShellClientWire::StartSystemUname { .. } => Ok(ShellClientRequest::StartSystemUname),
        ShellClientWire::SubmitDecision {
            request_id,
            preview_sha256,
            decision,
            ..
        } => Ok(ShellClientRequest::SubmitDecision {
            request_id: parse_request_id(request_id)?,
            preview_sha256: validate_digest(preview_sha256)?,
            decision,
        }),
        ShellClientWire::CancelPending {
            request_id,
            preview_sha256,
            ..
        } => Ok(ShellClientRequest::CancelPending {
            request_id: parse_request_id(request_id)?,
            preview_sha256: validate_digest(preview_sha256)?,
        }),
        ShellClientWire::ReadActivity {
            after_sequence,
            limit,
            ..
        } => {
            if !(1..=MAX_ACTIVITY_BATCH).contains(&limit) {
                return Err(ShellProtocolError::InvalidActivityLimit);
            }
            Ok(ShellClientRequest::ReadActivity {
                after_sequence,
                limit,
            })
        }
    }
}

fn parse_request_id(value: String) -> Result<RequestId, ShellProtocolError> {
    RequestId::parse(value).map_err(|_: RequestError| ShellProtocolError::InvalidRequestId)
}

fn validate_digest(value: String) -> Result<String, ShellProtocolError> {
    if value.len() == DIGEST_HEX_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(value)
    } else {
        Err(ShellProtocolError::InvalidDigest)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ShellApprovalPreview {
    pub version: u16,
    pub request_id: String,
    pub operation: &'static str,
    pub purpose: &'static str,
    pub executable: &'static str,
    pub arguments: [&'static str; 1],
    pub capability: &'static str,
    pub resource_scope: &'static str,
    pub filesystem: &'static str,
    pub network: &'static str,
    pub privilege: &'static str,
    pub expected_side_effects: &'static str,
    pub approval: &'static str,
    pub expires_at_ms: u64,
    pub preview_sha256: String,
}

impl ShellApprovalPreview {
    pub fn system_uname(request_id: &RequestId, expires_at_ms: u64) -> Self {
        let mut preview = Self {
            version: SHELL_PROTOCOL_VERSION,
            request_id: request_id.as_str().into(),
            operation: "system.uname",
            purpose: "read the kernel operating-system name",
            executable: "/usr/bin/uname",
            arguments: ["-s"],
            capability: "system.read:kernel.identity",
            resource_scope: "kernel identity only",
            filesystem: "code-owned read-only runtime mounts",
            network: "denied",
            privilege: "unprivileged user",
            expected_side_effects: "none",
            approval: "once only",
            expires_at_ms,
            preview_sha256: String::new(),
        };
        preview.preview_sha256 = preview.compute_digest();
        preview
    }

    pub fn verify_digest(&self) -> bool {
        self.preview_sha256 == self.compute_digest()
    }

    fn compute_digest(&self) -> String {
        let mut hasher = Sha256::new();
        for field in [
            self.version.to_string(),
            self.request_id.clone(),
            self.operation.into(),
            self.purpose.into(),
            self.executable.into(),
            self.arguments[0].into(),
            self.capability.into(),
            self.resource_scope.into(),
            self.filesystem.into(),
            self.network.into(),
            self.privilege.into(),
            self.expected_side_effects.into(),
            self.approval.into(),
            self.expires_at_ms.to_string(),
        ] {
            hasher.update((field.len() as u64).to_be_bytes());
            hasher.update(field.as_bytes());
        }
        let hash = hasher.finalize();
        let mut encoded = String::with_capacity(hash.len() * 2);
        for byte in hash {
            write!(&mut encoded, "{byte:02x}").expect("writing to a String cannot fail");
        }
        encoded
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ShellActivityKind {
    Request,
    Policy,
    Approval,
    Execution,
    Verification,
    Terminal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ShellActivityCategory {
    Accepted,
    PolicyAsk,
    PolicyDenied,
    ApprovalIssued,
    ApprovedOnce,
    Denied,
    Cancelled,
    Expired,
    Started,
    ExecutionFailed,
    Verified,
    VerificationFailed,
    Indeterminate,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ShellActivityProjection {
    pub version: u16,
    pub sequence: u64,
    pub request_id: RequestId,
    pub kind: ShellActivityKind,
    pub category: ShellActivityCategory,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShellProtocolError {
    MessageTooLarge,
    MalformedMessage,
    UnsupportedVersion,
    InvalidRequestId,
    InvalidDigest,
    InvalidActivityLimit,
}

impl fmt::Display for ShellProtocolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::MessageTooLarge => "shell message exceeded the byte limit",
            Self::MalformedMessage => "shell message did not match the closed schema",
            Self::UnsupportedVersion => "shell protocol version is unsupported",
            Self::InvalidRequestId => "shell request identifier is invalid",
            Self::InvalidDigest => "shell preview digest is invalid",
            Self::InvalidActivityLimit => "shell activity limit is invalid",
        })
    }
}

impl std::error::Error for ShellProtocolError {}

#[cfg(test)]
mod tests {
    use super::*;

    const DIGEST: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    #[test]
    fn accepts_only_the_fixed_diagnostic_start() {
        assert_eq!(
            decode_shell_client_request(br#"{"kind":"start_system_uname","version":1}"#),
            Ok(ShellClientRequest::StartSystemUname)
        );
        for rejected in [
            br#"{"kind":"start_system_uptime","version":1}"#.as_slice(),
            br#"{"kind":"start_system_uname","version":1,"arguments":["-a"]}"#.as_slice(),
            br#"{"kind":"start_system_uname","version":1,"path":"/bin/sh"}"#.as_slice(),
        ] {
            assert_eq!(
                decode_shell_client_request(rejected),
                Err(ShellProtocolError::MalformedMessage)
            );
        }
    }

    #[test]
    fn rejects_version_size_and_malformed_messages() {
        assert_eq!(
            decode_shell_client_request(br#"{"kind":"start_system_uname","version":2}"#),
            Err(ShellProtocolError::UnsupportedVersion)
        );
        assert_eq!(
            decode_shell_client_request(&vec![b' '; MAX_SHELL_MESSAGE_BYTES + 1]),
            Err(ShellProtocolError::MessageTooLarge)
        );
        assert_eq!(
            decode_shell_client_request(b"not-json"),
            Err(ShellProtocolError::MalformedMessage)
        );
    }

    #[test]
    fn decision_is_closed_and_identifiers_are_bounded() {
        let accepted = format!(
            r#"{{"kind":"submit_decision","version":1,"request_id":"req-1","preview_sha256":"{DIGEST}","decision":"approve_once"}}"#
        );
        assert!(matches!(
            decode_shell_client_request(accepted.as_bytes()),
            Ok(ShellClientRequest::SubmitDecision {
                decision: ShellDecision::ApproveOnce,
                ..
            })
        ));

        for rejected in [
            accepted.replace("approve_once", "approve_always"),
            accepted.replace("req-1", "../req-1"),
            accepted.replace(DIGEST, "ABCDEF"),
            accepted.replace(DIGEST, &"A".repeat(DIGEST_HEX_BYTES)),
        ] {
            assert!(decode_shell_client_request(rejected.as_bytes()).is_err());
        }
    }

    #[test]
    fn cancellation_requires_exact_request_and_digest_shape() {
        let accepted = format!(
            r#"{{"kind":"cancel_pending","version":1,"request_id":"req-2","preview_sha256":"{DIGEST}"}}"#
        );
        assert!(matches!(
            decode_shell_client_request(accepted.as_bytes()),
            Ok(ShellClientRequest::CancelPending { .. })
        ));
        assert_eq!(
            decode_shell_client_request(accepted.replace(DIGEST, "00").as_bytes()),
            Err(ShellProtocolError::InvalidDigest)
        );
    }

    #[test]
    fn activity_reads_are_bounded() {
        for limit in [0, MAX_ACTIVITY_BATCH + 1] {
            let input = format!(
                r#"{{"kind":"read_activity","version":1,"after_sequence":null,"limit":{limit}}}"#
            );
            assert_eq!(
                decode_shell_client_request(input.as_bytes()),
                Err(ShellProtocolError::InvalidActivityLimit)
            );
        }
        let input = br#"{"kind":"read_activity","version":1,"after_sequence":41,"limit":16}"#;
        assert_eq!(
            decode_shell_client_request(input),
            Ok(ShellClientRequest::ReadActivity {
                after_sequence: Some(41),
                limit: 16,
            })
        );
    }

    #[test]
    fn preview_is_exact_and_mutation_changes_its_digest() {
        let request_id = RequestId::parse("req-shell-1".into()).expect("valid request id");
        let preview = ShellApprovalPreview::system_uname(&request_id, 31_000);
        assert_eq!(preview.executable, "/usr/bin/uname");
        assert_eq!(preview.arguments, ["-s"]);
        assert_eq!(preview.capability, "system.read:kernel.identity");
        assert_eq!(preview.network, "denied");
        assert!(preview.verify_digest());

        let mut mutated = preview.clone();
        mutated.expires_at_ms += 1;
        assert!(!mutated.verify_digest());
    }

    #[test]
    fn preview_serialization_contains_no_token_or_generic_authority() {
        let request_id = RequestId::parse("req-ui-2".into()).expect("valid request id");
        let encoded =
            serde_json::to_string(&ShellApprovalPreview::system_uname(&request_id, 31_000))
                .expect("preview should serialize");
        for forbidden in ["token", "shell", "sudo", "command_arguments", "tool_name"] {
            assert!(!encoded.contains(forbidden));
        }
    }
}
