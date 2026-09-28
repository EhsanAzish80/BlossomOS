#![cfg(target_os = "linux")]

use blossom_cli::{ApprovalChoice, Clock, Interaction, run_file_read};
use blossom_core::{CommandSpec, ExecutionResult, Executor, ExecutorError, RequestId};
use std::fs;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Debug)]
struct RejectingExecutor(Arc<AtomicUsize>);
impl Executor for RejectingExecutor {
    fn execute(&mut self, _: &CommandSpec) -> Result<ExecutionResult, ExecutorError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Err(ExecutorError::Rejected)
    }
}
struct ScriptedInteraction {
    interactive: bool,
    choice: ApprovalChoice,
    preview: String,
}
impl Interaction for ScriptedInteraction {
    fn is_interactive(&self) -> bool {
        self.interactive
    }
    fn choose(&mut self, preview: &str) -> ApprovalChoice {
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

fn fixture() -> (std::path::PathBuf, std::path::PathBuf) {
    let root = std::env::temp_dir().join(format!("blossom-cli-file-read-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir(&root).unwrap();
    let path = root.join("selected.txt");
    fs::write(&path, "hello\n\u{1b}[31mnot-terminal-code").unwrap();
    (root, path)
}

fn run(
    interactive: bool,
    choice: ApprovalChoice,
    times: Vec<u64>,
) -> (
    blossom_cli::RunOutcome,
    ScriptedInteraction,
    usize,
    std::path::PathBuf,
) {
    let (root, path) = fixture();
    let executions = Arc::new(AtomicUsize::new(0));
    let mut interaction = ScriptedInteraction {
        interactive,
        choice,
        preview: String::new(),
    };
    let outcome = run_file_read(
        RejectingExecutor(Arc::clone(&executions)),
        path.to_str().unwrap(),
        &mut interaction,
        &mut ScriptedClock(times),
        RequestId::parse("file-read-flow".into()).unwrap(),
    );
    (
        outcome,
        interaction,
        executions.load(Ordering::SeqCst),
        root,
    )
}

#[test]
fn approval_uses_resolved_descriptor_and_never_the_executor() {
    let (outcome, interaction, executions, root) =
        run(true, ApprovalChoice::ApproveOnce, vec![1_000, 1_001]);
    assert_eq!(outcome.exit_code, 0);
    assert_eq!(executions, 0);
    assert!(interaction.preview.contains("Selected identity: device="));
    assert!(
        outcome
            .result
            .unwrap()
            .contains(r#"hello\n\u001b[31mnot-terminal-code"#)
    );
    assert!(!outcome.activity.contains("not-terminal-code"));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn denial_cancel_expiry_and_noninteractive_never_execute() {
    for (interactive, choice, times, exit) in [
        (true, ApprovalChoice::Deny, vec![1_000, 1_001], 2),
        (true, ApprovalChoice::Cancel, vec![1_000, 1_001], 2),
        (false, ApprovalChoice::ApproveOnce, vec![1_000, 1_001], 2),
        (true, ApprovalChoice::ApproveOnce, vec![1_000, 31_001], 3),
    ] {
        let (outcome, _, executions, root) = run(interactive, choice, times);
        assert_eq!(outcome.exit_code, exit);
        assert_eq!(executions, 0);
        assert!(outcome.result.is_none());
        let _ = fs::remove_dir_all(root);
    }
}
