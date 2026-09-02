use crate::executor::ExecutionResult;
use crate::os_identity::{MAX_OS_RELEASE_BYTES, MAX_OS_RELEASE_VALUE_BYTES, OsIdentity};
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
    ValidOsIdentity,
    InvalidOsIdentityProvenance,
    InvalidOsIdentitySchema,
}

pub fn verify_os_identity(identity: &OsIdentity) -> Verification {
    let provenance_valid = identity.source_path == identity.source.as_path()
        && identity.source_bytes <= MAX_OS_RELEASE_BYTES
        && identity.source_sha256.len() == 64
        && identity
            .source_sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase());
    let fields = [
        &identity.id,
        &identity.name,
        &identity.pretty_name,
        &identity.version_id,
        &identity.version_codename,
        &identity.build_id,
        &identity.variant_id,
    ];
    let schema_valid = fields.iter().all(|field| {
        field.as_ref().is_none_or(|value| {
            value.len() <= MAX_OS_RELEASE_VALUE_BYTES
                && !value.contains('\0')
                && !value.chars().any(char::is_control)
        })
    });
    let reason = if !provenance_valid {
        VerificationReason::InvalidOsIdentityProvenance
    } else if !schema_valid {
        VerificationReason::InvalidOsIdentitySchema
    } else {
        VerificationReason::ValidOsIdentity
    };
    Verification {
        succeeded: reason == VerificationReason::ValidOsIdentity,
        reason,
    }
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

    #[test]
    fn verifies_os_identity_schema_and_provenance_without_io() {
        let mut identity = OsIdentity {
            source: crate::os_identity::OsReleaseSource::EtcOsRelease,
            source_path: "/etc/os-release".into(),
            source_sha256: "a".repeat(64),
            source_bytes: 8,
            id: Some("arch".into()),
            name: Some("Arch Linux".into()),
            pretty_name: None,
            version_id: None,
            version_codename: None,
            build_id: Some("rolling".into()),
            variant_id: None,
        };
        assert!(verify_os_identity(&identity).succeeded);

        identity.source_path = "/usr/lib/os-release".into();
        assert_eq!(
            verify_os_identity(&identity).reason,
            VerificationReason::InvalidOsIdentityProvenance
        );

        identity.source_path = "/etc/os-release".into();
        identity.name = Some("bad\nname".into());
        assert_eq!(
            verify_os_identity(&identity).reason,
            VerificationReason::InvalidOsIdentitySchema
        );
    }
}
