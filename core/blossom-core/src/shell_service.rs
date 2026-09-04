use crate::{
    ApprovalStore, BeginOutcome, BlossomEngine, EngineError, Executor, PolicyDecision,
    PolicyEngine, PolicyRule, RequestId, ShellApprovalPreview, ShellClientRequest, ShellDecision,
    ShellPeerId, ShellSessionApprovals, ShellSessionError, ToolRequest,
};
use crate::{Capability, CompletionOutcome};
use std::fmt;

pub const SHELL_APPROVAL_TTL_MS: u64 = 30_000;

pub struct ShellDiagnosticService<E: Executor> {
    engine: BlossomEngine<E>,
    sessions: ShellSessionApprovals<crate::ApprovalToken>,
    instance_nonce: u64,
    next_request: u64,
}

impl<E: Executor> ShellDiagnosticService<E> {
    pub fn new(executor: E, instance_nonce: u64) -> Self {
        let policy = PolicyEngine::new(vec![PolicyRule {
            capability: Capability::SystemReadKernelIdentity,
            decision: PolicyDecision::Ask,
        }]);
        Self {
            engine: BlossomEngine::new(policy, ApprovalStore::new(SHELL_APPROVAL_TTL_MS), executor),
            sessions: ShellSessionApprovals::default(),
            instance_nonce,
            next_request: 1,
        }
    }

    pub fn begin_system_uname(
        &mut self,
        peer: ShellPeerId,
        now_ms: u64,
    ) -> Result<ShellServiceOutcome, ShellServiceError> {
        if self.sessions.has_pending(&peer) {
            return Err(ShellSessionError::ApprovalAlreadyPending.into());
        }
        let request_id = self.next_request_id()?;
        let request = ToolRequest::SystemUname {
            request_id: request_id.clone(),
        };
        match self.engine.begin_request(request, now_ms)? {
            BeginOutcome::ApprovalRequired { token, .. } => {
                let preview = self.sessions.register_system_uname(
                    peer,
                    request_id,
                    now_ms.saturating_add(SHELL_APPROVAL_TTL_MS),
                    token,
                )?;
                Ok(ShellServiceOutcome::AwaitingApproval(Box::new(preview)))
            }
            BeginOutcome::Denied => Ok(ShellServiceOutcome::Denied),
            BeginOutcome::Completed(completion) => Ok(completion_outcome(completion)),
        }
    }

    pub fn handle_client_request(
        &mut self,
        peer: &ShellPeerId,
        request: ShellClientRequest,
        now_ms: u64,
    ) -> Result<ShellServiceOutcome, ShellServiceError> {
        match request {
            ShellClientRequest::SubmitDecision {
                request_id,
                preview_sha256,
                decision,
            } => {
                let resolved = match self.sessions.resolve(
                    peer,
                    &request_id,
                    &preview_sha256,
                    decision,
                    now_ms,
                ) {
                    Ok(resolved) => resolved,
                    Err(ShellSessionError::ApprovalExpired) => {
                        self.expire_pending(peer, now_ms)?;
                        return Err(ShellSessionError::ApprovalExpired.into());
                    }
                    Err(error) => return Err(error.into()),
                };
                let request = ToolRequest::SystemUname { request_id };
                let token = resolved.into_secret();
                match decision {
                    ShellDecision::ApproveOnce => Ok(completion_outcome(
                        self.engine.approve(token, request, now_ms)?,
                    )),
                    ShellDecision::Deny => {
                        self.engine.deny_approval(token, request, now_ms)?;
                        Ok(ShellServiceOutcome::Denied)
                    }
                }
            }
            ShellClientRequest::CancelPending {
                request_id,
                preview_sha256,
            } => {
                let cancelled =
                    match self
                        .sessions
                        .cancel(peer, &request_id, &preview_sha256, now_ms)
                    {
                        Ok(cancelled) => cancelled,
                        Err(ShellSessionError::ApprovalExpired) => {
                            self.expire_pending(peer, now_ms)?;
                            return Err(ShellSessionError::ApprovalExpired.into());
                        }
                        Err(error) => return Err(error.into()),
                    };
                let token = cancelled.into_secret();
                self.engine.cancel_approval(
                    token,
                    ToolRequest::SystemUname { request_id },
                    now_ms,
                )?;
                Ok(ShellServiceOutcome::Cancelled)
            }
            ShellClientRequest::StartSystemUname | ShellClientRequest::ReadActivity { .. } => {
                Err(ShellServiceError::WrongMethod)
            }
        }
    }

    pub fn disconnect(
        &mut self,
        peer: &ShellPeerId,
        now_ms: u64,
    ) -> Result<bool, ShellServiceError> {
        let Some(cancelled) = self.sessions.disconnect(peer) else {
            return Ok(false);
        };
        let request_id = cancelled.request_id.clone();
        self.engine.cancel_approval(
            cancelled.into_secret(),
            ToolRequest::SystemUname { request_id },
            now_ms,
        )?;
        Ok(true)
    }

    pub fn audit(&self) -> &crate::AuditLog {
        self.engine.audit()
    }

    fn expire_pending(&mut self, peer: &ShellPeerId, now_ms: u64) -> Result<(), ShellServiceError> {
        let Some(expired) = self.sessions.expire(peer, now_ms) else {
            return Err(ShellSessionError::NoPendingApproval.into());
        };
        let request_id = expired.request_id.clone();
        self.engine.cancel_approval(
            expired.into_secret(),
            ToolRequest::SystemUname { request_id },
            now_ms,
        )?;
        Ok(())
    }

    fn next_request_id(&mut self) -> Result<RequestId, ShellServiceError> {
        let sequence = self.next_request;
        self.next_request = self
            .next_request
            .checked_add(1)
            .ok_or(ShellServiceError::RequestIdExhausted)?;
        RequestId::parse(format!("shell-{:016x}-{sequence}", self.instance_nonce))
            .map_err(|_| ShellServiceError::RequestIdExhausted)
    }
}

fn completion_outcome(completion: CompletionOutcome) -> ShellServiceOutcome {
    if completion.verification.succeeded {
        ShellServiceOutcome::Verified
    } else {
        ShellServiceOutcome::VerificationFailed
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShellServiceOutcome {
    AwaitingApproval(Box<ShellApprovalPreview>),
    Denied,
    Cancelled,
    Verified,
    VerificationFailed,
}

#[derive(Debug)]
pub enum ShellServiceError {
    Session(ShellSessionError),
    Engine(EngineError),
    WrongMethod,
    RequestIdExhausted,
}

impl From<ShellSessionError> for ShellServiceError {
    fn from(value: ShellSessionError) -> Self {
        Self::Session(value)
    }
}

impl From<EngineError> for ShellServiceError {
    fn from(value: EngineError) -> Self {
        Self::Engine(value)
    }
}

impl fmt::Display for ShellServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Session(_) => "shell session rejected the request",
            Self::Engine(_) => "shell engine operation failed",
            Self::WrongMethod => "shell request was sent to the wrong service method",
            Self::RequestIdExhausted => "shell request identifier space was exhausted",
        })
    }
}

impl std::error::Error for ShellServiceError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CommandSpec, ExecutionResult, ExecutorError, decode_shell_client_request};
    use std::cell::Cell;
    use std::rc::Rc;

    struct CountingExecutor {
        calls: Rc<Cell<usize>>,
        result: ExecutionResult,
    }

    impl Executor for CountingExecutor {
        fn execute(&mut self, command: &CommandSpec) -> Result<ExecutionResult, ExecutorError> {
            assert_eq!(command, &CommandSpec::system_uname());
            self.calls.set(self.calls.get() + 1);
            Ok(self.result.clone())
        }
    }

    fn service(calls: Rc<Cell<usize>>) -> ShellDiagnosticService<CountingExecutor> {
        ShellDiagnosticService::new(
            CountingExecutor {
                calls,
                result: ExecutionResult {
                    exit_code: Some(0),
                    stdout: b"Linux\n".to_vec(),
                    stderr: vec![],
                    timed_out: false,
                    output_truncated: false,
                },
            },
            7,
        )
    }

    fn peer(value: &str) -> ShellPeerId {
        ShellPeerId::from_bus_unique_name(value).expect("peer")
    }

    fn decision(preview: &ShellApprovalPreview, value: &str) -> ShellClientRequest {
        let encoded = format!(
            r#"{{"kind":"submit_decision","version":1,"request_id":"{}","preview_sha256":"{}","decision":"{value}"}}"#,
            preview.request_id, preview.preview_sha256
        );
        decode_shell_client_request(encoded.as_bytes()).expect("decision schema")
    }

    #[test]
    fn exact_approval_executes_once_and_reports_only_verified_success() {
        let calls = Rc::new(Cell::new(0));
        let mut service = service(calls.clone());
        let owner = peer(":1.10");
        let ShellServiceOutcome::AwaitingApproval(preview) = service
            .begin_system_uname(owner.clone(), 1_000)
            .expect("begin")
        else {
            panic!("expected approval")
        };
        assert_eq!(calls.get(), 0);
        assert_eq!(
            service
                .handle_client_request(&owner, decision(&preview, "approve_once"), 1_001)
                .expect("approve"),
            ShellServiceOutcome::Verified
        );
        assert_eq!(calls.get(), 1);
        assert!(
            service
                .handle_client_request(&owner, decision(&preview, "approve_once"), 1_002)
                .is_err()
        );
        assert_eq!(calls.get(), 1);
    }

    #[test]
    fn denial_disconnect_expiry_and_cross_peer_start_nothing() {
        let calls = Rc::new(Cell::new(0));
        let mut service = service(calls.clone());
        let owner = peer(":1.10");
        let attacker = peer(":1.11");
        let ShellServiceOutcome::AwaitingApproval(preview) = service
            .begin_system_uname(owner.clone(), 1_000)
            .expect("begin")
        else {
            panic!("approval")
        };
        assert!(
            service
                .handle_client_request(&attacker, decision(&preview, "approve_once"), 1_001)
                .is_err()
        );
        assert_eq!(
            service
                .handle_client_request(&owner, decision(&preview, "deny"), 1_002)
                .expect("deny"),
            ShellServiceOutcome::Denied
        );

        service
            .begin_system_uname(owner.clone(), 2_000)
            .expect("begin 2");
        assert!(service.disconnect(&owner, 2_001).expect("disconnect"));

        let ShellServiceOutcome::AwaitingApproval(expiring) = service
            .begin_system_uname(owner.clone(), 3_000)
            .expect("begin 3")
        else {
            panic!("approval")
        };
        assert!(matches!(
            service.handle_client_request(
                &owner,
                decision(&expiring, "approve_once"),
                3_000 + SHELL_APPROVAL_TTL_MS + 1
            ),
            Err(ShellServiceError::Session(
                ShellSessionError::ApprovalExpired
            ))
        ));
        assert_eq!(calls.get(), 0);
    }

    #[test]
    fn schema_cannot_route_start_or_activity_through_decision_method() {
        let calls = Rc::new(Cell::new(0));
        let mut service = service(calls.clone());
        let owner = peer(":1.10");
        for input in [
            br#"{"kind":"start_system_uname","version":1}"#.as_slice(),
            br#"{"kind":"read_activity","version":1,"after_sequence":null,"limit":1}"#.as_slice(),
        ] {
            let request = decode_shell_client_request(input).expect("schema");
            assert!(matches!(
                service.handle_client_request(&owner, request, 1_000),
                Err(ShellServiceError::WrongMethod)
            ));
        }
        assert_eq!(calls.get(), 0);
    }

    #[test]
    fn replacement_issues_no_orphan_token_and_expiry_reaches_engine_audit() {
        let calls = Rc::new(Cell::new(0));
        let mut service = service(calls.clone());
        let owner = peer(":1.10");
        let ShellServiceOutcome::AwaitingApproval(preview) = service
            .begin_system_uname(owner.clone(), 1_000)
            .expect("begin")
        else {
            panic!("approval")
        };
        let records_before_replacement = service.audit().records().len();
        assert!(matches!(
            service.begin_system_uname(owner.clone(), 1_001),
            Err(ShellServiceError::Session(
                ShellSessionError::ApprovalAlreadyPending
            ))
        ));
        assert_eq!(service.audit().records().len(), records_before_replacement);

        assert!(matches!(
            service.handle_client_request(
                &owner,
                decision(&preview, "approve_once"),
                1_000 + SHELL_APPROVAL_TTL_MS + 1
            ),
            Err(ShellServiceError::Session(
                ShellSessionError::ApprovalExpired
            ))
        ));
        let events: Vec<_> = service
            .audit()
            .records()
            .iter()
            .rev()
            .take(2)
            .map(|record| &record.event)
            .collect();
        assert!(matches!(
            events[0],
            crate::AuditEvent::ApprovalCancelled { .. }
        ));
        assert!(matches!(
            events[1],
            crate::AuditEvent::ApprovalRejected { .. }
        ));
        assert_eq!(calls.get(), 0);
    }
}
