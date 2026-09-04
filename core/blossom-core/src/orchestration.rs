//! Closed Phase 5 plan and lifecycle types.
//!
//! These types contain no executor and accept no caller-supplied capability.
//! Capabilities are projected from the existing typed request registry.

use crate::{Capability, PolicyEngine, RequestId, ToolRequest};
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
}
