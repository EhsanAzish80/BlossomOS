use crate::executor::ExecutionResult;
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Verification {
    pub succeeded: bool,
    pub reason: VerificationReason,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum VerificationReason {
    ValidSystemName,
    TimedOut,
    OutputTruncated,
    NonZeroExit,
    EmptyOutput,
}

pub fn verify_execution(result: &ExecutionResult) -> Verification {
    let reason = if result.timed_out {
        VerificationReason::TimedOut
    } else if result.output_truncated {
        VerificationReason::OutputTruncated
    } else if result.exit_code != Some(0) {
        VerificationReason::NonZeroExit
    } else if result.stdout.iter().all(u8::is_ascii_whitespace) {
        VerificationReason::EmptyOutput
    } else {
        VerificationReason::ValidSystemName
    };
    Verification {
        succeeded: reason == VerificationReason::ValidSystemName,
        reason,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requires_successful_nonempty_bounded_output() {
        let valid = ExecutionResult {
            exit_code: Some(0),
            stdout: b"Linux\n".to_vec(),
            stderr: Vec::new(),
            timed_out: false,
            output_truncated: false,
        };
        assert!(verify_execution(&valid).succeeded);

        let mut timeout = valid.clone();
        timeout.timed_out = true;
        assert!(!verify_execution(&timeout).succeeded);

        let mut failed = valid;
        failed.exit_code = Some(1);
        assert!(!verify_execution(&failed).succeeded);
    }
}
