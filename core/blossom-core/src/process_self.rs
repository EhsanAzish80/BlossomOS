use nix::unistd::{getegid, geteuid, getpid, getppid};
use serde::Serialize;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessSelfSource {
    NativeProcessIdentity,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ProcessSelf {
    pub source: ProcessSelfSource,
    pub process_id: u32,
    pub parent_process_id: u32,
    pub effective_user_id: u32,
    pub effective_group_id: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessSelfError {
    InvalidProcessId,
    IdentifierOverflow,
}

impl fmt::Display for ProcessSelfError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidProcessId => "native process identity returned an invalid process ID",
            Self::IdentifierOverflow => "native process identity exceeds the supported range",
        })
    }
}

impl std::error::Error for ProcessSelfError {}

pub trait ProcessSelfProvider {
    fn read_process_self(&mut self) -> Result<ProcessSelf, ProcessSelfError>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct UnavailableProcessSelfProvider;

impl ProcessSelfProvider for UnavailableProcessSelfProvider {
    fn read_process_self(&mut self) -> Result<ProcessSelf, ProcessSelfError> {
        Err(ProcessSelfError::InvalidProcessId)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct NativeProcessSelfReader;

impl ProcessSelfProvider for NativeProcessSelfReader {
    fn read_process_self(&mut self) -> Result<ProcessSelf, ProcessSelfError> {
        build_process_self(
            getpid().as_raw(),
            getppid().as_raw(),
            u64::from(geteuid().as_raw()),
            u64::from(getegid().as_raw()),
        )
    }
}

fn build_process_self(
    process_id: i32,
    parent_process_id: i32,
    effective_user_id: u64,
    effective_group_id: u64,
) -> Result<ProcessSelf, ProcessSelfError> {
    if process_id <= 0 || parent_process_id < 0 {
        return Err(ProcessSelfError::InvalidProcessId);
    }
    Ok(ProcessSelf {
        source: ProcessSelfSource::NativeProcessIdentity,
        process_id: u32::try_from(process_id).map_err(|_| ProcessSelfError::IdentifierOverflow)?,
        parent_process_id: u32::try_from(parent_process_id)
            .map_err(|_| ProcessSelfError::IdentifierOverflow)?,
        effective_user_id: u32::try_from(effective_user_id)
            .map_err(|_| ProcessSelfError::IdentifierOverflow)?,
        effective_group_id: u32::try_from(effective_group_id)
            .map_err(|_| ProcessSelfError::IdentifierOverflow)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_minimal_self_identity() {
        let identity = build_process_self(42, 7, 1000, 1001).expect("valid identity");
        assert_eq!(identity.process_id, 42);
        assert_eq!(identity.parent_process_id, 7);
        assert_eq!(identity.effective_user_id, 1000);
        assert_eq!(identity.effective_group_id, 1001);
        assert_eq!(identity.source, ProcessSelfSource::NativeProcessIdentity);
    }

    #[test]
    fn rejects_invalid_and_overflowing_identifiers() {
        assert_eq!(
            build_process_self(0, 1, 0, 0),
            Err(ProcessSelfError::InvalidProcessId)
        );
        assert_eq!(
            build_process_self(1, -1, 0, 0),
            Err(ProcessSelfError::InvalidProcessId)
        );
        assert_eq!(
            build_process_self(1, 0, u64::from(u32::MAX) + 1, 0),
            Err(ProcessSelfError::IdentifierOverflow)
        );
    }

    #[test]
    fn reads_current_process_without_proc_or_subprocess() {
        let identity = NativeProcessSelfReader
            .read_process_self()
            .expect("native identity should be available");
        assert_eq!(identity.process_id, std::process::id());
    }
}
