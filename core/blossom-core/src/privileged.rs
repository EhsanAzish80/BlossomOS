use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;

pub const PRIVILEGED_PROTOCOL_VERSION: u16 = 1;
pub const PRIVILEGED_BUS_NAME: &str = "org.blossomos.Privileged1";
pub const PRIVILEGED_OBJECT_PATH: &str = "/org/blossomos/Privileged1";
pub const PRIVILEGED_INTERFACE: &str = "org.blossomos.Privileged1";
pub const BLUETOOTH_METHOD: &str = "TryRestartBluetooth1";
pub const BLUETOOTH_UNIT: &str = "bluetooth.service";
pub const SYSTEMD_JOB_MODE: &str = "replace";
pub const BLUETOOTH_POLKIT_ACTION: &str = "org.blossomos.privileged1.try-restart-bluetooth";
pub const MAX_CORRELATION_ID_BYTES: usize = 64;
pub const IDEMPOTENCY_KEY_HEX_BYTES: usize = 32;
pub const MAX_STATE_BYTES: usize = 64;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BluetoothRestartRequest {
    pub version: u16,
    pub correlation_id: String,
    pub idempotency_key: String,
    pub interactive: bool,
}

impl BluetoothRestartRequest {
    pub fn validate(&self) -> Result<(), PrivilegedProtocolError> {
        if self.version != PRIVILEGED_PROTOCOL_VERSION {
            return Err(PrivilegedProtocolError::UnsupportedVersion);
        }
        if !valid_identifier(&self.correlation_id, MAX_CORRELATION_ID_BYTES) {
            return Err(PrivilegedProtocolError::InvalidCorrelationId);
        }
        if self.idempotency_key.len() != IDEMPOTENCY_KEY_HEX_BYTES
            || !self
                .idempotency_key
                .bytes()
                .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
        {
            return Err(PrivilegedProtocolError::InvalidIdempotencyKey);
        }
        Ok(())
    }

    pub fn normalized_digest(
        &self,
        authenticated_uid: u32,
    ) -> Result<String, PrivilegedProtocolError> {
        self.validate()?;
        let normalized = NormalizedBluetoothRestart {
            version: self.version,
            method: BLUETOOTH_METHOD,
            unit: BLUETOOTH_UNIT,
            job_mode: SYSTEMD_JOB_MODE,
            polkit_action: BLUETOOTH_POLKIT_ACTION,
            authenticated_uid,
            correlation_id: &self.correlation_id,
            idempotency_key: &self.idempotency_key,
            interactive: self.interactive,
        };
        let bytes = serde_json::to_vec(&normalized)
            .map_err(|_| PrivilegedProtocolError::ProtocolViolation)?;
        Ok(hex_digest(&bytes))
    }
}

#[derive(Serialize)]
struct NormalizedBluetoothRestart<'a> {
    version: u16,
    method: &'static str,
    unit: &'static str,
    job_mode: &'static str,
    polkit_action: &'static str,
    authenticated_uid: u32,
    correlation_id: &'a str,
    idempotency_key: &'a str,
    interactive: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BluetoothObservation {
    pub canonical_unit: String,
    pub load_state: String,
    pub active_state: String,
    pub invocation_id: [u8; 16],
}

impl BluetoothObservation {
    pub fn validate(&self) -> Result<(), PrivilegedProtocolError> {
        if self.canonical_unit != BLUETOOTH_UNIT
            || self.load_state != "loaded"
            || !valid_state(&self.active_state)
        {
            return Err(PrivilegedProtocolError::ProtocolViolation);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BluetoothRestartFailure {
    Denied,
    Cancelled,
    Expired,
    UnitUnavailable,
    JobFailed,
    VerificationFailed,
    OutcomeIndeterminate,
    AuthorizationUnavailable,
    JournalUnavailable,
    InterruptedBeforeSubmission,
    ProtocolViolation,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum BluetoothRestartOutcome {
    RestartedActive {
        before: BluetoothObservation,
        after: BluetoothObservation,
        job_result: String,
    },
    NotRunning {
        observation: BluetoothObservation,
    },
    Failed {
        error: BluetoothRestartFailure,
        job_submitted: bool,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BluetoothRestartResult {
    pub version: u16,
    pub correlation_id: String,
    pub authenticated_uid: u32,
    pub request_sha256: String,
    pub replayed: bool,
    pub outcome: BluetoothRestartOutcome,
}

pub fn verify_bluetooth_restart_result(
    request: &BluetoothRestartRequest,
    result: &BluetoothRestartResult,
) -> Result<(), PrivilegedProtocolError> {
    request.validate()?;
    if result.version != PRIVILEGED_PROTOCOL_VERSION
        || result.correlation_id != request.correlation_id
        || result.request_sha256 != request.normalized_digest(result.authenticated_uid)?
    {
        return Err(PrivilegedProtocolError::ProtocolViolation);
    }
    match &result.outcome {
        BluetoothRestartOutcome::RestartedActive {
            before,
            after,
            job_result,
        } => {
            before.validate()?;
            after.validate()?;
            if before.active_state != "active"
                || after.active_state != "active"
                || before.invocation_id.iter().all(|byte| *byte == 0)
                || after.invocation_id.iter().all(|byte| *byte == 0)
                || before.invocation_id == after.invocation_id
                || job_result != "done"
            {
                return Err(PrivilegedProtocolError::VerificationFailed);
            }
        }
        BluetoothRestartOutcome::NotRunning { observation } => {
            observation.validate()?;
            if observation.active_state == "active" {
                return Err(PrivilegedProtocolError::VerificationFailed);
            }
        }
        BluetoothRestartOutcome::Failed {
            error,
            job_submitted,
        } => {
            if matches!(
                error,
                BluetoothRestartFailure::Denied
                    | BluetoothRestartFailure::Cancelled
                    | BluetoothRestartFailure::Expired
                    | BluetoothRestartFailure::UnitUnavailable
                    | BluetoothRestartFailure::AuthorizationUnavailable
                    | BluetoothRestartFailure::JournalUnavailable
                    | BluetoothRestartFailure::InterruptedBeforeSubmission
                    | BluetoothRestartFailure::ProtocolViolation
            ) && *job_submitted
            {
                return Err(PrivilegedProtocolError::VerificationFailed);
            }
            if matches!(
                error,
                BluetoothRestartFailure::JobFailed
                    | BluetoothRestartFailure::VerificationFailed
                    | BluetoothRestartFailure::OutcomeIndeterminate
            ) && !*job_submitted
            {
                return Err(PrivilegedProtocolError::VerificationFailed);
            }
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrivilegedProtocolError {
    UnsupportedVersion,
    InvalidCorrelationId,
    InvalidIdempotencyKey,
    ProtocolViolation,
    VerificationFailed,
}

impl fmt::Display for PrivilegedProtocolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::UnsupportedVersion => "unsupported privileged protocol version",
            Self::InvalidCorrelationId => "invalid privileged correlation identifier",
            Self::InvalidIdempotencyKey => "invalid privileged idempotency key",
            Self::ProtocolViolation => "privileged protocol invariant failed",
            Self::VerificationFailed => "privileged result verification failed",
        })
    }
}

impl std::error::Error for PrivilegedProtocolError {}

fn valid_identifier(value: &str, limit: usize) -> bool {
    !value.is_empty()
        && value.len() <= limit
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn valid_state(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_STATE_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn hex_digest(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write;
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> BluetoothRestartRequest {
        BluetoothRestartRequest {
            version: PRIVILEGED_PROTOCOL_VERSION,
            correlation_id: "privileged-1".into(),
            idempotency_key: "0".repeat(IDEMPOTENCY_KEY_HEX_BYTES),
            interactive: true,
        }
    }

    fn observation(invocation: u8, active: &str) -> BluetoothObservation {
        BluetoothObservation {
            canonical_unit: BLUETOOTH_UNIT.into(),
            load_state: "loaded".into(),
            active_state: active.into(),
            invocation_id: [invocation; 16],
        }
    }

    #[test]
    fn request_schema_is_closed_bounded_and_versioned() {
        assert_eq!(request().validate(), Ok(()));
        let json = serde_json::to_string(&request()).unwrap();
        assert!(
            serde_json::from_str::<BluetoothRestartRequest>(&json.replace(
                "\"interactive\":true",
                "\"interactive\":true,\"unit\":\"ssh.service\""
            ))
            .is_err()
        );
        let mut changed = request();
        changed.version += 1;
        assert_eq!(
            changed.validate(),
            Err(PrivilegedProtocolError::UnsupportedVersion)
        );
        changed = request();
        changed.idempotency_key = "A".repeat(IDEMPOTENCY_KEY_HEX_BYTES);
        assert_eq!(
            changed.validate(),
            Err(PrivilegedProtocolError::InvalidIdempotencyKey)
        );
    }

    #[test]
    fn wire_schema_fixtures_are_byte_for_byte_stable() {
        let request_json = serde_json::to_string(&request()).unwrap();
        assert_eq!(
            request_json,
            r#"{"version":1,"correlation_id":"privileged-1","idempotency_key":"00000000000000000000000000000000","interactive":true}"#
        );

        let failed = BluetoothRestartResult {
            version: PRIVILEGED_PROTOCOL_VERSION,
            correlation_id: "privileged-1".into(),
            authenticated_uid: 1000,
            request_sha256: "1".repeat(64),
            replayed: false,
            outcome: BluetoothRestartOutcome::Failed {
                error: BluetoothRestartFailure::Denied,
                job_submitted: false,
            },
        };
        assert_eq!(
            serde_json::to_string(&failed).unwrap(),
            r#"{"version":1,"correlation_id":"privileged-1","authenticated_uid":1000,"request_sha256":"1111111111111111111111111111111111111111111111111111111111111111","replayed":false,"outcome":{"status":"failed","error":"denied","job_submitted":false}}"#
        );
    }

    #[test]
    fn digest_binds_uid_ids_interactivity_and_every_fixed_operation_constant() {
        let original = request();
        let digest = original.normalized_digest(1000).unwrap();
        assert_eq!(digest.len(), 64);
        assert_ne!(digest, original.normalized_digest(1001).unwrap());
        let mut changed = original.clone();
        changed.interactive = false;
        assert_ne!(digest, changed.normalized_digest(1000).unwrap());
        changed = original;
        changed.correlation_id = "privileged-2".into();
        assert_ne!(digest, changed.normalized_digest(1000).unwrap());
    }

    #[test]
    fn verifies_only_a_completed_new_active_invocation() {
        let request = request();
        let result = BluetoothRestartResult {
            version: PRIVILEGED_PROTOCOL_VERSION,
            correlation_id: request.correlation_id.clone(),
            authenticated_uid: 1000,
            request_sha256: request.normalized_digest(1000).unwrap(),
            replayed: false,
            outcome: BluetoothRestartOutcome::RestartedActive {
                before: observation(1, "active"),
                after: observation(2, "active"),
                job_result: "done".into(),
            },
        };
        assert_eq!(verify_bluetooth_restart_result(&request, &result), Ok(()));
        let mut unchanged = result.clone();
        let BluetoothRestartOutcome::RestartedActive { after, .. } = &mut unchanged.outcome else {
            unreachable!()
        };
        after.invocation_id = [1; 16];
        assert_eq!(
            verify_bluetooth_restart_result(&request, &unchanged),
            Err(PrivilegedProtocolError::VerificationFailed)
        );
    }

    #[test]
    fn pre_authorization_failures_cannot_claim_a_submitted_job() {
        let request = request();
        let result = BluetoothRestartResult {
            version: PRIVILEGED_PROTOCOL_VERSION,
            correlation_id: request.correlation_id.clone(),
            authenticated_uid: 1000,
            request_sha256: request.normalized_digest(1000).unwrap(),
            replayed: false,
            outcome: BluetoothRestartOutcome::Failed {
                error: BluetoothRestartFailure::Denied,
                job_submitted: true,
            },
        };
        assert_eq!(
            verify_bluetooth_restart_result(&request, &result),
            Err(PrivilegedProtocolError::VerificationFailed)
        );
    }
}
