use blossom_cli::{ApprovalChoice, Clock, Interaction, run_fixed_diagnostic};
#[cfg(target_os = "linux")]
use blossom_core::executor::bubblewrap::BubblewrapExecutor;
use blossom_core::{CommandSpec, ExecutionResult, Executor, ExecutorError, RequestId};
use std::collections::VecDeque;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug)]
struct RecordingExecutor {
    calls: Arc<AtomicUsize>,
}

impl Executor for RecordingExecutor {
    fn execute(&mut self, command: &CommandSpec) -> Result<ExecutionResult, ExecutorError> {
        assert_eq!(command, &CommandSpec::system_uname());
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(ExecutionResult {
            exit_code: Some(0),
            stdout: b"Linux\n".to_vec(),
            stderr: Vec::new(),
            timed_out: false,
            output_truncated: false,
        })
    }
}

struct ScriptedInteraction {
    interactive: bool,
    choice: ApprovalChoice,
    prompts: usize,
    last_preview: String,
}

impl Interaction for ScriptedInteraction {
    fn is_interactive(&self) -> bool {
        self.interactive
    }

    fn choose(&mut self, preview: &str) -> ApprovalChoice {
        self.prompts += 1;
        self.last_preview = preview.into();
        self.choice
    }
}

struct ScriptedClock(VecDeque<u64>);

impl Clock for ScriptedClock {
    fn now_ms(&mut self) -> u64 {
        self.0.pop_front().expect("test clock exhausted")
    }
}

fn run(
    interactive: bool,
    choice: ApprovalChoice,
    times: [u64; 2],
) -> (blossom_cli::RunOutcome, usize, String, usize) {
    let calls = Arc::new(AtomicUsize::new(0));
    let executor = RecordingExecutor {
        calls: Arc::clone(&calls),
    };
    let mut interaction = ScriptedInteraction {
        interactive,
        choice,
        prompts: 0,
        last_preview: String::new(),
    };
    let mut clock = ScriptedClock(times.into());
    let request_id = RequestId::parse("integration-1".into()).expect("valid request id");
    let outcome = run_fixed_diagnostic(executor, &mut interaction, &mut clock, request_id);
    (
        outcome,
        calls.load(Ordering::SeqCst),
        interaction.last_preview,
        interaction.prompts,
    )
}

#[test]
fn approved_once_executes_exactly_once_and_reports_verification() {
    let (outcome, calls, preview, prompts) = run(true, ApprovalChoice::ApproveOnce, [1_000, 1_001]);
    assert_eq!(outcome.exit_code, 0);
    assert_eq!(calls, 1);
    assert_eq!(prompts, 1);
    assert!(preview.contains("Command: /usr/bin/uname -s"));
    assert!(preview.contains("Capability: system.read:kernel.identity"));
    assert!(outcome.activity.contains("approved once"));
    assert!(outcome.activity.contains("verification: succeeded=true"));
    assert!(!outcome.activity.contains("token"));
}

#[test]
fn denial_is_audited_and_never_executes() {
    let (outcome, calls, _, _) = run(true, ApprovalChoice::Deny, [1_000, 1_001]);
    assert_eq!(outcome.exit_code, 2);
    assert_eq!(calls, 0);
    assert!(outcome.activity.contains("denied"));
    assert!(!outcome.activity.contains("started /usr/bin/uname"));
}

#[test]
fn cancellation_is_audited_and_never_executes() {
    let (outcome, calls, _, _) = run(true, ApprovalChoice::Cancel, [1_000, 1_001]);
    assert_eq!(outcome.exit_code, 2);
    assert_eq!(calls, 0);
    assert!(outcome.activity.contains("cancelled"));
    assert!(!outcome.activity.contains("started /usr/bin/uname"));
}

#[test]
fn cancellation_remains_audited_after_the_prompt_expires() {
    let (outcome, calls, _, _) = run(true, ApprovalChoice::Cancel, [1_000, 31_001]);
    assert_eq!(outcome.exit_code, 2);
    assert_eq!(calls, 0);
    assert!(
        outcome
            .activity
            .contains("approval rejected (approval token expired)")
    );
    assert!(outcome.activity.contains("cancelled"));
    assert!(!outcome.activity.contains("started /usr/bin/uname"));
}

#[test]
fn expired_approval_is_rejected_and_never_executes() {
    let (outcome, calls, _, _) = run(true, ApprovalChoice::ApproveOnce, [1_000, 31_001]);
    assert_eq!(outcome.exit_code, 3);
    assert_eq!(calls, 0);
    assert!(
        outcome
            .activity
            .contains("approval rejected (approval token expired)")
    );
    assert!(!outcome.activity.contains("started /usr/bin/uname"));
}

#[test]
fn non_interactive_execution_denies_without_prompting_or_execution() {
    let (outcome, calls, preview, prompts) =
        run(false, ApprovalChoice::ApproveOnce, [1_000, 1_001]);
    assert_eq!(outcome.exit_code, 2);
    assert_eq!(calls, 0);
    assert_eq!(prompts, 0);
    assert!(preview.is_empty());
    assert!(outcome.activity.contains("denied"));
}

#[test]
fn binary_with_piped_stdio_denies_before_execution() {
    let output = Command::new(env!("CARGO_BIN_EXE_blossom-cli"))
        .stdin(Stdio::piped())
        .output()
        .expect("CLI should start");
    assert_eq!(output.status.code(), Some(2));
    let stdout = String::from_utf8(output.stdout).expect("CLI output should be UTF-8");
    assert!(stdout.contains("Non-interactive input is denied by default."));
    assert!(stdout.contains("Command: /usr/bin/uname -s"));
    assert!(stdout.contains("awaiting one-time approval"));
    assert!(stdout.contains("denied"));
    assert!(!stdout.contains("started /usr/bin/uname"));
    assert!(!stdout.contains("token"));
}

#[cfg(target_os = "linux")]
#[test]
fn approved_flow_executes_end_to_end_in_real_bubblewrap() {
    assert!(
        std::path::Path::new("/usr/bin/bwrap").is_file(),
        "CI must install bubblewrap"
    );
    let mut interaction = ScriptedInteraction {
        interactive: true,
        choice: ApprovalChoice::ApproveOnce,
        prompts: 0,
        last_preview: String::new(),
    };
    let mut clock = ScriptedClock([1_000, 1_001].into());
    let request_id = RequestId::parse("linux-end-to-end".into()).expect("valid request id");
    let outcome = run_fixed_diagnostic(
        BubblewrapExecutor::phase1_default(),
        &mut interaction,
        &mut clock,
        request_id,
    );
    assert_eq!(outcome.exit_code, 0);
    assert!(outcome.activity.contains("approved once"));
    assert!(outcome.activity.contains("started /usr/bin/uname"));
    assert!(outcome.activity.contains("verification: succeeded=true"));
}
