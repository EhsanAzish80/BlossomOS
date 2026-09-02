use crate::approval::{ApprovalError, ApprovalStore, ApprovalToken};
use crate::audit::{AuditEvent, AuditLog};
use crate::executor::{CommandSpec, Executor, ExecutorError};
use crate::policy::{PolicyDecision, PolicyEngine};
use crate::request::{RequestError, ToolRequest};
use crate::verification::{Verification, verify_execution};

#[derive(Debug)]
pub struct BlossomEngine<E> {
    policy: PolicyEngine,
    approvals: ApprovalStore,
    executor: E,
    audit: AuditLog,
}

impl<E: Executor> BlossomEngine<E> {
    pub fn new(policy: PolicyEngine, approvals: ApprovalStore, executor: E) -> Self {
        Self {
            policy,
            approvals,
            executor,
            audit: AuditLog::default(),
        }
    }

    pub fn begin(&mut self, input: &str, now_ms: u64) -> Result<BeginOutcome, EngineError> {
        let request = match ToolRequest::parse_json(input) {
            Ok(request) => request,
            Err(error) => {
                self.audit.append(AuditEvent::RequestRejected {
                    category: request_error_category(&error).into(),
                });
                return Err(EngineError::InvalidRequest(error));
            }
        };
        self.audit.append(AuditEvent::RequestAccepted {
            request_id: request.request_id().as_str().into(),
            tool: request.tool_name().into(),
        });
        let capability = PolicyEngine::required_capability(&request);
        let decision = self.policy.evaluate(&request);
        self.audit.append(AuditEvent::PolicyEvaluated {
            request_id: request.request_id().as_str().into(),
            capability,
            decision,
        });
        match decision {
            PolicyDecision::Deny => {
                self.audit.append(AuditEvent::Denied {
                    request_id: request.request_id().as_str().into(),
                });
                Ok(BeginOutcome::Denied)
            }
            PolicyDecision::Ask => {
                let token = self.approvals.issue(request.clone(), now_ms);
                self.audit.append(AuditEvent::ApprovalIssued {
                    request_id: request.request_id().as_str().into(),
                });
                Ok(BeginOutcome::ApprovalRequired { request, token })
            }
            PolicyDecision::Allow => self.execute(request).map(BeginOutcome::Completed),
        }
    }

    pub fn approve(
        &mut self,
        token: ApprovalToken,
        request: ToolRequest,
        now_ms: u64,
    ) -> Result<CompletionOutcome, EngineError> {
        if let Err(error) = self.approvals.consume(token, &request, now_ms) {
            self.audit.append(AuditEvent::ApprovalRejected {
                request_id: request.request_id().as_str().into(),
                error,
            });
            return Err(EngineError::Approval(error));
        }
        self.audit.append(AuditEvent::ApprovalConsumed {
            request_id: request.request_id().as_str().into(),
        });
        self.execute(request)
    }

    pub fn deny_approval(
        &mut self,
        token: ApprovalToken,
        request: ToolRequest,
        now_ms: u64,
    ) -> Result<(), EngineError> {
        if let Err(error) = self.approvals.consume(token, &request, now_ms) {
            self.audit.append(AuditEvent::ApprovalRejected {
                request_id: request.request_id().as_str().into(),
                error,
            });
            return Err(EngineError::Approval(error));
        }
        self.audit.append(AuditEvent::ApprovalDenied {
            request_id: request.request_id().as_str().into(),
        });
        Ok(())
    }

    pub fn cancel_approval(
        &mut self,
        token: ApprovalToken,
        request: ToolRequest,
        now_ms: u64,
    ) -> Result<(), EngineError> {
        if let Err(error) = self.approvals.consume(token, &request, now_ms) {
            self.audit.append(AuditEvent::ApprovalRejected {
                request_id: request.request_id().as_str().into(),
                error,
            });
            if error != ApprovalError::Expired {
                return Err(EngineError::Approval(error));
            }
        }
        self.audit.append(AuditEvent::ApprovalCancelled {
            request_id: request.request_id().as_str().into(),
        });
        Ok(())
    }

    pub fn audit(&self) -> &AuditLog {
        &self.audit
    }

    fn execute(&mut self, request: ToolRequest) -> Result<CompletionOutcome, EngineError> {
        let command = command_for(&request);
        self.audit.append(AuditEvent::ExecutionStarted {
            request_id: request.request_id().as_str().into(),
            program: command.program.display().to_string(),
        });
        let result = match self.executor.execute(&command) {
            Ok(result) => result,
            Err(error) => {
                self.audit.append(AuditEvent::ExecutionFailed {
                    request_id: request.request_id().as_str().into(),
                    error: error.clone(),
                });
                return Err(EngineError::Executor(error));
            }
        };
        self.audit
            .append(AuditEvent::execution_finished(&request, &result));
        let verification = verify_execution(&result);
        self.audit.append(AuditEvent::VerificationFinished {
            request_id: request.request_id().as_str().into(),
            verification: verification.clone(),
        });
        Ok(CompletionOutcome {
            request,
            verification,
        })
    }
}

pub fn command_for(request: &ToolRequest) -> CommandSpec {
    match request {
        ToolRequest::SystemUname { .. } => CommandSpec::system_uname(),
    }
}

fn request_error_category(error: &RequestError) -> &'static str {
    match error {
        RequestError::MalformedJson { .. } => "malformed_json",
        RequestError::InvalidRequestId => "invalid_request_id",
        RequestError::UnknownTool { .. } => "unknown_tool",
        RequestError::InvalidArguments { .. } => "invalid_arguments",
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BeginOutcome {
    Denied,
    ApprovalRequired {
        request: ToolRequest,
        token: ApprovalToken,
    },
    Completed(CompletionOutcome),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompletionOutcome {
    pub request: ToolRequest,
    pub verification: Verification,
}

#[derive(Debug)]
pub enum EngineError {
    InvalidRequest(RequestError),
    Approval(ApprovalError),
    Executor(ExecutorError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::ExecutionResult;
    use crate::policy::{Capability, PolicyRule};
    use std::collections::VecDeque;

    const REQUEST: &str = r#"{"request_id":"req-1","tool":"system.uname","arguments":{}}"#;

    #[derive(Debug)]
    struct ScriptedExecutor {
        outcomes: VecDeque<Result<ExecutionResult, ExecutorError>>,
        calls: Vec<CommandSpec>,
    }

    impl ScriptedExecutor {
        fn successful() -> Self {
            Self {
                outcomes: VecDeque::from([Ok(ExecutionResult {
                    exit_code: Some(0),
                    stdout: b"Linux\n".to_vec(),
                    stderr: Vec::new(),
                    timed_out: false,
                    output_truncated: false,
                })]),
                calls: Vec::new(),
            }
        }
    }

    impl Executor for ScriptedExecutor {
        fn execute(&mut self, command: &CommandSpec) -> Result<ExecutionResult, ExecutorError> {
            self.calls.push(command.clone());
            self.outcomes
                .pop_front()
                .unwrap_or(Err(ExecutorError::Failed))
        }
    }

    fn engine(
        decision: PolicyDecision,
        executor: ScriptedExecutor,
    ) -> BlossomEngine<ScriptedExecutor> {
        BlossomEngine::new(
            PolicyEngine::new(vec![PolicyRule {
                capability: Capability::SystemReadKernelIdentity,
                decision,
            }]),
            ApprovalStore::new(100),
            executor,
        )
    }

    #[test]
    fn completes_ask_approve_execute_verify_audit_flow() {
        let mut engine = engine(PolicyDecision::Ask, ScriptedExecutor::successful());
        let (request, token) = match engine.begin(REQUEST, 1_000).expect("begin should work") {
            BeginOutcome::ApprovalRequired { request, token } => (request, token),
            outcome => panic!("unexpected begin outcome: {outcome:?}"),
        };
        let completed = engine
            .approve(token, request, 1_001)
            .expect("approval should work");
        assert!(completed.verification.succeeded);
        assert!(engine.audit().verify_chain());
        assert_eq!(engine.executor.calls.len(), 1);
        assert_eq!(
            engine.executor.calls[0].program.to_string_lossy(),
            "/usr/bin/uname"
        );
        assert!(!engine.executor.calls[0].network_allowed);
    }

    #[test]
    fn deny_never_calls_executor() {
        let mut engine = engine(PolicyDecision::Deny, ScriptedExecutor::successful());
        assert_eq!(
            engine.begin(REQUEST, 1_000).expect("begin should work"),
            BeginOutcome::Denied
        );
        assert!(engine.executor.calls.is_empty());
        assert!(engine.audit().verify_chain());
    }

    #[test]
    fn user_denial_consumes_approval_without_execution() {
        let mut engine = engine(PolicyDecision::Ask, ScriptedExecutor::successful());
        let (request, token) = match engine.begin(REQUEST, 1_000).expect("begin should work") {
            BeginOutcome::ApprovalRequired { request, token } => (request, token),
            outcome => panic!("unexpected begin outcome: {outcome:?}"),
        };
        engine
            .deny_approval(token, request.clone(), 1_001)
            .expect("denial should consume approval");
        assert!(engine.executor.calls.is_empty());
        assert!(matches!(
            engine.approve(token, request, 1_002),
            Err(EngineError::Approval(ApprovalError::Replay))
        ));
        assert!(engine.audit().verify_chain());
    }

    #[test]
    fn cancellation_consumes_approval_without_execution() {
        let mut engine = engine(PolicyDecision::Ask, ScriptedExecutor::successful());
        let (request, token) = match engine.begin(REQUEST, 1_000).expect("begin should work") {
            BeginOutcome::ApprovalRequired { request, token } => (request, token),
            outcome => panic!("unexpected begin outcome: {outcome:?}"),
        };
        engine
            .cancel_approval(token, request.clone(), 1_001)
            .expect("cancellation should consume approval");
        assert!(engine.executor.calls.is_empty());
        assert!(matches!(
            engine.approve(token, request, 1_002),
            Err(EngineError::Approval(ApprovalError::Replay))
        ));
        assert!(matches!(
            engine.audit().records()[3].event,
            AuditEvent::ApprovalCancelled { .. }
        ));
    }

    #[test]
    fn cancellation_is_recorded_even_after_approval_expires() {
        let mut engine = engine(PolicyDecision::Ask, ScriptedExecutor::successful());
        let (request, token) = match engine.begin(REQUEST, 1_000).expect("begin should work") {
            BeginOutcome::ApprovalRequired { request, token } => (request, token),
            outcome => panic!("unexpected begin outcome: {outcome:?}"),
        };
        engine
            .cancel_approval(token, request, 1_101)
            .expect("cancellation should remain auditable after expiry");
        assert!(engine.executor.calls.is_empty());
        assert!(matches!(
            engine.audit().records().last().map(|record| &record.event),
            Some(AuditEvent::ApprovalCancelled { .. })
        ));
    }

    #[test]
    fn explicit_allow_executes_without_approval() {
        let mut engine = engine(PolicyDecision::Allow, ScriptedExecutor::successful());
        let outcome = engine.begin(REQUEST, 1_000).expect("begin should work");
        assert!(matches!(outcome, BeginOutcome::Completed(_)));
        assert_eq!(engine.executor.calls.len(), 1);
    }

    #[test]
    fn records_executor_failure() {
        let executor = ScriptedExecutor {
            outcomes: VecDeque::from([Err(ExecutorError::Timeout)]),
            calls: Vec::new(),
        };
        let mut engine = engine(PolicyDecision::Allow, executor);
        assert!(matches!(
            engine.begin(REQUEST, 1_000),
            Err(EngineError::Executor(ExecutorError::Timeout))
        ));
        assert!(engine.audit().verify_chain());
    }

    #[test]
    fn rejects_malformed_request_before_execution() {
        let mut engine = engine(PolicyDecision::Allow, ScriptedExecutor::successful());
        assert!(matches!(
            engine.begin("not-json", 1_000),
            Err(EngineError::InvalidRequest(_))
        ));
        assert!(engine.executor.calls.is_empty());
        assert_eq!(engine.audit().records().len(), 1);
        assert!(matches!(
            engine.audit().records()[0].event,
            AuditEvent::RequestRejected { .. }
        ));
    }
}
