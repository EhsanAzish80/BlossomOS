#![cfg(all(target_os = "linux", target_env = "gnu"))]

use blossom_cli::{ApprovalChoice, Clock, Interaction, run_workspace_create};
use blossom_core::{CommandSpec, ExecutionResult, Executor, ExecutorError, RequestId};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

static FIXTURE_SEQUENCE: AtomicUsize = AtomicUsize::new(0);

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
fn root() -> std::path::PathBuf {
    let sequence = FIXTURE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "blossom-workspace-create-{}-{sequence}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir(&root).unwrap();
    root
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
    let root = root();
    let executions = Arc::new(AtomicUsize::new(0));
    let mut interaction = ScriptedInteraction {
        interactive,
        choice,
        preview: String::new(),
    };
    let outcome = run_workspace_create(
        RejectingExecutor(Arc::clone(&executions)),
        root.to_str().unwrap(),
        "new.txt",
        "hello\nvisible",
        &mut interaction,
        &mut ScriptedClock(times),
        RequestId::parse("workspace-create-flow".into()).unwrap(),
    );
    (
        outcome,
        interaction,
        executions.load(Ordering::SeqCst),
        root,
    )
}

#[test]
fn approval_creates_verified_private_file_without_executor() {
    let (outcome, interaction, executions, root) =
        run(true, ApprovalChoice::ApproveOnce, vec![1_000, 1_001]);
    assert_eq!(outcome.exit_code, 0);
    assert_eq!(executions, 0);
    assert!(interaction.preview.contains("Content SHA-256:"));
    assert_eq!(
        fs::read_to_string(root.join("new.txt")).unwrap(),
        "hello\nvisible"
    );
    assert_eq!(
        fs::metadata(root.join("new.txt"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    assert!(!outcome.activity.contains("hello"));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn denial_cancel_expiry_and_noninteractive_create_nothing() {
    for (interactive, choice, times, exit) in [
        (true, ApprovalChoice::Deny, vec![1_000, 1_001], 2),
        (true, ApprovalChoice::Cancel, vec![1_000, 1_001], 2),
        (false, ApprovalChoice::ApproveOnce, vec![1_000, 1_001], 2),
        (true, ApprovalChoice::ApproveOnce, vec![1_000, 31_001], 3),
    ] {
        let (outcome, _, executions, root) = run(interactive, choice, times);
        assert_eq!(outcome.exit_code, exit);
        assert_eq!(executions, 0);
        assert!(!root.join("new.txt").exists());
        let _ = fs::remove_dir_all(root);
    }
}
