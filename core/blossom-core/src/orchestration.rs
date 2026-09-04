//! Closed Phase 5 plan and lifecycle types.
//!
//! These types contain no executor and accept no caller-supplied capability.
//! Capabilities are projected from the existing typed request registry.

use crate::verification::VerificationReason;
use crate::{
    ApprovalToken, BeginOutcome, Capability, CompletionOutcome, EngineError, PolicyEngine,
    RequestId, ToolRequest,
};
use serde::Serialize;
use std::collections::HashSet;
use std::fmt;

pub const MAX_PLAN_STEPS: usize = 16;
const MAX_PLAN_ID_BYTES: usize = 64;
const MAX_STEP_ID_BYTES: usize = 64;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
pub struct PlanId(String);

impl PlanId {
    pub fn parse(value: String) -> Result<Self, PlanError> {
        if valid_identifier(&value, MAX_PLAN_ID_BYTES) {
            Ok(Self(value))
        } else {
            Err(PlanError::InvalidPlanId)
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
pub struct StepId(String);

impl StepId {
    pub fn parse(value: String) -> Result<Self, PlanError> {
        if valid_identifier(&value, MAX_STEP_ID_BYTES) {
            Ok(Self(value))
        } else {
            Err(PlanError::InvalidStepId)
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn valid_identifier(value: &str, bound: usize) -> bool {
    !value.is_empty()
        && value.len() <= bound
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProposedPlanStep {
    pub step_id: StepId,
    pub request: ToolRequest,
    pub depends_on: Vec<StepId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ValidatedPlanStep {
    step_id: StepId,
    request: ToolRequest,
    capability: Capability,
    depends_on: Vec<StepId>,
}

impl ValidatedPlanStep {
    pub fn step_id(&self) -> &StepId {
        &self.step_id
    }

    pub fn request(&self) -> &ToolRequest {
        &self.request
    }

    pub fn capability(&self) -> Capability {
        self.capability
    }

    pub fn depends_on(&self) -> &[StepId] {
        &self.depends_on
    }

    pub fn is_effectful(&self) -> bool {
        matches!(self.capability, Capability::FilesWriteCreate)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ValidatedPlan {
    plan_id: PlanId,
    correlation_id: RequestId,
    steps: Vec<ValidatedPlanStep>,
}

impl ValidatedPlan {
    pub fn new(
        plan_id: PlanId,
        correlation_id: RequestId,
        proposed: Vec<ProposedPlanStep>,
    ) -> Result<Self, PlanError> {
        if proposed.is_empty() {
            return Err(PlanError::EmptyPlan);
        }
        if proposed.len() > MAX_PLAN_STEPS {
            return Err(PlanError::TooManySteps);
        }

        let mut prior_steps = HashSet::new();
        let mut request_ids = HashSet::new();
        let mut steps = Vec::with_capacity(proposed.len());
        for step in proposed {
            if prior_steps.contains(&step.step_id) {
                return Err(PlanError::DuplicateStepId);
            }
            if !request_ids.insert(step.request.request_id().as_str().to_owned()) {
                return Err(PlanError::DuplicateRequestId);
            }
            let mut dependencies = HashSet::new();
            for dependency in &step.depends_on {
                if !dependencies.insert(dependency) {
                    return Err(PlanError::DuplicateDependency);
                }
                if !prior_steps.contains(dependency) {
                    return Err(PlanError::DependencyNotEarlier);
                }
            }
            prior_steps.insert(step.step_id.clone());
            steps.push(ValidatedPlanStep {
                capability: PolicyEngine::required_capability(&step.request),
                step_id: step.step_id,
                request: step.request,
                depends_on: step.depends_on,
            });
        }

        Ok(Self {
            plan_id,
            correlation_id,
            steps,
        })
    }

    pub fn plan_id(&self) -> &PlanId {
        &self.plan_id
    }

    pub fn correlation_id(&self) -> &RequestId {
        &self.correlation_id
    }

    pub fn steps(&self) -> &[ValidatedPlanStep] {
        &self.steps
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum StepTerminalOutcome {
    Verified,
    Denied,
    CancelledBeforeStart,
    CancelledAfterStart,
    ExecutionFailed,
    VerificationFailed,
    Blocked,
    Indeterminate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum StepPhase {
    Validated,
    CapabilityAnalyzed,
    AwaitingApproval,
    Executing,
    Verifying,
    Terminal(StepTerminalOutcome),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct StepLifecycle {
    step_id: StepId,
    phase: StepPhase,
}

impl StepLifecycle {
    pub fn new(step_id: StepId) -> Self {
        Self {
            step_id,
            phase: StepPhase::Validated,
        }
    }

    pub fn step_id(&self) -> &StepId {
        &self.step_id
    }

    pub fn phase(&self) -> StepPhase {
        self.phase
    }

    pub fn capability_analyzed(&mut self) -> Result<(), StateError> {
        self.transition(StepPhase::Validated, StepPhase::CapabilityAnalyzed)
    }

    pub fn awaiting_approval(&mut self) -> Result<(), StateError> {
        self.transition(StepPhase::CapabilityAnalyzed, StepPhase::AwaitingApproval)
    }

    pub fn executing(&mut self) -> Result<(), StateError> {
        match self.phase {
            StepPhase::CapabilityAnalyzed | StepPhase::AwaitingApproval => {
                self.phase = StepPhase::Executing;
                Ok(())
            }
            _ => Err(StateError::InvalidTransition),
        }
    }

    pub fn verifying(&mut self) -> Result<(), StateError> {
        self.transition(StepPhase::Executing, StepPhase::Verifying)
    }

    pub fn finish(&mut self, outcome: StepTerminalOutcome) -> Result<(), StateError> {
        let permitted = match outcome {
            StepTerminalOutcome::Verified | StepTerminalOutcome::VerificationFailed => {
                self.phase == StepPhase::Verifying
            }
            StepTerminalOutcome::Denied => matches!(
                self.phase,
                StepPhase::CapabilityAnalyzed | StepPhase::AwaitingApproval
            ),
            StepTerminalOutcome::CancelledBeforeStart | StepTerminalOutcome::Blocked => matches!(
                self.phase,
                StepPhase::Validated | StepPhase::CapabilityAnalyzed | StepPhase::AwaitingApproval
            ),
            StepTerminalOutcome::CancelledAfterStart
            | StepTerminalOutcome::ExecutionFailed
            | StepTerminalOutcome::Indeterminate => self.phase == StepPhase::Executing,
        };
        if !permitted {
            return Err(StateError::InvalidTerminalOutcome);
        }
        self.phase = StepPhase::Terminal(outcome);
        Ok(())
    }

    fn transition(&mut self, from: StepPhase, to: StepPhase) -> Result<(), StateError> {
        if self.phase != from {
            return Err(StateError::InvalidTransition);
        }
        self.phase = to;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum PlanOutcome {
    Completed,
    PartiallyCompleted,
    Cancelled,
    Blocked,
    Indeterminate,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TruthfulPlanSummary {
    pub outcome: PlanOutcome,
    pub verified_steps: usize,
    pub verified_effectful_steps: usize,
    pub not_started_steps: usize,
    pub uncertain_steps: usize,
}

impl TruthfulPlanSummary {
    pub fn from_terminal_steps(
        plan: &ValidatedPlan,
        outcomes: &[StepTerminalOutcome],
    ) -> Result<Self, SummaryError> {
        if outcomes.len() != plan.steps.len() {
            return Err(SummaryError::IncompleteTerminalSet);
        }
        let verified_steps = outcomes
            .iter()
            .filter(|outcome| **outcome == StepTerminalOutcome::Verified)
            .count();
        let verified_effectful_steps = outcomes
            .iter()
            .zip(&plan.steps)
            .filter(|(outcome, step)| {
                **outcome == StepTerminalOutcome::Verified && step.is_effectful()
            })
            .count();
        let not_started_steps = outcomes
            .iter()
            .filter(|outcome| {
                matches!(
                    outcome,
                    StepTerminalOutcome::Denied
                        | StepTerminalOutcome::CancelledBeforeStart
                        | StepTerminalOutcome::Blocked
                )
            })
            .count();
        let uncertain_steps = outcomes
            .iter()
            .filter(|outcome| {
                matches!(
                    outcome,
                    StepTerminalOutcome::CancelledAfterStart | StepTerminalOutcome::Indeterminate
                )
            })
            .count();

        let outcome = if outcomes.contains(&StepTerminalOutcome::Indeterminate) {
            PlanOutcome::Indeterminate
        } else if verified_steps == outcomes.len() {
            PlanOutcome::Completed
        } else if verified_effectful_steps > 0 {
            PlanOutcome::PartiallyCompleted
        } else if outcomes.iter().any(|outcome| {
            matches!(
                outcome,
                StepTerminalOutcome::CancelledBeforeStart
                    | StepTerminalOutcome::CancelledAfterStart
            )
        }) {
            PlanOutcome::Cancelled
        } else {
            PlanOutcome::Blocked
        };

        Ok(Self {
            outcome,
            verified_steps,
            verified_effectful_steps,
            not_started_steps,
            uncertain_steps,
        })
    }
}

/// Narrow adapter implemented by the existing trusted engine. Approval tokens
/// never leave `PlanOrchestrator`.
pub trait TypedRequestEngine {
    fn begin_typed(
        &mut self,
        request: ToolRequest,
        now_ms: u64,
    ) -> Result<BeginOutcome, EngineError>;

    fn approve_typed(
        &mut self,
        token: ApprovalToken,
        request: ToolRequest,
        now_ms: u64,
    ) -> Result<CompletionOutcome, EngineError>;

    fn deny_typed(
        &mut self,
        token: ApprovalToken,
        request: ToolRequest,
        now_ms: u64,
    ) -> Result<(), EngineError>;

    fn cancel_typed(
        &mut self,
        token: ApprovalToken,
        request: ToolRequest,
        now_ms: u64,
    ) -> Result<(), EngineError>;
}

struct PendingStep {
    index: usize,
    token: ApprovalToken,
    request: ToolRequest,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OrchestrationEvent {
    ApprovalRequired {
        plan_id: PlanId,
        step_id: StepId,
        request: ToolRequest,
        capability: Capability,
    },
    StepFinished {
        plan_id: PlanId,
        step_id: StepId,
        outcome: StepTerminalOutcome,
    },
    PlanFinished(TruthfulPlanSummary),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrchestrationError {
    ApprovalAlreadyPending,
    NoApprovalPending,
    EngineContractViolation,
    InvalidLifecycle,
    SummaryUnavailable,
}

pub struct PlanOrchestrator<E> {
    engine: E,
    plan: ValidatedPlan,
    lifecycles: Vec<StepLifecycle>,
    outcomes: Vec<Option<StepTerminalOutcome>>,
    cursor: usize,
    pending: Option<PendingStep>,
    cancellation_requested: bool,
}

impl<E: TypedRequestEngine> PlanOrchestrator<E> {
    pub fn new(engine: E, plan: ValidatedPlan) -> Self {
        let lifecycles = plan
            .steps()
            .iter()
            .map(|step| StepLifecycle::new(step.step_id().clone()))
            .collect();
        let outcomes = vec![None; plan.steps().len()];
        Self {
            engine,
            plan,
            lifecycles,
            outcomes,
            cursor: 0,
            pending: None,
            cancellation_requested: false,
        }
    }

    pub fn plan(&self) -> &ValidatedPlan {
        &self.plan
    }

    pub fn lifecycles(&self) -> &[StepLifecycle] {
        &self.lifecycles
    }

    pub fn outcomes(&self) -> &[Option<StepTerminalOutcome>] {
        &self.outcomes
    }

    pub fn engine(&self) -> &E {
        &self.engine
    }

    pub fn into_engine(self) -> E {
        self.engine
    }

    pub fn advance(&mut self, now_ms: u64) -> Result<OrchestrationEvent, OrchestrationError> {
        if self.pending.is_some() {
            return Err(OrchestrationError::ApprovalAlreadyPending);
        }
        if self.cursor == self.plan.steps().len() {
            return self.finished_event();
        }

        let index = self.cursor;
        let step = self.plan.steps()[index].clone();
        if self.cancellation_requested {
            return self.finish_without_start(index, StepTerminalOutcome::CancelledBeforeStart);
        }
        if !self.dependencies_verified(&step) {
            return self.finish_without_start(index, StepTerminalOutcome::Blocked);
        }

        self.lifecycles[index]
            .capability_analyzed()
            .map_err(|_| OrchestrationError::InvalidLifecycle)?;
        match self.engine.begin_typed(step.request().clone(), now_ms) {
            Ok(BeginOutcome::Denied) => {
                self.finish_without_start(index, StepTerminalOutcome::Denied)
            }
            Ok(BeginOutcome::ApprovalRequired { request, token }) => {
                self.lifecycles[index]
                    .awaiting_approval()
                    .map_err(|_| OrchestrationError::InvalidLifecycle)?;
                if request != *step.request() {
                    let _ = self.engine.cancel_typed(token, request, now_ms);
                    self.lifecycles[index]
                        .finish(StepTerminalOutcome::Blocked)
                        .map_err(|_| OrchestrationError::InvalidLifecycle)?;
                    self.outcomes[index] = Some(StepTerminalOutcome::Blocked);
                    self.cursor += 1;
                    return Err(OrchestrationError::EngineContractViolation);
                }
                self.pending = Some(PendingStep {
                    index,
                    token,
                    request: request.clone(),
                });
                Ok(OrchestrationEvent::ApprovalRequired {
                    plan_id: self.plan.plan_id().clone(),
                    step_id: step.step_id().clone(),
                    request,
                    capability: step.capability(),
                })
            }
            Ok(BeginOutcome::Completed(completion)) => self.finish_completion(index, completion),
            Err(_) => self.finish_after_start(index, StepTerminalOutcome::ExecutionFailed),
        }
    }

    pub fn approve_pending(
        &mut self,
        now_ms: u64,
    ) -> Result<OrchestrationEvent, OrchestrationError> {
        let pending = self
            .pending
            .take()
            .ok_or(OrchestrationError::NoApprovalPending)?;
        match self
            .engine
            .approve_typed(pending.token, pending.request, now_ms)
        {
            Ok(completion) => self.finish_completion(pending.index, completion),
            Err(EngineError::Approval(_)) => {
                self.finish_without_start(pending.index, StepTerminalOutcome::Blocked)
            }
            Err(_) => self.finish_after_start(pending.index, StepTerminalOutcome::ExecutionFailed),
        }
    }

    pub fn deny_pending(&mut self, now_ms: u64) -> Result<OrchestrationEvent, OrchestrationError> {
        let pending = self
            .pending
            .take()
            .ok_or(OrchestrationError::NoApprovalPending)?;
        let outcome = if self
            .engine
            .deny_typed(pending.token, pending.request, now_ms)
            .is_ok()
        {
            StepTerminalOutcome::Denied
        } else {
            StepTerminalOutcome::Blocked
        };
        self.finish_without_start(pending.index, outcome)
    }

    pub fn cancel(&mut self, now_ms: u64) -> Result<OrchestrationEvent, OrchestrationError> {
        self.cancellation_requested = true;
        if let Some(pending) = self.pending.take() {
            let outcome = if self
                .engine
                .cancel_typed(pending.token, pending.request, now_ms)
                .is_ok()
            {
                StepTerminalOutcome::CancelledBeforeStart
            } else {
                StepTerminalOutcome::Blocked
            };
            return self.finish_without_start(pending.index, outcome);
        }
        self.advance(now_ms)
    }

    fn dependencies_verified(&self, step: &ValidatedPlanStep) -> bool {
        step.depends_on().iter().all(|dependency| {
            self.plan
                .steps()
                .iter()
                .position(|candidate| candidate.step_id() == dependency)
                .and_then(|index| self.outcomes[index])
                == Some(StepTerminalOutcome::Verified)
        })
    }

    fn finish_completion(
        &mut self,
        index: usize,
        completion: CompletionOutcome,
    ) -> Result<OrchestrationEvent, OrchestrationError> {
        if completion.request != *self.plan.steps()[index].request() {
            return self.finish_after_start(index, StepTerminalOutcome::Indeterminate);
        }
        self.lifecycles[index]
            .executing()
            .and_then(|_| self.lifecycles[index].verifying())
            .map_err(|_| OrchestrationError::InvalidLifecycle)?;
        let outcome = if completion.verification.succeeded {
            StepTerminalOutcome::Verified
        } else if completion.verification.reason
            == VerificationReason::WorkspaceFileDurabilityUncertain
        {
            StepTerminalOutcome::Indeterminate
        } else {
            StepTerminalOutcome::VerificationFailed
        };
        self.finish_terminal(index, outcome)
    }

    fn finish_after_start(
        &mut self,
        index: usize,
        outcome: StepTerminalOutcome,
    ) -> Result<OrchestrationEvent, OrchestrationError> {
        self.lifecycles[index]
            .executing()
            .map_err(|_| OrchestrationError::InvalidLifecycle)?;
        self.finish_terminal(index, outcome)
    }

    fn finish_without_start(
        &mut self,
        index: usize,
        outcome: StepTerminalOutcome,
    ) -> Result<OrchestrationEvent, OrchestrationError> {
        self.finish_terminal(index, outcome)
    }

    fn finish_terminal(
        &mut self,
        index: usize,
        outcome: StepTerminalOutcome,
    ) -> Result<OrchestrationEvent, OrchestrationError> {
        self.lifecycles[index]
            .finish(outcome)
            .map_err(|_| OrchestrationError::InvalidLifecycle)?;
        self.outcomes[index] = Some(outcome);
        self.cursor += 1;
        Ok(OrchestrationEvent::StepFinished {
            plan_id: self.plan.plan_id().clone(),
            step_id: self.plan.steps()[index].step_id().clone(),
            outcome,
        })
    }

    fn finished_event(&self) -> Result<OrchestrationEvent, OrchestrationError> {
        let outcomes = self
            .outcomes
            .iter()
            .copied()
            .collect::<Option<Vec<_>>>()
            .ok_or(OrchestrationError::SummaryUnavailable)?;
        TruthfulPlanSummary::from_terminal_steps(&self.plan, &outcomes)
            .map(OrchestrationEvent::PlanFinished)
            .map_err(|_| OrchestrationError::SummaryUnavailable)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlanError {
    InvalidPlanId,
    InvalidStepId,
    EmptyPlan,
    TooManySteps,
    DuplicateStepId,
    DuplicateRequestId,
    DuplicateDependency,
    DependencyNotEarlier,
}

impl fmt::Display for PlanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidPlanId => "invalid plan identifier",
            Self::InvalidStepId => "invalid step identifier",
            Self::EmptyPlan => "plan must contain a step",
            Self::TooManySteps => "plan exceeds the fixed step limit",
            Self::DuplicateStepId => "plan contains a duplicate step identifier",
            Self::DuplicateRequestId => "plan contains a duplicate request identifier",
            Self::DuplicateDependency => "plan contains a duplicate dependency",
            Self::DependencyNotEarlier => "dependency must identify an earlier step",
        })
    }
}

impl std::error::Error for PlanError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StateError {
    InvalidTransition,
    InvalidTerminalOutcome,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SummaryError {
    IncompleteTerminalSet,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ApprovalStore, BlossomEngine, CommandSpec, ExecutionResult, Executor, ExecutorError,
        PolicyRule,
    };
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    #[derive(Debug)]
    struct CountingExecutor {
        calls: Arc<AtomicUsize>,
        result: ExecutionResult,
    }

    impl Executor for CountingExecutor {
        fn execute(&mut self, _: &CommandSpec) -> Result<ExecutionResult, ExecutorError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.result.clone())
        }
    }

    fn id(value: &str) -> RequestId {
        RequestId::parse(value.into()).expect("valid request id")
    }

    fn step(step: &str, request: &str, dependencies: &[&str]) -> ProposedPlanStep {
        ProposedPlanStep {
            step_id: StepId::parse(step.into()).expect("valid step id"),
            request: ToolRequest::SystemUptime {
                request_id: id(request),
            },
            depends_on: dependencies
                .iter()
                .map(|value| StepId::parse((*value).into()).expect("valid dependency"))
                .collect(),
        }
    }

    fn uname_step(step: &str, request: &str, dependencies: &[&str]) -> ProposedPlanStep {
        ProposedPlanStep {
            step_id: StepId::parse(step.into()).expect("valid step id"),
            request: ToolRequest::SystemUname {
                request_id: id(request),
            },
            depends_on: dependencies
                .iter()
                .map(|value| StepId::parse((*value).into()).expect("valid dependency"))
                .collect(),
        }
    }

    fn uname_engine(
        decision: crate::PolicyDecision,
        exit_code: i32,
        calls: Arc<AtomicUsize>,
    ) -> BlossomEngine<CountingExecutor> {
        BlossomEngine::new(
            crate::PolicyEngine::new(vec![PolicyRule {
                capability: Capability::SystemReadKernelIdentity,
                decision,
            }]),
            ApprovalStore::new(100),
            CountingExecutor {
                calls,
                result: ExecutionResult {
                    exit_code: Some(exit_code),
                    stdout: b"Linux\n".to_vec(),
                    stderr: Vec::new(),
                    timed_out: false,
                    output_truncated: false,
                },
            },
        )
    }

    fn plan(steps: Vec<ProposedPlanStep>) -> Result<ValidatedPlan, PlanError> {
        ValidatedPlan::new(
            PlanId::parse("plan-1".into()).unwrap(),
            id("correlation-1"),
            steps,
        )
    }

    #[test]
    fn identifiers_are_closed_and_bounded() {
        for invalid in ["", "has space", "path/name", "line\nbreak"] {
            assert_eq!(PlanId::parse(invalid.into()), Err(PlanError::InvalidPlanId));
            assert_eq!(StepId::parse(invalid.into()), Err(PlanError::InvalidStepId));
        }
        assert_eq!(
            PlanId::parse("a".repeat(MAX_PLAN_ID_BYTES + 1)),
            Err(PlanError::InvalidPlanId)
        );
    }

    #[test]
    fn plans_are_nonempty_and_bounded() {
        assert_eq!(plan(vec![]), Err(PlanError::EmptyPlan));
        let too_many = (0..=MAX_PLAN_STEPS)
            .map(|index| step(&format!("step-{index}"), &format!("req-{index}"), &[]))
            .collect();
        assert_eq!(plan(too_many), Err(PlanError::TooManySteps));
    }

    #[test]
    fn dependencies_must_be_unique_and_strictly_earlier() {
        assert_eq!(
            plan(vec![step("step-1", "req-1", &["step-2"])]),
            Err(PlanError::DependencyNotEarlier)
        );
        assert_eq!(
            plan(vec![step("step-1", "req-1", &["step-1"])]),
            Err(PlanError::DependencyNotEarlier)
        );
        assert_eq!(
            plan(vec![
                step("step-1", "req-1", &[]),
                step("step-2", "req-2", &["step-1", "step-1"]),
            ]),
            Err(PlanError::DuplicateDependency)
        );
    }

    #[test]
    fn duplicate_step_and_request_ids_fail_closed() {
        assert_eq!(
            plan(vec![
                step("step-1", "req-1", &[]),
                step("step-1", "req-2", &[]),
            ]),
            Err(PlanError::DuplicateStepId)
        );
        assert_eq!(
            plan(vec![
                step("step-1", "req-1", &[]),
                step("step-2", "req-1", &[]),
            ]),
            Err(PlanError::DuplicateRequestId)
        );
    }

    #[test]
    fn capability_is_derived_only_from_the_typed_request() {
        let plan = plan(vec![step("step-1", "req-1", &[])]).unwrap();
        assert_eq!(plan.steps()[0].capability(), Capability::SystemReadUptime);
    }

    #[test]
    fn lifecycle_is_monotonic_and_verified_requires_verifying() {
        let mut lifecycle = StepLifecycle::new(StepId::parse("step-1".into()).unwrap());
        assert_eq!(
            lifecycle.finish(StepTerminalOutcome::Verified),
            Err(StateError::InvalidTerminalOutcome)
        );
        lifecycle.capability_analyzed().unwrap();
        lifecycle.awaiting_approval().unwrap();
        lifecycle.executing().unwrap();
        lifecycle.verifying().unwrap();
        lifecycle.finish(StepTerminalOutcome::Verified).unwrap();
        assert_eq!(
            lifecycle.capability_analyzed(),
            Err(StateError::InvalidTransition)
        );
    }

    #[test]
    fn denial_and_pre_start_cancellation_never_enter_execution() {
        let step_id = StepId::parse("step-1".into()).unwrap();
        let mut denied = StepLifecycle::new(step_id.clone());
        denied.capability_analyzed().unwrap();
        denied.awaiting_approval().unwrap();
        denied.finish(StepTerminalOutcome::Denied).unwrap();
        assert_eq!(denied.executing(), Err(StateError::InvalidTransition));

        let mut cancelled = StepLifecycle::new(step_id);
        cancelled
            .finish(StepTerminalOutcome::CancelledBeforeStart)
            .unwrap();
        assert_eq!(cancelled.executing(), Err(StateError::InvalidTransition));
    }

    #[test]
    fn execution_cannot_claim_verified_without_verification() {
        let mut lifecycle = StepLifecycle::new(StepId::parse("step-1".into()).unwrap());
        lifecycle.capability_analyzed().unwrap();
        lifecycle.executing().unwrap();
        assert_eq!(
            lifecycle.finish(StepTerminalOutcome::Verified),
            Err(StateError::InvalidTerminalOutcome)
        );
        lifecycle
            .finish(StepTerminalOutcome::ExecutionFailed)
            .unwrap();
    }

    #[test]
    fn indeterminate_dominates_the_authoritative_summary() {
        let plan = plan(vec![
            step("step-1", "req-1", &[]),
            step("step-2", "req-2", &["step-1"]),
        ])
        .unwrap();
        let summary = TruthfulPlanSummary::from_terminal_steps(
            &plan,
            &[
                StepTerminalOutcome::Verified,
                StepTerminalOutcome::Indeterminate,
            ],
        )
        .unwrap();
        assert_eq!(summary.outcome, PlanOutcome::Indeterminate);
        assert_eq!(summary.verified_steps, 1);
        assert_eq!(summary.uncertain_steps, 1);
    }

    #[test]
    fn summary_rejects_missing_terminal_evidence() {
        let plan = plan(vec![step("step-1", "req-1", &[])]).unwrap();
        assert_eq!(
            TruthfulPlanSummary::from_terminal_steps(&plan, &[]),
            Err(SummaryError::IncompleteTerminalSet)
        );
    }

    #[test]
    fn ask_keeps_token_private_and_approval_runs_exactly_once() {
        let calls = Arc::new(AtomicUsize::new(0));
        let plan = ValidatedPlan::new(
            PlanId::parse("plan-ask".into()).unwrap(),
            id("correlation-ask"),
            vec![uname_step("step-1", "req-1", &[])],
        )
        .unwrap();
        let mut orchestrator = PlanOrchestrator::new(
            uname_engine(crate::PolicyDecision::Ask, 0, calls.clone()),
            plan,
        );
        let OrchestrationEvent::ApprovalRequired { request, .. } =
            orchestrator.advance(1_000).unwrap()
        else {
            panic!("approval required")
        };
        assert_eq!(request.request_id().as_str(), "req-1");
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert!(matches!(
            orchestrator.approve_pending(1_001).unwrap(),
            OrchestrationEvent::StepFinished {
                outcome: StepTerminalOutcome::Verified,
                ..
            }
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert!(matches!(
            orchestrator.advance(1_002).unwrap(),
            OrchestrationEvent::PlanFinished(TruthfulPlanSummary {
                outcome: PlanOutcome::Completed,
                ..
            })
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn denial_blocks_a_dependent_step_and_starts_nothing() {
        let calls = Arc::new(AtomicUsize::new(0));
        let plan = ValidatedPlan::new(
            PlanId::parse("plan-deny".into()).unwrap(),
            id("correlation-deny"),
            vec![
                uname_step("step-1", "req-1", &[]),
                uname_step("step-2", "req-2", &["step-1"]),
            ],
        )
        .unwrap();
        let mut orchestrator = PlanOrchestrator::new(
            uname_engine(crate::PolicyDecision::Ask, 0, calls.clone()),
            plan,
        );
        assert!(matches!(
            orchestrator.advance(1_000).unwrap(),
            OrchestrationEvent::ApprovalRequired { .. }
        ));
        assert!(matches!(
            orchestrator.deny_pending(1_001).unwrap(),
            OrchestrationEvent::StepFinished {
                outcome: StepTerminalOutcome::Denied,
                ..
            }
        ));
        assert!(matches!(
            orchestrator.advance(1_002).unwrap(),
            OrchestrationEvent::StepFinished {
                outcome: StepTerminalOutcome::Blocked,
                ..
            }
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn cancellation_consumes_pending_approval_and_starts_nothing() {
        let calls = Arc::new(AtomicUsize::new(0));
        let plan = ValidatedPlan::new(
            PlanId::parse("plan-cancel".into()).unwrap(),
            id("correlation-cancel"),
            vec![uname_step("step-1", "req-1", &[])],
        )
        .unwrap();
        let mut orchestrator = PlanOrchestrator::new(
            uname_engine(crate::PolicyDecision::Ask, 0, calls.clone()),
            plan,
        );
        orchestrator.advance(1_000).unwrap();
        assert!(matches!(
            orchestrator.cancel(1_001).unwrap(),
            OrchestrationEvent::StepFinished {
                outcome: StepTerminalOutcome::CancelledBeforeStart,
                ..
            }
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(
            orchestrator.approve_pending(1_002),
            Err(OrchestrationError::NoApprovalPending)
        );
    }

    #[test]
    fn failed_verification_never_becomes_plan_success() {
        let calls = Arc::new(AtomicUsize::new(0));
        let plan = ValidatedPlan::new(
            PlanId::parse("plan-verify".into()).unwrap(),
            id("correlation-verify"),
            vec![uname_step("step-1", "req-1", &[])],
        )
        .unwrap();
        let mut orchestrator = PlanOrchestrator::new(
            uname_engine(crate::PolicyDecision::Allow, 1, calls.clone()),
            plan,
        );
        assert!(matches!(
            orchestrator.advance(1_000).unwrap(),
            OrchestrationEvent::StepFinished {
                outcome: StepTerminalOutcome::VerificationFailed,
                ..
            }
        ));
        assert!(matches!(
            orchestrator.advance(1_001).unwrap(),
            OrchestrationEvent::PlanFinished(TruthfulPlanSummary {
                outcome: PlanOutcome::Blocked,
                verified_steps: 0,
                ..
            })
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}
