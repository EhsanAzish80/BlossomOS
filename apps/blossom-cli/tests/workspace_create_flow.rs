use blossom_cli::{ApprovalChoice, Clock, Interaction, run_workspace_create};
#[cfg(all(target_os = "linux", target_env = "gnu"))]
use blossom_core::AtomicWorkspaceFileCreator;
use blossom_core::{
    CommandSpec, DirectoryIdentity, ExecutionResult, Executor, ExecutorError, RequestId,
    WORKSPACE_FILE_MODE, WorkspaceCreateError, WorkspaceCreateProvider, WorkspaceCreateSelection,
    WorkspaceCreateState, WorkspaceFileCreated,
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
    selection: WorkspaceCreateSelection,
    creates: Arc<AtomicUsize>,
    state: WorkspaceCreateState,
}
impl WorkspaceCreateProvider for FixtureProvider {
    fn selection(&self) -> &WorkspaceCreateSelection {
        &self.selection
    }
    fn create_selected_file(
        &mut self,
        expected: &WorkspaceCreateSelection,
    ) -> Result<WorkspaceFileCreated, WorkspaceCreateError> {
        self.creates.fetch_add(1, Ordering::SeqCst);
        if expected != &self.selection {
            return Err(WorkspaceCreateError::IdentityChanged);
        }
        Ok(WorkspaceFileCreated {
            workspace_root: expected.workspace_root.clone(),
            relative_destination: expected.relative_destination.clone(),
            root_identity: expected.root_identity.clone(),
            parent_identity: expected.parent_identity.clone(),
            created_device: expected.parent_identity.device,
            created_inode: 99,
            source_bytes: expected.content.len(),
            source_sha256: expected.content_sha256.clone(),
            mode: WORKSPACE_FILE_MODE,
            state: self.state,
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

fn selection() -> WorkspaceCreateSelection {
    let content = "hello\n\u{1b}[31mnot-terminal-code".to_string();
    let content_sha256 = Sha256::digest(content.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    WorkspaceCreateSelection {
        workspace_root: "/home/user/workspace".into(),
        root_identity: DirectoryIdentity {
            device: 10,
            inode: 20,
        },
        parent_identity: DirectoryIdentity {
            device: 10,
            inode: 21,
        },
        relative_destination: "docs/new.txt".into(),
        content,
        content_sha256,
        mode: WORKSPACE_FILE_MODE,
    }
}

fn run(
    interactive: bool,
    choice: ApprovalChoice,
    times: Vec<u64>,
    state: WorkspaceCreateState,
) -> (blossom_cli::RunOutcome, ScriptedInteraction, usize, usize) {
    let creates = Arc::new(AtomicUsize::new(0));
    let executions = Arc::new(AtomicUsize::new(0));
    let provider = FixtureProvider {
        selection: selection(),
        creates: Arc::clone(&creates),
        state,
    };
    let mut interaction = ScriptedInteraction {
        interactive,
        choice,
        calls: 0,
        preview: String::new(),
    };
    let outcome = run_workspace_create(
        RejectingExecutor(Arc::clone(&executions)),
        provider,
        &mut interaction,
        &mut ScriptedClock(times),
        RequestId::parse("workspace-create-flow".into()).expect("id"),
    );
    (
        outcome,
        interaction,
        creates.load(Ordering::SeqCst),
        executions.load(Ordering::SeqCst),
    )
}

#[test]
fn exact_preview_and_once_only_approval_create_a_verified_result() {
    let (outcome, interaction, creates, executions) = run(
        true,
        ApprovalChoice::ApproveOnce,
        vec![1_000, 1_001],
        WorkspaceCreateState::DurableCreated,
    );
    assert_eq!(outcome.exit_code, 0);
    assert_eq!(creates, 1);
    assert_eq!(executions, 0);
    assert!(
        interaction
            .preview
            .contains("Capability: files.write:create")
    );
    assert!(
        interaction
            .preview
            .contains("Workspace root: /home/user/workspace")
    );
    assert!(
        interaction
            .preview
            .contains("Exact relative destination: docs/new.txt")
    );
    assert!(
        interaction
            .preview
            .contains(r#"hello\n\u001b[31mnot-terminal-code"#)
    );
    assert!(outcome.activity.contains("policy Ask"));
    assert!(outcome.activity.contains("verification: succeeded=true"));
    assert!(!outcome.activity.contains("/home/user/workspace"));
    assert!(!outcome.activity.contains("docs/new.txt"));
    assert!(!outcome.activity.contains("not-terminal-code"));
}

#[test]
fn denial_cancellation_expiry_and_noninteractive_mode_create_nothing() {
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
        let (outcome, _, creates, executions) = run(
            interactive,
            choice,
            times,
            WorkspaceCreateState::DurableCreated,
        );
        assert_eq!(outcome.exit_code, exit);
        assert_eq!(creates, 0);
        assert_eq!(executions, 0);
        assert!(outcome.result.is_none());
        assert!(outcome.activity.contains(marker));
        assert!(!outcome.activity.contains("started workspace create"));
    }
}

#[test]
fn published_but_not_durable_is_truthful_non_success() {
    let (outcome, _, creates, _) = run(
        true,
        ApprovalChoice::ApproveOnce,
        vec![1_000, 1_001],
        WorkspaceCreateState::PublishedDurabilityUncertain,
    );
    assert_eq!(creates, 1);
    assert_eq!(outcome.exit_code, 4);
    assert!(
        outcome
            .result
            .expect("partial result")
            .contains("PublishedDurabilityUncertain")
    );
    assert!(outcome.activity.contains("verification: succeeded=false"));
    assert!(
        outcome
            .activity
            .contains("WorkspaceFileDurabilityUncertain")
    );
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
#[test]
fn target_linux_atomically_creates_private_file_without_executor() {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    let root =
        std::env::temp_dir().join(format!("blossom-workspace-create-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir(&root).expect("root");
    let creator = AtomicWorkspaceFileCreator::select(
        root.to_str().expect("path"),
        "nested/new.txt",
        "Linux evidence",
    );
    assert!(creator.is_err(), "missing parent must be rejected");
    fs::create_dir(root.join("nested")).expect("parent");
    let creator = AtomicWorkspaceFileCreator::select(
        root.to_str().expect("path"),
        "nested/new.txt",
        "Linux evidence",
    )
    .expect("selection");
    let executions = Arc::new(AtomicUsize::new(0));
    let mut interaction = ScriptedInteraction {
        interactive: true,
        choice: ApprovalChoice::ApproveOnce,
        calls: 0,
        preview: String::new(),
    };
    let outcome = run_workspace_create(
        RejectingExecutor(Arc::clone(&executions)),
        creator,
        &mut interaction,
        &mut ScriptedClock(vec![1_000, 1_001]),
        RequestId::parse("linux-workspace-create".into()).expect("id"),
    );
    assert_eq!(outcome.exit_code, 0);
    assert_eq!(executions.load(Ordering::SeqCst), 0);
    assert_eq!(
        fs::read_to_string(root.join("nested/new.txt")).expect("created"),
        "Linux evidence"
    );
    assert_eq!(
        fs::metadata(root.join("nested/new.txt"))
            .expect("metadata")
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    assert!(
        fs::read_dir(root.join("nested"))
            .expect("directory")
            .all(|entry| !entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .starts_with(".blossom-tmp-"))
    );
    let _ = fs::remove_dir_all(&root);
}
