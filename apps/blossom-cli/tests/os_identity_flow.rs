use blossom_cli::{Clock, run_os_identity};
#[cfg(target_os = "linux")]
use blossom_core::OsReleaseReader;
use blossom_core::{
    CommandSpec, ExecutionResult, Executor, ExecutorError, OsIdentity, OsIdentityError,
    OsIdentityProvider, OsReleaseSource, RequestId, parse_os_release,
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

struct FixtureProvider(Option<OsIdentity>);

impl OsIdentityProvider for FixtureProvider {
    fn read_os_identity(&mut self) -> Result<OsIdentity, OsIdentityError> {
        self.0.take().ok_or(OsIdentityError::ReadFailed)
    }
}

#[test]
fn allowed_native_read_returns_only_allowlisted_fields_and_never_executes() {
    let identity = parse_os_release(
        OsReleaseSource::EtcOsRelease,
        b"ID=arch\nNAME=\"Arch Linux\"\nPRETTY_NAME=\"Arch Linux\"\nBUILD_ID=rolling\nHOME_URL=\"https://archlinux.org/\"\n",
    )
    .expect("parse fixture");
    let calls = Arc::new(AtomicUsize::new(0));
    let executor = RejectingExecutor {
        calls: Arc::clone(&calls),
    };
    let request_id = RequestId::parse("os-flow".into()).expect("valid request id");
    let outcome = run_os_identity(
        executor,
        FixtureProvider(Some(identity)),
        &mut FixedClock,
        request_id,
    );
    assert_eq!(outcome.exit_code, 0);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let result = outcome.result.expect("identity result should be displayed");
    assert!(result.contains("ID: arch"));
    assert!(result.contains("NAME: Arch Linux"));
    assert!(result.contains("BUILD_ID: rolling"));
    assert!(!result.contains("HOME_URL"));
    assert!(outcome.activity.contains("policy Allow"));
    assert!(
        outcome
            .activity
            .contains("started native read of os.identity")
    );
    assert!(outcome.activity.contains("verification: succeeded=true"));
    assert!(!outcome.activity.contains("Arch Linux"));
}

#[cfg(target_os = "linux")]
#[test]
fn target_linux_reads_real_os_release_without_executor() {
    let calls = Arc::new(AtomicUsize::new(0));
    let executor = RejectingExecutor {
        calls: Arc::clone(&calls),
    };
    let request_id = RequestId::parse("linux-os-identity".into()).expect("valid request id");
    let outcome = run_os_identity(
        executor,
        OsReleaseReader::default(),
        &mut FixedClock,
        request_id,
    );
    assert_eq!(outcome.exit_code, 0);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert!(outcome.result.is_some());
    assert!(outcome.activity.contains("/etc/os-release"));
    assert!(outcome.activity.contains("verification: succeeded=true"));
}
