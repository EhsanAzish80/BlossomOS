use blossom_cli::{Clock, run_uptime};
#[cfg(target_os = "linux")]
use blossom_core::ProcUptimeReader;
use blossom_core::{
    CommandSpec, ExecutionResult, Executor, ExecutorError, RequestId, SystemUptime, UptimeError,
    UptimeProvider, parse_proc_uptime,
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

struct FixtureProvider(Option<SystemUptime>);

impl UptimeProvider for FixtureProvider {
    fn read_uptime(&mut self) -> Result<SystemUptime, UptimeError> {
        self.0.take().ok_or(UptimeError::ReadFailed)
    }
}

#[test]
fn allowed_native_read_returns_uptime_and_never_executes() {
    let uptime = parse_proc_uptime(b"93784.25 123456.00\n").expect("parse uptime fixture");
    let calls = Arc::new(AtomicUsize::new(0));
    let executor = RejectingExecutor {
        calls: Arc::clone(&calls),
    };
    let request_id = RequestId::parse("uptime-flow".into()).expect("valid request id");
    let outcome = run_uptime(
        executor,
        FixtureProvider(Some(uptime)),
        &mut FixedClock,
        request_id,
    );
    assert_eq!(outcome.exit_code, 0);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let result = outcome.result.expect("uptime result should be displayed");
    assert!(result.contains("1 days 02:03:04.250"));
    assert!(result.contains("/proc/uptime"));
    assert!(!result.contains("123456"));
    assert!(outcome.activity.contains("policy Allow"));
    assert!(outcome.activity.contains("started native read of uptime"));
    assert!(outcome.activity.contains("verification: succeeded=true"));
    assert!(!outcome.activity.contains("93784"));
}

#[cfg(target_os = "linux")]
#[test]
fn target_linux_reads_real_proc_uptime_without_executor() {
    let calls = Arc::new(AtomicUsize::new(0));
    let executor = RejectingExecutor {
        calls: Arc::clone(&calls),
    };
    let request_id = RequestId::parse("linux-uptime".into()).expect("valid request id");
    let outcome = run_uptime(
        executor,
        ProcUptimeReader::default(),
        &mut FixedClock,
        request_id,
    );
    assert_eq!(outcome.exit_code, 0);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert!(outcome.result.is_some());
    assert!(outcome.activity.contains("/proc/uptime"));
    assert!(outcome.activity.contains("verification: succeeded=true"));
}
