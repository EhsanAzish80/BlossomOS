use blossom_cli::{ApprovalChoice, Clock, Interaction, run_file_read};
#[cfg(target_os = "linux")]
use blossom_core::Openat2FileReader;
use blossom_core::{
    CommandSpec, ExecutionResult, Executor, ExecutorError, FileContent, FileContentProvider,
    FileIdentity, FileReadError, FileSelection, RequestId,
};
use sha2::{Digest, Sha256};
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

struct FixtureProvider {
    selection: FileSelection,
    reads: Arc<AtomicUsize>,
}
impl FileContentProvider for FixtureProvider {
    fn selection(&self) -> &FileSelection {
        &self.selection
    }
    fn read_selected_file(
        &mut self,
        expected: &FileSelection,
    ) -> Result<FileContent, FileReadError> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        if expected != &self.selection {
            return Err(FileReadError::IdentityChanged);
        }
        let content = "hello\n\u{1b}[31mnot-terminal-code".to_string();
        let bytes = content.as_bytes();
        let source_bytes = bytes.len();
        let digest = Sha256::digest(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        Ok(FileContent {
            selection: expected.clone(),
            content,
            source_bytes,
            source_sha256: digest,
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

fn selection() -> FileSelection {
    FileSelection {
        absolute_path: "/home/user/private note.txt".into(),
        identity: FileIdentity {
            device: 10,
            inode: 20,
            size: 28,
            modified_seconds: 1,
            modified_nanoseconds: 2,
            changed_seconds: 3,
            changed_nanoseconds: 4,
        },
    }
}

fn run(
    interactive: bool,
    choice: ApprovalChoice,
    times: Vec<u64>,
) -> (blossom_cli::RunOutcome, ScriptedInteraction, usize, usize) {
    let executor_calls = Arc::new(AtomicUsize::new(0));
    let reads = Arc::new(AtomicUsize::new(0));
    let provider = FixtureProvider {
        selection: selection(),
        reads: Arc::clone(&reads),
    };
    let mut interaction = ScriptedInteraction {
        interactive,
        choice,
        calls: 0,
        preview: String::new(),
    };
    let outcome = run_file_read(
        RejectingExecutor(Arc::clone(&executor_calls)),
        provider,
        &mut interaction,
        &mut ScriptedClock(times),
        RequestId::parse("file-read-flow".into()).expect("id"),
    );
    (
        outcome,
        interaction,
        reads.load(Ordering::SeqCst),
        executor_calls.load(Ordering::SeqCst),
    )
}

#[test]
fn approval_is_bound_to_exact_path_and_identity_and_output_is_terminal_escaped() {
    let (outcome, interaction, reads, executions) =
        run(true, ApprovalChoice::ApproveOnce, vec![1_000, 1_001]);
    assert_eq!(outcome.exit_code, 0);
    assert_eq!(reads, 1);
    assert_eq!(executions, 0);
    assert!(
        interaction
            .preview
            .contains("Exact path: /home/user/private note.txt")
    );
    assert!(interaction.preview.contains("device=10, inode=20"));
    assert!(interaction.preview.contains("Approval: once only"));
    let result = outcome.result.expect("content");
    assert!(result.contains(r#"hello\n\u001b[31mnot-terminal-code"#));
    assert!(!outcome.activity.contains("private note.txt"));
    assert!(!outcome.activity.contains("not-terminal-code"));
    assert!(outcome.activity.contains("policy Ask"));
    assert!(outcome.activity.contains("verification: succeeded=true"));
}

#[test]
fn denial_cancellation_expiry_and_noninteractive_mode_read_nothing() {
    for (interactive, choice, times, exit, marker) in [
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
            "approval token expired",
        ),
    ] {
        let (outcome, _, reads, executions) = run(interactive, choice, times);
        assert_eq!(outcome.exit_code, exit);
        assert_eq!(reads, 0);
        assert_eq!(executions, 0);
        assert!(outcome.result.is_none());
        assert!(outcome.activity.contains(marker));
        assert!(!outcome.activity.contains("started native read"));
    }
}

#[cfg(target_os = "linux")]
#[test]
fn target_linux_reads_real_exact_file_through_openat2_without_executor() {
    use std::fs;
    let root = std::env::temp_dir().join(format!("blossom-cli-file-read-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir(&root).expect("fixture root");
    let path = root.join("selected.txt");
    fs::write(&path, "target Linux evidence").expect("fixture file");
    let reader =
        Openat2FileReader::select(path.to_str().expect("path")).expect("openat2 selection");
    let executor_calls = Arc::new(AtomicUsize::new(0));
    let mut interaction = ScriptedInteraction {
        interactive: true,
        choice: ApprovalChoice::ApproveOnce,
        calls: 0,
        preview: String::new(),
    };
    let outcome = run_file_read(
        RejectingExecutor(Arc::clone(&executor_calls)),
        reader,
        &mut interaction,
        &mut ScriptedClock(vec![1_000, 1_001]),
        RequestId::parse("linux-file-read".into()).expect("id"),
    );
    let _ = fs::remove_dir_all(&root);
    assert_eq!(outcome.exit_code, 0);
    assert_eq!(executor_calls.load(Ordering::SeqCst), 0);
    assert!(
        outcome
            .result
            .expect("content")
            .contains("target Linux evidence")
    );
}
