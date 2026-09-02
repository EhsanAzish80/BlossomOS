use blossom_core::privileged::{
    BluetoothObservation, BluetoothRestartFailure, BluetoothRestartOutcome,
    BluetoothRestartRequest, BluetoothRestartResult, PRIVILEGED_PROTOCOL_VERSION,
    verify_bluetooth_restart_result,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

#[cfg(unix)]
mod file_journal;
#[cfg(unix)]
pub use file_journal::FileJournal;
#[cfg(unix)]
mod file_audit;
#[cfg(unix)]
pub use file_audit::FileAudit;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthorizationDecision {
    Authorized,
    Denied,
    Cancelled,
    Expired,
    Unavailable,
}

pub trait Authorizer {
    fn authorize(
        &mut self,
        caller_uid: u32,
        request: &BluetoothRestartRequest,
    ) -> AuthorizationDecision;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ManagerError {
    UnitUnavailable,
    Rejected,
    Failed,
    Timeout,
    Disconnected,
    ProtocolViolation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RestartCompletion {
    pub job_result: String,
    pub after: BluetoothObservation,
}

pub trait BluetoothManager {
    fn observe(&mut self) -> Result<BluetoothObservation, ManagerError>;
    fn try_restart(&mut self) -> Result<RestartCompletion, ManagerError>;
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JournalKey {
    pub caller_uid: u32,
    pub idempotency_key: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "phase", rename_all = "snake_case", deny_unknown_fields)]
pub enum JournalState {
    Claimed {
        request_sha256: String,
    },
    Submitted {
        request_sha256: String,
    },
    Completed {
        request_sha256: String,
        result: Box<BluetoothRestartResult>,
    },
}

impl JournalState {
    fn digest(&self) -> &str {
        match self {
            Self::Claimed { request_sha256 }
            | Self::Submitted { request_sha256 }
            | Self::Completed { request_sha256, .. } => request_sha256,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClaimOutcome {
    New,
    Existing(JournalState),
    DigestMismatch,
}

pub trait IdempotencyJournal {
    fn claim(&mut self, key: &JournalKey, digest: &str) -> Result<ClaimOutcome, JournalError>;
    fn mark_submitted(&mut self, key: &JournalKey, digest: &str) -> Result<(), JournalError>;
    fn complete(
        &mut self,
        key: &JournalKey,
        digest: &str,
        result: &BluetoothRestartResult,
    ) -> Result<(), JournalError>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JournalError {
    Unavailable,
    InvalidTransition,
}

#[derive(Clone, Debug, Default)]
pub struct MemoryJournal {
    entries: HashMap<JournalKey, JournalState>,
}

impl IdempotencyJournal for MemoryJournal {
    fn claim(&mut self, key: &JournalKey, digest: &str) -> Result<ClaimOutcome, JournalError> {
        match self.entries.get(key) {
            None => {
                self.entries.insert(
                    key.clone(),
                    JournalState::Claimed {
                        request_sha256: digest.into(),
                    },
                );
                Ok(ClaimOutcome::New)
            }
            Some(state) if state.digest() == digest => Ok(ClaimOutcome::Existing(state.clone())),
            Some(_) => Ok(ClaimOutcome::DigestMismatch),
        }
    }

    fn mark_submitted(&mut self, key: &JournalKey, digest: &str) -> Result<(), JournalError> {
        match self.entries.get(key) {
            Some(JournalState::Claimed { request_sha256 }) if request_sha256 == digest => {
                self.entries.insert(
                    key.clone(),
                    JournalState::Submitted {
                        request_sha256: digest.into(),
                    },
                );
                Ok(())
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
        match self.entries.get(key) {
            Some(state)
                if state.digest() == digest && !matches!(state, JournalState::Completed { .. }) =>
            {
                self.entries.insert(
                    key.clone(),
                    JournalState::Completed {
                        request_sha256: digest.into(),
                        result: Box::new(result.clone()),
                    },
                );
                Ok(())
            }
            _ => Err(JournalError::InvalidTransition),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum HelperAuditEvent {
    RequestReceived {
        correlation_id: String,
        caller_uid: u32,
    },
    AuthorizationFinished {
        correlation_id: String,
        decision: AuthorizationDecision,
    },
    JournalClaimed {
        correlation_id: String,
        request_sha256: String,
    },
    ReplayReturned {
        correlation_id: String,
        phase: String,
    },
    PreStateObserved {
        correlation_id: String,
        active: bool,
    },
    JobMarkedSubmitted {
        correlation_id: String,
    },
    JobFinished {
        correlation_id: String,
        category: String,
    },
    VerificationFinished {
        correlation_id: String,
        verified: bool,
    },
    RequestFinished {
        correlation_id: String,
        category: String,
    },
}

pub trait HelperAudit {
    fn record(&mut self, event: HelperAuditEvent) -> Result<(), AuditError>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuditError {
    Unavailable,
}

#[derive(Clone, Debug, Default)]
pub struct MemoryAudit {
    pub events: Vec<HelperAuditEvent>,
}

impl HelperAudit for MemoryAudit {
    fn record(&mut self, event: HelperAuditEvent) -> Result<(), AuditError> {
        self.events.push(event);
        Ok(())
    }
}

pub struct PrivilegedHelper<A, M, J, L> {
    authorizer: A,
    manager: M,
    journal: J,
    audit: L,
}

impl<A: Authorizer, M: BluetoothManager, J: IdempotencyJournal, L: HelperAudit>
    PrivilegedHelper<A, M, J, L>
{
    pub fn new(authorizer: A, manager: M, journal: J, audit: L) -> Self {
        Self {
            authorizer,
            manager,
            journal,
            audit,
        }
    }

    pub fn handle(
        &mut self,
        caller_uid: u32,
        request: BluetoothRestartRequest,
    ) -> BluetoothRestartResult {
        let correlation = request.correlation_id.clone();
        let fallback_digest = request
            .normalized_digest(caller_uid)
            .unwrap_or_else(|_| zero_digest());
        if self
            .audit
            .record(HelperAuditEvent::RequestReceived {
                correlation_id: correlation.clone(),
                caller_uid,
            })
            .is_err()
        {
            return failure(
                &request,
                caller_uid,
                &fallback_digest,
                BluetoothRestartFailure::JournalUnavailable,
                false,
            );
        }
        if request.validate().is_err() {
            return self.finish_failure(
                &request,
                caller_uid,
                &fallback_digest,
                BluetoothRestartFailure::ProtocolViolation,
                false,
            );
        }
        let digest = request
            .normalized_digest(caller_uid)
            .expect("validated request digest");
        let decision = self.authorizer.authorize(caller_uid, &request);
        if self
            .audit
            .record(HelperAuditEvent::AuthorizationFinished {
                correlation_id: correlation.clone(),
                decision,
            })
            .is_err()
        {
            return failure(
                &request,
                caller_uid,
                &digest,
                BluetoothRestartFailure::JournalUnavailable,
                false,
            );
        }
        if decision != AuthorizationDecision::Authorized {
            let error = match decision {
                AuthorizationDecision::Denied => BluetoothRestartFailure::Denied,
                AuthorizationDecision::Cancelled => BluetoothRestartFailure::Cancelled,
                AuthorizationDecision::Expired => BluetoothRestartFailure::Expired,
                AuthorizationDecision::Unavailable => {
                    BluetoothRestartFailure::AuthorizationUnavailable
                }
                AuthorizationDecision::Authorized => unreachable!(),
            };
            return self.finish_failure(&request, caller_uid, &digest, error, false);
        }
        let key = JournalKey {
            caller_uid,
            idempotency_key: request.idempotency_key.clone(),
        };
        match self.journal.claim(&key, &digest) {
            Ok(ClaimOutcome::Existing(state)) => {
                return self.replay(&request, caller_uid, &digest, state);
            }
            Ok(ClaimOutcome::DigestMismatch) => {
                return self.finish_failure(
                    &request,
                    caller_uid,
                    &digest,
                    BluetoothRestartFailure::ProtocolViolation,
                    false,
                );
            }
            Err(_) => {
                return self.finish_failure(
                    &request,
                    caller_uid,
                    &digest,
                    BluetoothRestartFailure::JournalUnavailable,
                    false,
                );
            }
            Ok(ClaimOutcome::New) => {}
        }
        if self
            .audit
            .record(HelperAuditEvent::JournalClaimed {
                correlation_id: correlation.clone(),
                request_sha256: digest.clone(),
            })
            .is_err()
        {
            return failure(
                &request,
                caller_uid,
                &digest,
                BluetoothRestartFailure::JournalUnavailable,
                false,
            );
        }
        let before = match self.manager.observe() {
            Ok(value) if value.validate().is_ok() => value,
            Err(ManagerError::UnitUnavailable) => {
                return self.complete_failure(
                    &request,
                    caller_uid,
                    &key,
                    &digest,
                    BluetoothRestartFailure::UnitUnavailable,
                    false,
                );
            }
            _ => {
                return self.complete_failure(
                    &request,
                    caller_uid,
                    &key,
                    &digest,
                    BluetoothRestartFailure::ProtocolViolation,
                    false,
                );
            }
        };
        let active = before.active_state == "active";
        if self
            .audit
            .record(HelperAuditEvent::PreStateObserved {
                correlation_id: correlation.clone(),
                active,
            })
            .is_err()
        {
            return self.complete_failure(
                &request,
                caller_uid,
                &key,
                &digest,
                BluetoothRestartFailure::JournalUnavailable,
                false,
            );
        }
        if !active {
            let result = result(
                &request,
                caller_uid,
                &digest,
                BluetoothRestartOutcome::NotRunning {
                    observation: before,
                },
            );
            return self.complete_result(&key, &digest, result);
        }
        if self.journal.mark_submitted(&key, &digest).is_err() {
            return self.complete_failure(
                &request,
                caller_uid,
                &key,
                &digest,
                BluetoothRestartFailure::JournalUnavailable,
                false,
            );
        }
        if self
            .audit
            .record(HelperAuditEvent::JobMarkedSubmitted {
                correlation_id: correlation.clone(),
            })
            .is_err()
        {
            return self.complete_failure(
                &request,
                caller_uid,
                &key,
                &digest,
                BluetoothRestartFailure::OutcomeIndeterminate,
                true,
            );
        }
        let completion = match self.manager.try_restart() {
            Ok(value) => value,
            Err(ManagerError::Rejected | ManagerError::Failed) => {
                return self.complete_failure(
                    &request,
                    caller_uid,
                    &key,
                    &digest,
                    BluetoothRestartFailure::JobFailed,
                    true,
                );
            }
            Err(_) => {
                return self.complete_failure(
                    &request,
                    caller_uid,
                    &key,
                    &digest,
                    BluetoothRestartFailure::OutcomeIndeterminate,
                    true,
                );
            }
        };
        if self
            .audit
            .record(HelperAuditEvent::JobFinished {
                correlation_id: correlation.clone(),
                category: bounded_category(&completion.job_result),
            })
            .is_err()
        {
            return self.complete_failure(
                &request,
                caller_uid,
                &key,
                &digest,
                BluetoothRestartFailure::OutcomeIndeterminate,
                true,
            );
        }
        let candidate = result(
            &request,
            caller_uid,
            &digest,
            BluetoothRestartOutcome::RestartedActive {
                before,
                after: completion.after,
                job_result: completion.job_result,
            },
        );
        let verified = verify_bluetooth_restart_result(&request, &candidate).is_ok();
        if self
            .audit
            .record(HelperAuditEvent::VerificationFinished {
                correlation_id: correlation,
                verified,
            })
            .is_err()
        {
            return self.complete_failure(
                &request,
                caller_uid,
                &key,
                &digest,
                BluetoothRestartFailure::JournalUnavailable,
                true,
            );
        }
        if verified {
            self.complete_result(&key, &digest, candidate)
        } else {
            self.complete_failure(
                &request,
                caller_uid,
                &key,
                &digest,
                BluetoothRestartFailure::VerificationFailed,
                true,
            )
        }
    }

    fn replay(
        &mut self,
        request: &BluetoothRestartRequest,
        uid: u32,
        digest: &str,
        state: JournalState,
    ) -> BluetoothRestartResult {
        let phase = match &state {
            JournalState::Claimed { .. } => "claimed",
            JournalState::Submitted { .. } => "submitted",
            JournalState::Completed { .. } => "completed",
        };
        if self
            .audit
            .record(HelperAuditEvent::ReplayReturned {
                correlation_id: request.correlation_id.clone(),
                phase: phase.into(),
            })
            .is_err()
        {
            let prior = match &state {
                JournalState::Completed { result, .. } => (**result).clone(),
                JournalState::Claimed { .. } => failure(
                    request,
                    uid,
                    digest,
                    BluetoothRestartFailure::InterruptedBeforeSubmission,
                    false,
                ),
                JournalState::Submitted { .. } => failure(
                    request,
                    uid,
                    digest,
                    BluetoothRestartFailure::OutcomeIndeterminate,
                    true,
                ),
            };
            return journal_failure_from_result(&prior);
        }
        match state {
            JournalState::Completed { result, .. } => {
                let mut result = *result;
                result.replayed = true;
                result
            }
            JournalState::Claimed { .. } => failure(
                request,
                uid,
                digest,
                BluetoothRestartFailure::InterruptedBeforeSubmission,
                false,
            ),
            JournalState::Submitted { .. } => failure(
                request,
                uid,
                digest,
                BluetoothRestartFailure::OutcomeIndeterminate,
                true,
            ),
        }
    }

    fn finish_failure(
        &mut self,
        request: &BluetoothRestartRequest,
        uid: u32,
        digest: &str,
        error: BluetoothRestartFailure,
        submitted: bool,
    ) -> BluetoothRestartResult {
        let result = failure(request, uid, digest, error, submitted);
        if self
            .audit
            .record(HelperAuditEvent::RequestFinished {
                correlation_id: request.correlation_id.clone(),
                category: "failed".into(),
            })
            .is_err()
        {
            journal_failure_from_result(&result)
        } else {
            result
        }
    }

    fn complete_failure(
        &mut self,
        request: &BluetoothRestartRequest,
        uid: u32,
        key: &JournalKey,
        digest: &str,
        error: BluetoothRestartFailure,
        submitted: bool,
    ) -> BluetoothRestartResult {
        let result = failure(request, uid, digest, error, submitted);
        self.complete_result(key, digest, result)
    }

    fn complete_result(
        &mut self,
        key: &JournalKey,
        digest: &str,
        result: BluetoothRestartResult,
    ) -> BluetoothRestartResult {
        if self.journal.complete(key, digest, &result).is_err() {
            return journal_failure_from_result(&result);
        }
        let category = match result.outcome {
            BluetoothRestartOutcome::RestartedActive { .. } => "restarted_active",
            BluetoothRestartOutcome::NotRunning { .. } => "not_running",
            BluetoothRestartOutcome::Failed { .. } => "failed",
        };
        if self
            .audit
            .record(HelperAuditEvent::RequestFinished {
                correlation_id: result.correlation_id.clone(),
                category: category.into(),
            })
            .is_err()
        {
            return journal_failure_from_result(&result);
        }
        result
    }

    pub fn into_parts(self) -> (A, M, J, L) {
        (self.authorizer, self.manager, self.journal, self.audit)
    }
}

fn result(
    request: &BluetoothRestartRequest,
    uid: u32,
    digest: &str,
    outcome: BluetoothRestartOutcome,
) -> BluetoothRestartResult {
    BluetoothRestartResult {
        version: PRIVILEGED_PROTOCOL_VERSION,
        correlation_id: request.correlation_id.clone(),
        authenticated_uid: uid,
        request_sha256: digest.into(),
        replayed: false,
        outcome,
    }
}

fn failure(
    request: &BluetoothRestartRequest,
    uid: u32,
    digest: &str,
    error: BluetoothRestartFailure,
    job_submitted: bool,
) -> BluetoothRestartResult {
    result(
        request,
        uid,
        digest,
        BluetoothRestartOutcome::Failed {
            error,
            job_submitted,
        },
    )
}

fn journal_failure_from_result(result: &BluetoothRestartResult) -> BluetoothRestartResult {
    let job_submitted = match &result.outcome {
        BluetoothRestartOutcome::RestartedActive { .. } => true,
        BluetoothRestartOutcome::NotRunning { .. } => false,
        BluetoothRestartOutcome::Failed { job_submitted, .. } => *job_submitted,
    };
    let error = if job_submitted {
        BluetoothRestartFailure::OutcomeIndeterminate
    } else {
        BluetoothRestartFailure::JournalUnavailable
    };
    BluetoothRestartResult {
        outcome: BluetoothRestartOutcome::Failed {
            error,
            job_submitted,
        },
        ..result.clone()
    }
}

fn bounded_category(value: &str) -> String {
    if !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
    {
        value.into()
    } else {
        "invalid".into()
    }
}

fn zero_digest() -> String {
    "0".repeat(64)
}

pub fn audit_chain_hash(
    previous: &str,
    event: &HelperAuditEvent,
) -> Result<String, serde_json::Error> {
    let bytes = serde_json::to_vec(&(previous, event))?;
    let digest = Sha256::digest(bytes);
    Ok(digest.iter().map(|byte| format!("{byte:02x}")).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use blossom_core::privileged::BLUETOOTH_UNIT;

    struct Allow;
    impl Authorizer for Allow {
        fn authorize(&mut self, _: u32, _: &BluetoothRestartRequest) -> AuthorizationDecision {
            AuthorizationDecision::Authorized
        }
    }
    struct Deny;
    impl Authorizer for Deny {
        fn authorize(&mut self, _: u32, _: &BluetoothRestartRequest) -> AuthorizationDecision {
            AuthorizationDecision::Denied
        }
    }

    struct FailAuditAt {
        call: usize,
        fail_at: usize,
    }

    impl HelperAudit for FailAuditAt {
        fn record(&mut self, _: HelperAuditEvent) -> Result<(), AuditError> {
            self.call += 1;
            if self.call == self.fail_at {
                Err(AuditError::Unavailable)
            } else {
                Ok(())
            }
        }
    }

    #[derive(Clone)]
    struct Manager {
        calls: usize,
        before: Result<BluetoothObservation, ManagerError>,
        restart: Result<RestartCompletion, ManagerError>,
    }
    impl BluetoothManager for Manager {
        fn observe(&mut self) -> Result<BluetoothObservation, ManagerError> {
            self.before.clone()
        }
        fn try_restart(&mut self) -> Result<RestartCompletion, ManagerError> {
            self.calls += 1;
            self.restart.clone()
        }
    }

    fn request() -> BluetoothRestartRequest {
        BluetoothRestartRequest {
            version: 1,
            correlation_id: "request-1".into(),
            idempotency_key: "0".repeat(32),
            interactive: true,
        }
    }
    fn observation(id: u8, state: &str) -> BluetoothObservation {
        BluetoothObservation {
            canonical_unit: BLUETOOTH_UNIT.into(),
            load_state: "loaded".into(),
            active_state: state.into(),
            invocation_id: [id; 16],
        }
    }
    fn manager() -> Manager {
        Manager {
            calls: 0,
            before: Ok(observation(1, "active")),
            restart: Ok(RestartCompletion {
                job_result: "done".into(),
                after: observation(2, "active"),
            }),
        }
    }

    #[test]
    fn authorized_request_executes_once_verifies_and_audits() {
        let mut helper = PrivilegedHelper::new(
            Allow,
            manager(),
            MemoryJournal::default(),
            MemoryAudit::default(),
        );
        let first = helper.handle(1000, request());
        assert!(matches!(
            first.outcome,
            BluetoothRestartOutcome::RestartedActive { .. }
        ));
        let replay = helper.handle(1000, request());
        assert!(replay.replayed);
        let (_, manager, _, audit) = helper.into_parts();
        assert_eq!(manager.calls, 1);
        assert!(audit.events.iter().any(|event| matches!(
            event,
            HelperAuditEvent::VerificationFinished { verified: true, .. }
        )));
    }

    #[test]
    fn denial_and_inactive_state_submit_no_job() {
        let mut denied = PrivilegedHelper::new(
            Deny,
            manager(),
            MemoryJournal::default(),
            MemoryAudit::default(),
        );
        assert!(matches!(
            denied.handle(1000, request()).outcome,
            BluetoothRestartOutcome::Failed {
                error: BluetoothRestartFailure::Denied,
                job_submitted: false
            }
        ));
        assert_eq!(denied.into_parts().1.calls, 0);
        let mut inactive_manager = manager();
        inactive_manager.before = Ok(observation(0, "inactive"));
        let mut inactive = PrivilegedHelper::new(
            Allow,
            inactive_manager,
            MemoryJournal::default(),
            MemoryAudit::default(),
        );
        assert!(matches!(
            inactive.handle(1000, request()).outcome,
            BluetoothRestartOutcome::NotRunning { .. }
        ));
        assert_eq!(inactive.into_parts().1.calls, 0);
    }

    #[test]
    fn submitted_failure_is_truthful_and_never_retried() {
        let mut failing = manager();
        failing.restart = Err(ManagerError::Timeout);
        let mut helper = PrivilegedHelper::new(
            Allow,
            failing,
            MemoryJournal::default(),
            MemoryAudit::default(),
        );
        let first = helper.handle(1000, request());
        assert!(matches!(
            first.outcome,
            BluetoothRestartOutcome::Failed {
                error: BluetoothRestartFailure::OutcomeIndeterminate,
                job_submitted: true
            }
        ));
        let replay = helper.handle(1000, request());
        assert!(replay.replayed);
        assert_eq!(helper.into_parts().1.calls, 1);
    }

    #[test]
    fn key_reuse_with_changed_digest_is_rejected() {
        let journal = MemoryJournal::default();
        let mut helper = PrivilegedHelper::new(Allow, manager(), journal, MemoryAudit::default());
        helper.handle(1000, request());
        let mut changed = request();
        changed.correlation_id = "request-2".into();
        assert!(matches!(
            helper.handle(1000, changed).outcome,
            BluetoothRestartOutcome::Failed {
                error: BluetoothRestartFailure::ProtocolViolation,
                job_submitted: false
            }
        ));
        assert_eq!(helper.into_parts().1.calls, 1);
    }

    #[test]
    fn recovered_claim_and_submission_never_execute_again() {
        let key = JournalKey {
            caller_uid: 1000,
            idempotency_key: request().idempotency_key.clone(),
        };
        let digest = request().normalized_digest(1000).unwrap();
        let mut claimed = MemoryJournal::default();
        claimed.claim(&key, &digest).unwrap();
        let mut helper = PrivilegedHelper::new(Allow, manager(), claimed, MemoryAudit::default());
        assert!(matches!(
            helper.handle(1000, request()).outcome,
            BluetoothRestartOutcome::Failed {
                error: BluetoothRestartFailure::InterruptedBeforeSubmission,
                job_submitted: false
            }
        ));
        assert_eq!(helper.into_parts().1.calls, 0);
        let mut submitted = MemoryJournal::default();
        submitted.claim(&key, &digest).unwrap();
        submitted.mark_submitted(&key, &digest).unwrap();
        let mut helper = PrivilegedHelper::new(Allow, manager(), submitted, MemoryAudit::default());
        assert!(matches!(
            helper.handle(1000, request()).outcome,
            BluetoothRestartOutcome::Failed {
                error: BluetoothRestartFailure::OutcomeIndeterminate,
                job_submitted: true
            }
        ));
        assert_eq!(helper.into_parts().1.calls, 0);
    }

    #[test]
    fn audit_failure_after_submission_is_indeterminate_and_starts_nothing() {
        let audit = FailAuditAt {
            call: 0,
            // request, authorization, claim, observation, submitted
            fail_at: 5,
        };
        let mut helper = PrivilegedHelper::new(Allow, manager(), MemoryJournal::default(), audit);
        let result = helper.handle(1000, request());
        assert!(matches!(
            result.outcome,
            BluetoothRestartOutcome::Failed {
                error: BluetoothRestartFailure::OutcomeIndeterminate,
                job_submitted: true
            }
        ));
        assert_eq!(helper.into_parts().1.calls, 0);
    }

    #[test]
    fn audit_chain_is_deterministic_and_binds_previous_hash() {
        let event = HelperAuditEvent::JobMarkedSubmitted {
            correlation_id: "request-1".into(),
        };
        let first = audit_chain_hash(&zero_digest(), &event).unwrap();
        assert_eq!(first, audit_chain_hash(&zero_digest(), &event).unwrap());
        assert_ne!(first, audit_chain_hash(&"1".repeat(64), &event).unwrap());
    }

    #[test]
    fn post_operation_audit_failure_cannot_report_success() {
        let audit = FailAuditAt {
            call: 0,
            // request, authorization, claim, observation, submitted, job result
            fail_at: 6,
        };
        let mut helper = PrivilegedHelper::new(Allow, manager(), MemoryJournal::default(), audit);
        let result = helper.handle(1000, request());
        assert!(matches!(
            result.outcome,
            BluetoothRestartOutcome::Failed {
                error: BluetoothRestartFailure::OutcomeIndeterminate,
                job_submitted: true
            }
        ));
        assert_eq!(helper.into_parts().1.calls, 1);
    }

    #[test]
    fn terminal_audit_failure_replaces_a_pre_submission_denial() {
        let audit = FailAuditAt {
            call: 0,
            // request, authorization, terminal result
            fail_at: 3,
        };
        let mut helper = PrivilegedHelper::new(Deny, manager(), MemoryJournal::default(), audit);
        assert!(matches!(
            helper.handle(1000, request()).outcome,
            BluetoothRestartOutcome::Failed {
                error: BluetoothRestartFailure::JournalUnavailable,
                job_submitted: false
            }
        ));
        assert_eq!(helper.into_parts().1.calls, 0);
    }
}
