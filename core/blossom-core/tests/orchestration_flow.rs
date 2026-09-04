use blossom_core::{
    ApprovalStore, BlossomEngine, Capability, CommandSpec, ExecutionResult, Executor,
    ExecutorError, OrchestrationEvent, PlanId, PlanOrchestrator, PlanOutcome, PolicyDecision,
    PolicyEngine, PolicyRule, ProposedPlanStep, RequestId, StepId, StepTerminalOutcome,
    ToolRequest, ValidatedPlan,
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Debug)]
struct CountingExecutor {
    calls: Arc<AtomicUsize>,
    exit_code: i32,
}

impl Executor for CountingExecutor {
    fn execute(&mut self, _: &CommandSpec) -> Result<ExecutionResult, ExecutorError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(ExecutionResult {
            exit_code: Some(self.exit_code),
            stdout: b"Linux\n".to_vec(),
            stderr: Vec::new(),
            timed_out: false,
            output_truncated: false,
        })
    }
}

fn request_id(value: &str) -> RequestId {
    RequestId::parse(value.into()).unwrap()
}

fn step(step: &str, request: &str, dependency: Option<&str>) -> ProposedPlanStep {
    ProposedPlanStep {
        step_id: StepId::parse(step.into()).unwrap(),
        request: ToolRequest::SystemUname {
            request_id: request_id(request),
        },
        depends_on: dependency
            .map(|value| vec![StepId::parse(value.into()).unwrap()])
            .unwrap_or_default(),
    }
}

fn plan() -> ValidatedPlan {
    ValidatedPlan::new(
        PlanId::parse("integration-plan".into()).unwrap(),
        request_id("integration-correlation"),
        vec![
            step("step-1", "request-1", None),
            step("step-2", "request-2", Some("step-1")),
        ],
    )
    .unwrap()
}

fn engine(
    decision: PolicyDecision,
    ttl_ms: u64,
    exit_code: i32,
    calls: Arc<AtomicUsize>,
) -> BlossomEngine<CountingExecutor> {
    BlossomEngine::new(
        PolicyEngine::new(vec![PolicyRule {
            capability: Capability::SystemReadKernelIdentity,
            decision,
        }]),
        ApprovalStore::new(ttl_ms),
        CountingExecutor { calls, exit_code },
    )
}

#[test]
fn every_step_requires_fresh_approval_and_verified_results_complete_the_plan() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut orchestrator =
        PlanOrchestrator::new(engine(PolicyDecision::Ask, 100, 0, calls.clone()), plan());
    assert!(matches!(
        orchestrator.advance(1_000).unwrap(),
        OrchestrationEvent::ApprovalRequired { .. }
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert!(matches!(
        orchestrator.approve_pending(1_001).unwrap(),
        OrchestrationEvent::StepFinished {
            outcome: StepTerminalOutcome::Verified,
            ..
        }
    ));
    assert!(matches!(
        orchestrator.advance(1_002).unwrap(),
        OrchestrationEvent::ApprovalRequired { .. }
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    orchestrator.approve_pending(1_003).unwrap();
    assert!(matches!(
        orchestrator.advance(1_004).unwrap(),
        OrchestrationEvent::PlanFinished(_)
    ));
    assert_eq!(
        orchestrator.report().unwrap().summary.outcome,
        PlanOutcome::Completed
    );
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert!(orchestrator.engine().audit().verify_chain());
}

#[test]
fn expired_approval_blocks_the_step_and_its_dependency_without_execution() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut orchestrator =
        PlanOrchestrator::new(engine(PolicyDecision::Ask, 10, 0, calls.clone()), plan());
    orchestrator.advance(1_000).unwrap();
    assert!(matches!(
        orchestrator.approve_pending(1_011).unwrap(),
        OrchestrationEvent::StepFinished {
            outcome: StepTerminalOutcome::Blocked,
            ..
        }
    ));
    assert!(matches!(
        orchestrator.advance(1_012).unwrap(),
        OrchestrationEvent::StepFinished {
            outcome: StepTerminalOutcome::Blocked,
            ..
        }
    ));
    orchestrator.advance(1_013).unwrap();
    assert_eq!(
        orchestrator.report().unwrap().summary.outcome,
        PlanOutcome::Blocked
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[test]
fn cancellation_consumes_approval_and_prevents_every_remaining_step() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut orchestrator =
        PlanOrchestrator::new(engine(PolicyDecision::Ask, 100, 0, calls.clone()), plan());
    orchestrator.advance(1_000).unwrap();
    assert!(matches!(
        orchestrator.cancel(1_001).unwrap(),
        OrchestrationEvent::StepFinished {
            outcome: StepTerminalOutcome::CancelledBeforeStart,
            ..
        }
    ));
    assert!(matches!(
        orchestrator.advance(1_002).unwrap(),
        OrchestrationEvent::StepFinished {
            outcome: StepTerminalOutcome::CancelledBeforeStart,
            ..
        }
    ));
    orchestrator.advance(1_003).unwrap();
    assert_eq!(
        orchestrator.report().unwrap().summary.outcome,
        PlanOutcome::Cancelled
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[test]
fn execution_dispatch_without_verification_never_reports_completion() {
    let calls = Arc::new(AtomicUsize::new(0));
    let single = ValidatedPlan::new(
        PlanId::parse("failed-verification".into()).unwrap(),
        request_id("failed-correlation"),
        vec![step("step-1", "failed-request", None)],
    )
    .unwrap();
    let mut orchestrator =
        PlanOrchestrator::new(engine(PolicyDecision::Allow, 100, 1, calls.clone()), single);
    orchestrator.advance(1_000).unwrap();
    orchestrator.advance(1_001).unwrap();
    let report = orchestrator.report().unwrap();
    assert_ne!(report.summary.outcome, PlanOutcome::Completed);
    assert!(report.render().contains("verification failed"));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}
