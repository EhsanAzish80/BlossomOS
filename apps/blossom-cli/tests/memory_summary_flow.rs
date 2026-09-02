use blossom_cli::{Clock, run_memory_summary};
#[cfg(target_os = "linux")]
use blossom_core::ProcMeminfoReader;
use blossom_core::{
    CommandSpec, ExecutionResult, Executor, ExecutorError, MemorySummary, MemorySummaryError,
    MemorySummaryProvider, RequestId, parse_proc_meminfo,
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

struct FixtureProvider(Option<MemorySummary>);

impl MemorySummaryProvider for FixtureProvider {
    fn read_memory_summary(&mut self) -> Result<MemorySummary, MemorySummaryError> {
        self.0.take().ok_or(MemorySummaryError::ReadFailed)
    }
}

#[test]
fn allowed_native_read_returns_summary_and_never_executes() {
    let summary = parse_proc_meminfo(
        b"MemTotal: 16777216 kB\nMemAvailable: 8388608 kB\nCached: 123456 kB\nSwapTotal: 4194304 kB\nSwapFree: 2097152 kB\n",
    )
    .expect("parse memory fixture");
    let calls = Arc::new(AtomicUsize::new(0));
    let executor = RejectingExecutor {
        calls: Arc::clone(&calls),
    };
    let request_id = RequestId::parse("memory-flow".into()).expect("valid request id");
    let outcome = run_memory_summary(
        executor,
        FixtureProvider(Some(summary)),
        &mut FixedClock,
        request_id,
    );
    assert_eq!(outcome.exit_code, 0);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let result = outcome.result.expect("memory summary should be displayed");
    assert!(result.contains("Total: 16.00 GiB"));
    assert!(result.contains("Available: 8.00 GiB"));
    assert!(result.contains("Swap total: 4.00 GiB"));
    assert!(!result.contains("Cached"));
    assert!(outcome.activity.contains("policy Allow"));
    assert!(
        outcome
            .activity
            .contains("started native read of memory.summary")
    );
    assert!(outcome.activity.contains("verification: succeeded=true"));
    assert!(!outcome.activity.contains("17179869184"));
}

#[cfg(target_os = "linux")]
#[test]
fn target_linux_reads_real_proc_meminfo_without_executor() {
    let calls = Arc::new(AtomicUsize::new(0));
    let executor = RejectingExecutor {
        calls: Arc::clone(&calls),
    };
    let request_id = RequestId::parse("linux-memory".into()).expect("valid request id");
    let outcome = run_memory_summary(
        executor,
        ProcMeminfoReader::default(),
        &mut FixedClock,
        request_id,
    );
    assert_eq!(outcome.exit_code, 0);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert!(outcome.result.is_some());
    assert!(outcome.activity.contains("/proc/meminfo"));
    assert!(outcome.activity.contains("verification: succeeded=true"));
}
