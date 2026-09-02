use super::{CommandSpec, ExecutionResult, Executor, ExecutorError};
use std::ffi::OsString;
use std::path::{Path, PathBuf};

const MAX_TIMEOUT_MS: u64 = 30_000;
const MAX_OUTPUT_BYTES: usize = 1024 * 1024;
const MAX_ENVIRONMENT_ENTRIES: usize = 16;
const MAX_ENVIRONMENT_VALUE_BYTES: usize = 4_096;

#[derive(Clone, Debug)]
pub struct BubblewrapExecutor {
    bubblewrap_path: PathBuf,
}

impl BubblewrapExecutor {
    pub fn new(bubblewrap_path: impl Into<PathBuf>) -> Self {
        Self {
            bubblewrap_path: bubblewrap_path.into(),
        }
    }

    pub fn phase1_default() -> Self {
        Self::new("/usr/bin/bwrap")
    }

    pub fn bubblewrap_path(&self) -> &Path {
        &self.bubblewrap_path
    }
}

pub fn validate_phase1_command(command: &CommandSpec) -> Result<(), ExecutorError> {
    let expected = CommandSpec::system_uname();
    let fixed_command = command.program == expected.program
        && command.arguments == expected.arguments
        && command.working_directory == expected.working_directory;
    let bounded = command.timeout_ms > 0
        && command.timeout_ms <= MAX_TIMEOUT_MS
        && command.max_output_bytes > 0
        && command.max_output_bytes <= MAX_OUTPUT_BYTES;
    let environment_is_valid = command.environment.len() <= MAX_ENVIRONMENT_ENTRIES
        && command.environment.iter().all(|(key, value)| {
            valid_environment_key(key)
                && value.len() <= MAX_ENVIRONMENT_VALUE_BYTES
                && !value.contains('\0')
        });
    if fixed_command && bounded && environment_is_valid && !command.network_allowed {
        Ok(())
    } else {
        Err(ExecutorError::Rejected)
    }
}

pub fn bubblewrap_arguments(command: &CommandSpec) -> Result<Vec<OsString>, ExecutorError> {
    validate_phase1_command(command)?;
    let mut arguments = [
        "--die-with-parent",
        "--new-session",
        "--unshare-all",
        "--disable-userns",
        "--cap-drop",
        "ALL",
        "--clearenv",
        "--ro-bind",
        "/usr",
        "/usr",
        "--symlink",
        "usr/lib",
        "/lib",
        "--symlink",
        "usr/lib64",
        "/lib64",
        "--proc",
        "/proc",
        "--dev",
        "/dev",
        "--tmpfs",
        "/tmp",
        "--chdir",
        "/",
    ]
    .into_iter()
    .map(OsString::from)
    .collect::<Vec<_>>();
    for (key, value) in &command.environment {
        arguments.push("--setenv".into());
        arguments.push(key.into());
        arguments.push(value.into());
    }
    arguments.push("--".into());
    arguments.push(command.program.as_os_str().into());
    arguments.extend(command.arguments.iter().map(OsString::from));
    Ok(arguments)
}

fn valid_environment_key(key: &str) -> bool {
    let mut bytes = key.bytes();
    matches!(bytes.next(), Some(first) if first.is_ascii_uppercase() || first == b'_')
        && bytes.all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
}

#[cfg(target_os = "linux")]
impl Executor for BubblewrapExecutor {
    fn execute(&mut self, command: &CommandSpec) -> Result<ExecutionResult, ExecutorError> {
        linux::execute(self.bubblewrap_path(), command)
    }
}

#[cfg(not(target_os = "linux"))]
impl Executor for BubblewrapExecutor {
    fn execute(&mut self, _command: &CommandSpec) -> Result<ExecutionResult, ExecutorError> {
        Err(ExecutorError::Unavailable)
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use std::io::Read;
    use std::process::{Command, Stdio};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::thread::{self, JoinHandle};
    use std::time::{Duration, Instant};

    pub(super) fn execute(
        bubblewrap_path: &Path,
        command: &CommandSpec,
    ) -> Result<ExecutionResult, ExecutorError> {
        let arguments = bubblewrap_arguments(command)?;
        let mut child = Command::new(bubblewrap_path)
            .args(arguments)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env_clear()
            .spawn()
            .map_err(|_| ExecutorError::SpawnFailed)?;

        let stdout = child.stdout.take().ok_or(ExecutorError::SpawnFailed)?;
        let stderr = child.stderr.take().ok_or(ExecutorError::SpawnFailed)?;
        let remaining = Arc::new(AtomicUsize::new(command.max_output_bytes));
        let truncated = Arc::new(AtomicBool::new(false));
        let stdout_reader = capture(stdout, Arc::clone(&remaining), Arc::clone(&truncated));
        let stderr_reader = capture(stderr, remaining, Arc::clone(&truncated));

        let deadline = Instant::now() + Duration::from_millis(command.timeout_ms);
        let (status, timed_out) = loop {
            if truncated.load(Ordering::Acquire) {
                let _ = child.kill();
                break (child.wait().map_err(|_| ExecutorError::Failed)?, false);
            }
            if let Some(status) = child.try_wait().map_err(|_| ExecutorError::Failed)? {
                break (status, false);
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                break (child.wait().map_err(|_| ExecutorError::Failed)?, true);
            }
            thread::sleep(Duration::from_millis(2));
        };

        let stdout = join_reader(stdout_reader)?;
        let stderr = join_reader(stderr_reader)?;
        if timed_out {
            return Err(ExecutorError::Timeout);
        }
        Ok(ExecutionResult {
            exit_code: status.code(),
            stdout,
            stderr,
            timed_out: false,
            output_truncated: truncated.load(Ordering::Acquire),
        })
    }

    fn capture<R>(
        mut reader: R,
        remaining: Arc<AtomicUsize>,
        truncated: Arc<AtomicBool>,
    ) -> JoinHandle<Result<Vec<u8>, ExecutorError>>
    where
        R: Read + Send + 'static,
    {
        thread::spawn(move || {
            let mut output = Vec::new();
            let mut buffer = [0_u8; 4_096];
            loop {
                let read = reader
                    .read(&mut buffer)
                    .map_err(|_| ExecutorError::Failed)?;
                if read == 0 {
                    break;
                }
                let reserved = reserve(&remaining, read);
                output.extend_from_slice(&buffer[..reserved]);
                if reserved < read {
                    truncated.store(true, Ordering::Release);
                }
            }
            Ok(output)
        })
    }

    fn reserve(remaining: &AtomicUsize, requested: usize) -> usize {
        let mut current = remaining.load(Ordering::Acquire);
        loop {
            let reserved = current.min(requested);
            match remaining.compare_exchange_weak(
                current,
                current - reserved,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return reserved,
                Err(observed) => current = observed,
            }
        }
    }

    fn join_reader(
        reader: JoinHandle<Result<Vec<u8>, ExecutorError>>,
    ) -> Result<Vec<u8>, ExecutorError> {
        reader.join().map_err(|_| ExecutorError::Failed)?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_a_networkless_read_only_namespace() {
        let arguments = bubblewrap_arguments(&CommandSpec::system_uname())
            .expect("fixed diagnostic should be accepted");
        let arguments = arguments
            .iter()
            .map(|argument| argument.to_string_lossy())
            .collect::<Vec<_>>();
        assert!(arguments.contains(&"--unshare-all".into()));
        assert!(arguments.contains(&"--disable-userns".into()));
        assert!(arguments.contains(&"--clearenv".into()));
        assert!(arguments.contains(&"--ro-bind".into()));
        assert!(!arguments.contains(&"--share-net".into()));
        assert!(arguments.ends_with(&["--".into(), "/usr/bin/uname".into(), "-s".into()]));
    }

    #[test]
    fn rejects_any_expansion_of_the_fixed_diagnostic() {
        let mut command = CommandSpec::system_uname();
        command.arguments.push("-a".into());
        assert_eq!(
            validate_phase1_command(&command),
            Err(ExecutorError::Rejected)
        );

        let mut command = CommandSpec::system_uname();
        command.network_allowed = true;
        assert_eq!(
            validate_phase1_command(&command),
            Err(ExecutorError::Rejected)
        );

        let mut command = CommandSpec::system_uname();
        command
            .environment
            .insert("lowercase".into(), "value".into());
        assert_eq!(
            validate_phase1_command(&command),
            Err(ExecutorError::Rejected)
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn executes_the_fixed_diagnostic_in_bubblewrap() {
        assert!(
            Path::new("/usr/bin/bwrap").is_file(),
            "CI must install bubblewrap"
        );
        let mut executor = BubblewrapExecutor::phase1_default();
        let result = executor
            .execute(&CommandSpec::system_uname())
            .expect("sandboxed uname should execute");
        assert_eq!(result.exit_code, Some(0));
        assert!(!result.stdout.is_empty());
        assert!(!result.output_truncated);
    }
}
