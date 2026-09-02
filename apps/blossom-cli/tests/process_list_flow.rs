use blossom_cli::{ApprovalChoice, Clock, Interaction, run_process_list};
#[cfg(target_os = "linux")]
use blossom_core::ProcProcessListReader;
use blossom_core::{
    CommandSpec, ExecutionResult, Executor, ExecutorError, ProcessList, ProcessListEntry,
    ProcessListError, ProcessListProvider, ProcessListSource, ProcessState, RequestId,
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

struct FixtureProvider(Option<ProcessList>);
impl ProcessListProvider for FixtureProvider {
    fn read_process_list(&mut self) -> Result<ProcessList, ProcessListError> {
        self.0.take().ok_or(ProcessListError::ReadDirectoryFailed)
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

fn fixture() -> ProcessList {
    ProcessList {
        source: ProcessListSource::ProcStatusSameEffectiveUser,
        processes: vec![ProcessListEntry {
            process_id: 42,
            name: "blossom".into(),
            state: ProcessState::Sleeping,
        }],
        skipped_entries: 2,
        truncated: false,
    }
}

fn run(
    interactive: bool,
    choice: ApprovalChoice,
    times: Vec<u64>,
) -> (blossom_cli::RunOutcome, ScriptedInteraction, usize) {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut interaction = ScriptedInteraction {
        interactive,
        choice,
        calls: 0,
        preview: String::new(),
    };
    let outcome = run_process_list(
        RejectingExecutor(Arc::clone(&calls)),
        FixtureProvider(Some(fixture())),
        &mut interaction,
        &mut ScriptedClock(times),
        RequestId::parse("process-list-flow".into()).expect("valid id"),
    );
    (outcome, interaction, calls.load(Ordering::SeqCst))
}

#[test]
fn exact_preview_and_once_only_approval_gate_native_read() {
    let (outcome, interaction, calls) = run(true, ApprovalChoice::ApproveOnce, vec![1_000, 1_001]);
    assert_eq!(outcome.exit_code, 0);
    assert_eq!(calls, 0);
    assert_eq!(interaction.calls, 1);
    assert!(
        interaction
            .preview
            .contains("Capability: process.read:list")
    );
    assert!(interaction.preview.contains("Approval: once only"));
    assert!(interaction.preview.contains("Command: none"));
    assert!(
        interaction
            .preview
            .contains("Command line, environment, open files, sockets, and memory: not read")
    );
    let result = outcome.result.expect("approved result");
    assert!(result.contains("42  blossom  Sleeping"));
    assert!(outcome.activity.contains("policy Ask"));
    assert!(outcome.activity.contains("approved once"));
    assert!(outcome.activity.contains("verification: succeeded=true"));
    assert!(!outcome.activity.contains("blossom"));
    assert!(!outcome.activity.contains("PID: 42"));
}

#[test]
fn denial_cancellation_and_noninteractive_mode_start_nothing() {
    for (interactive, choice, expected) in [
        (true, ApprovalChoice::Deny, "denied"),
        (true, ApprovalChoice::Cancel, "cancelled"),
        (false, ApprovalChoice::ApproveOnce, "denied"),
    ] {
        let (outcome, interaction, calls) = run(interactive, choice, vec![1_000, 1_001]);
        assert_eq!(outcome.exit_code, 2);
        assert_eq!(calls, 0);
        assert!(outcome.result.is_none());
        assert!(outcome.activity.contains(expected));
        assert_eq!(interaction.calls, usize::from(interactive));
        assert!(!outcome.activity.contains("started native read"));
    }
}

#[test]
fn expired_approval_starts_nothing() {
    let (outcome, _, calls) = run(true, ApprovalChoice::ApproveOnce, vec![1_000, 31_001]);
    assert_eq!(outcome.exit_code, 3);
    assert_eq!(calls, 0);
    assert!(outcome.result.is_none());
    assert!(
        outcome
            .activity
            .contains("approval rejected (approval token expired)")
    );
    assert!(!outcome.activity.contains("started native read"));
}

#[cfg(target_os = "linux")]
#[test]
fn target_linux_reads_bounded_same_user_processes_without_executor() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut interaction = ScriptedInteraction {
        interactive: true,
        choice: ApprovalChoice::ApproveOnce,
        calls: 0,
        preview: String::new(),
    };
    let outcome = run_process_list(
        RejectingExecutor(Arc::clone(&calls)),
        ProcProcessListReader,
        &mut interaction,
        &mut ScriptedClock(vec![1_000, 1_001]),
        RequestId::parse("linux-process-list".into()).expect("valid id"),
    );
    assert_eq!(outcome.exit_code, 0);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert!(
        outcome
            .result
            .expect("Linux result")
            .contains("Same-user processes")
    );
}
