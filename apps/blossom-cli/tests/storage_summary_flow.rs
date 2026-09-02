use blossom_cli::{Clock, run_storage_summary};
#[cfg(target_os = "linux")]
use blossom_core::RootStorageReader;
use blossom_core::{
    CommandSpec, ExecutionResult, Executor, ExecutorError, RequestId, StorageSummary,
    StorageSummaryError, StorageSummaryProvider, StorageSummarySource,
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

struct FixtureProvider(Option<StorageSummary>);

impl StorageSummaryProvider for FixtureProvider {
    fn read_storage_summary(&mut self) -> Result<StorageSummary, StorageSummaryError> {
        self.0.take().ok_or(StorageSummaryError::StatFailed)
    }
}

#[test]
fn allowed_native_stat_returns_root_summary_and_never_executes() {
    let summary = StorageSummary {
        source: StorageSummarySource::RootStatvfs,
        resource_path: "/".into(),
        total_bytes: 16 * 1_073_741_824,
        available_bytes: 4 * 1_073_741_824,
    };
    let calls = Arc::new(AtomicUsize::new(0));
    let executor = RejectingExecutor {
        calls: Arc::clone(&calls),
    };
    let request_id = RequestId::parse("storage-flow".into()).expect("valid request id");
    let outcome = run_storage_summary(
        executor,
        FixtureProvider(Some(summary)),
        &mut FixedClock,
        request_id,
    );
    assert_eq!(outcome.exit_code, 0);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let result = outcome.result.expect("storage summary should be displayed");
    assert!(result.contains("Total: 16.00 GiB"));
    assert!(result.contains("Available to this user: 4.00 GiB"));
    assert!(result.contains("Scope: /"));
    assert!(outcome.activity.contains("policy Allow"));
    assert!(
        outcome
            .activity
            .contains("started native read of storage.summary:/")
    );
    assert!(outcome.activity.contains("verification: succeeded=true"));
    assert!(!outcome.activity.contains("17179869184"));
}

#[cfg(target_os = "linux")]
#[test]
fn target_linux_stats_real_root_without_executor() {
    let calls = Arc::new(AtomicUsize::new(0));
    let executor = RejectingExecutor {
        calls: Arc::clone(&calls),
    };
    let request_id = RequestId::parse("linux-storage".into()).expect("valid request id");
    let outcome = run_storage_summary(executor, RootStorageReader, &mut FixedClock, request_id);
    assert_eq!(outcome.exit_code, 0);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert!(outcome.result.is_some());
    assert!(outcome.activity.contains("via statvfs"));
    assert!(outcome.activity.contains("verification: succeeded=true"));
}
