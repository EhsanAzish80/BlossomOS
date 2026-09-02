use blossom_cli::{Clock, run_process_self};
#[cfg(target_os = "linux")]
use blossom_core::NativeProcessSelfReader;
use blossom_core::{
    CommandSpec, ExecutionResult, Executor, ExecutorError, ProcessSelf, ProcessSelfError,
    ProcessSelfProvider, ProcessSelfSource, RequestId,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug)]
struct RejectingExecutor {
    calls: Arc<AtomicUsize>,
}

impl Executor for RejectingExecutor {
    fn execute(&mut self, _command: &CommandSpec) -> Result<ExecutionResult, ExecutorError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Err(ExecutorError::Rejected)
    }
}

struct FixedClock;

impl Clock for FixedClock {
    fn now_ms(&mut self) -> u64 {
        1_000
    }
}

struct FixtureProvider(Option<ProcessSelf>);

impl ProcessSelfProvider for FixtureProvider {
    fn read_process_self(&mut self) -> Result<ProcessSelf, ProcessSelfError> {
        self.0.take().ok_or(ProcessSelfError::InvalidProcessId)
    }
}

#[test]
fn allowed_native_read_returns_minimal_self_identity_and_never_executes() {
    let identity = ProcessSelf {
        source: ProcessSelfSource::NativeProcessIdentity,
        process_id: 42,
        parent_process_id: 7,
        effective_user_id: 1000,
        effective_group_id: 1001,
    };
    let calls = Arc::new(AtomicUsize::new(0));
    let executor = RejectingExecutor {
        calls: Arc::clone(&calls),
    };
    let request_id = RequestId::parse("process-self-flow".into()).expect("valid request id");
    let outcome = run_process_self(
        executor,
        FixtureProvider(Some(identity)),
        &mut FixedClock,
        request_id,
    );
    assert_eq!(outcome.exit_code, 0);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let result = outcome
        .result
        .expect("process identity should be displayed");
    assert!(result.contains("PID: 42"));
    assert!(result.contains("Parent PID: 7"));
    assert!(result.contains("Effective user ID: 1000"));
    assert!(!result.contains("command"));
    assert!(!result.contains("environment"));
    assert!(outcome.activity.contains("policy Allow"));
    assert!(
        outcome
            .activity
            .contains("started native read of process.self")
    );
    assert!(outcome.activity.contains("verification: succeeded=true"));
    assert!(!outcome.activity.contains("PID: 42"));
    assert!(!outcome.activity.contains("1000"));
}

#[cfg(target_os = "linux")]
#[test]
fn target_linux_reads_current_identity_without_executor() {
    let calls = Arc::new(AtomicUsize::new(0));
    let executor = RejectingExecutor {
        calls: Arc::clone(&calls),
    };
    let request_id = RequestId::parse("linux-process-self".into()).expect("valid request id");
    let outcome = run_process_self(
        executor,
        NativeProcessSelfReader,
        &mut FixedClock,
        request_id,
    );
    assert_eq!(outcome.exit_code, 0);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert!(outcome.result.is_some());
    assert!(outcome.activity.contains("native_process_identity"));
    assert!(outcome.activity.contains("verification: succeeded=true"));
}
