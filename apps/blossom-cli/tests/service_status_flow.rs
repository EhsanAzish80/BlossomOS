use blossom_cli::{ApprovalChoice, Clock, Interaction, run_service_status};
use blossom_core::{
    CommandSpec, ExecutionResult, Executor, ExecutorError, RequestId, SYSTEMD_DESTINATION,
    SYSTEMD_MANAGER_INTERFACE, SYSTEMD_UNIT_INTERFACE, ServiceStatus, ServiceStatusError,
    ServiceStatusProvider,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug)]
struct RejectingExecutor(Arc<AtomicUsize>);
impl Executor for RejectingExecutor {
    fn execute(&mut self, _: &CommandSpec) -> Result<ExecutionResult, ExecutorError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Err(ExecutorError::Rejected)
    }
}

struct FixtureProvider {
    calls: Arc<AtomicUsize>,
}
impl ServiceStatusProvider for FixtureProvider {
    fn read_status(&mut self, unit: &str) -> Result<ServiceStatus, ServiceStatusError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(ServiceStatus {
            requested_unit: unit.into(),
            scope: "system".into(),
            canonical_unit: "canonical-private.service".into(),
            load_state: "loaded".into(),
            active_state: "active".into(),
            sub_state: "future-state".into(),
            destination: SYSTEMD_DESTINATION.into(),
            manager_interface: SYSTEMD_MANAGER_INTERFACE.into(),
            unit_interface: SYSTEMD_UNIT_INTERFACE.into(),
        })
    }
}

struct ScriptedInteraction {
    interactive: bool,
    choice: ApprovalChoice,
    calls: usize,
    preview: String,
}
impl Interaction for ScriptedInteraction {
    fn is_interactive(&self) -> bool {
        self.interactive
    }
    fn choose(&mut self, preview: &str) -> ApprovalChoice {
        self.calls += 1;
        self.preview = preview.into();
        self.choice
    }
}

struct ScriptedClock(Vec<u64>);
impl Clock for ScriptedClock {
    fn now_ms(&mut self) -> u64 {
        self.0.remove(0)
    }
}

fn run(
    interactive: bool,
    choice: ApprovalChoice,
    times: Vec<u64>,
) -> (blossom_cli::RunOutcome, ScriptedInteraction, usize, usize) {
    let executor_calls = Arc::new(AtomicUsize::new(0));
    let provider_calls = Arc::new(AtomicUsize::new(0));
    let mut interaction = ScriptedInteraction {
        interactive,
        choice,
        calls: 0,
        preview: String::new(),
    };
    let outcome = run_service_status(
        RejectingExecutor(Arc::clone(&executor_calls)),
        FixtureProvider {
            calls: Arc::clone(&provider_calls),
        },
        &mut interaction,
        &mut ScriptedClock(times),
        RequestId::parse("service-status-flow".into()).expect("valid id"),
        "private-work.service".into(),
    );
    (
        outcome,
        interaction,
        executor_calls.load(Ordering::SeqCst),
        provider_calls.load(Ordering::SeqCst),
    )
}

#[test]
fn exact_preview_and_once_only_approval_gate_the_fixed_provider() {
    let (outcome, interaction, executor_calls, provider_calls) =
        run(true, ApprovalChoice::ApproveOnce, vec![1_000, 1_001]);
    assert_eq!(outcome.exit_code, 0);
    assert_eq!(executor_calls, 0);
    assert_eq!(provider_calls, 1);
    assert_eq!(interaction.calls, 1);
    assert!(
        interaction
            .preview
            .contains("Capability: services.read:status")
    );
    assert!(
        interaction
            .preview
            .contains("Exact unit: private-work.service")
    );
    assert!(interaction.preview.contains("Approval: once only"));
    assert!(
        interaction
            .preview
            .contains("Calls: GetUnit, then Id, LoadState, ActiveState, SubState only")
    );
    assert!(
        interaction
            .preview
            .contains("Listing, loading, GetAll, mutation")
    );
    let result = outcome.result.expect("approved result");
    assert!(result.contains("Requested unit: private-work.service"));
    assert!(result.contains("Canonical unit: canonical-private.service"));
    assert!(result.contains("Sub-state: future-state"));
    assert!(result.contains("observed state may change"));
    assert!(outcome.activity.contains("policy Ask"));
    assert!(outcome.activity.contains("approved once"));
    assert!(outcome.activity.contains("verification: succeeded=true"));
    assert!(!outcome.activity.contains("private-work.service"));
    assert!(!outcome.activity.contains("canonical-private.service"));
    assert!(!outcome.activity.contains("future-state"));
}

#[test]
fn denial_cancellation_expiry_and_noninteractive_mode_make_zero_provider_calls() {
    for (interactive, choice, times, exit_code, activity) in [
        (true, ApprovalChoice::Deny, vec![1_000, 1_001], 2, "denied"),
        (
            true,
            ApprovalChoice::Cancel,
            vec![1_000, 1_001],
            2,
            "cancelled",
        ),
        (
            false,
            ApprovalChoice::ApproveOnce,
            vec![1_000, 1_001],
            2,
            "denied",
        ),
        (
            true,
            ApprovalChoice::ApproveOnce,
            vec![1_000, 31_001],
            3,
            "expired",
        ),
    ] {
        let (outcome, interaction, executor_calls, provider_calls) =
            run(interactive, choice, times);
        assert_eq!(outcome.exit_code, exit_code);
        assert_eq!(executor_calls, 0);
        assert_eq!(provider_calls, 0);
        assert!(outcome.result.is_none());
        assert!(outcome.activity.contains(activity));
        assert_eq!(interaction.calls, usize::from(interactive));
        assert!(!outcome.activity.contains("started native read"));
    }
}
