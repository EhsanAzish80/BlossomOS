use serde::Serialize;
use std::collections::BTreeMap;
use std::fmt;
use std::path::PathBuf;

pub mod bubblewrap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CommandSpec {
    pub program: PathBuf,
    pub arguments: Vec<String>,
    pub working_directory: PathBuf,
    pub environment: BTreeMap<String, String>,
    pub timeout_ms: u64,
    pub max_output_bytes: usize,
    pub network_allowed: bool,
}

impl CommandSpec {
    pub fn system_uname() -> Self {
        Self {
            program: PathBuf::from("/usr/bin/uname"),
            arguments: vec!["-s".into()],
            working_directory: PathBuf::from("/"),
            environment: BTreeMap::new(),
            timeout_ms: 1_000,
            max_output_bytes: 4_096,
            network_allowed: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionResult {
    pub exit_code: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub timed_out: bool,
    pub output_truncated: bool,
}

pub trait Executor {
    fn execute(&mut self, command: &CommandSpec) -> Result<ExecutionResult, ExecutorError>;
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum ExecutorError {
    Unavailable,
    Rejected,
    SpawnFailed,
    Timeout,
    Failed,
}

impl fmt::Display for ExecutorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Unavailable => "executor unavailable",
            Self::Rejected => "executor rejected the command",
            Self::SpawnFailed => "executor could not start the command",
            Self::Timeout => "command timed out",
            Self::Failed => "command failed",
        })
    }
}

impl std::error::Error for ExecutorError {}
