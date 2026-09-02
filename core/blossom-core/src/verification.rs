use crate::executor::ExecutionResult;
use crate::memory_summary::{MAX_PROC_MEMINFO_BYTES, MemorySummary, PROC_MEMINFO_PATH};
use crate::os_identity::{MAX_OS_RELEASE_BYTES, MAX_OS_RELEASE_VALUE_BYTES, OsIdentity};
use crate::storage_summary::{ROOT_FILESYSTEM_PATH, StorageSummary, StorageSummarySource};
use crate::uptime::{MAX_PROC_UPTIME_BYTES, PROC_UPTIME_PATH, SystemUptime};
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
    ValidUptime,
    InvalidUptimeProvenance,
    InvalidUptimeSchema,
    ValidMemorySummary,
    InvalidMemorySummaryProvenance,
    InvalidMemorySummarySchema,
    ValidStorageSummary,
    InvalidStorageSummaryProvenance,
    InvalidStorageSummarySchema,
}

pub fn verify_storage_summary(summary: &StorageSummary) -> Verification {
    let provenance_valid = summary.source == StorageSummarySource::RootStatvfs
        && summary.resource_path == ROOT_FILESYSTEM_PATH
        && summary.resource_path == summary.source.as_path();
    let schema_valid = summary.total_bytes > 0 && summary.available_bytes <= summary.total_bytes;
    let reason = if !provenance_valid {
        VerificationReason::InvalidStorageSummaryProvenance
    } else if !schema_valid {
        VerificationReason::InvalidStorageSummarySchema
    } else {
        VerificationReason::ValidStorageSummary
    };
    Verification {
        succeeded: reason == VerificationReason::ValidStorageSummary,
        reason,
    }
}

pub fn verify_memory_summary(summary: &MemorySummary) -> Verification {
    let provenance_valid = summary.source_path == PROC_MEMINFO_PATH
        && summary.source_bytes <= MAX_PROC_MEMINFO_BYTES
        && summary.source_sha256.len() == 64
        && summary
            .source_sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase());
    let schema_valid = summary.available_bytes <= summary.total_bytes
        && summary.swap_free_bytes <= summary.swap_total_bytes;
    let reason = if !provenance_valid {
        VerificationReason::InvalidMemorySummaryProvenance
    } else if !schema_valid {
        VerificationReason::InvalidMemorySummarySchema
    } else {
        VerificationReason::ValidMemorySummary
    };
    Verification {
        succeeded: reason == VerificationReason::ValidMemorySummary,
        reason,
    }
}

pub fn verify_uptime(uptime: &SystemUptime) -> Verification {
    let provenance_valid = uptime.source_path == PROC_UPTIME_PATH
        && uptime.source_bytes <= MAX_PROC_UPTIME_BYTES
        && uptime.source_sha256.len() == 64
        && uptime
            .source_sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase());
    let schema_valid = uptime.nanoseconds < 1_000_000_000;
    let reason = if !provenance_valid {
        VerificationReason::InvalidUptimeProvenance
    } else if !schema_valid {
        VerificationReason::InvalidUptimeSchema
    } else {
        VerificationReason::ValidUptime
    };
    Verification {
        succeeded: reason == VerificationReason::ValidUptime,
        reason,
    }
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

    #[test]
    fn verifies_uptime_schema_and_provenance_without_io() {
        let mut uptime = SystemUptime {
            seconds: 42,
            nanoseconds: 250_000_000,
            source_path: PROC_UPTIME_PATH.into(),
            source_sha256: "a".repeat(64),
            source_bytes: 16,
        };
        assert!(verify_uptime(&uptime).succeeded);
        uptime.source_path = "/tmp/uptime".into();
        assert_eq!(
            verify_uptime(&uptime).reason,
            VerificationReason::InvalidUptimeProvenance
        );
        uptime.source_path = PROC_UPTIME_PATH.into();
        uptime.nanoseconds = 1_000_000_000;
        assert_eq!(
            verify_uptime(&uptime).reason,
            VerificationReason::InvalidUptimeSchema
        );
    }

    #[test]
    fn verifies_memory_summary_schema_and_provenance_without_io() {
        let mut summary = MemorySummary {
            total_bytes: 16 * 1024,
            available_bytes: 8 * 1024,
            swap_total_bytes: 4 * 1024,
            swap_free_bytes: 2 * 1024,
            source_path: PROC_MEMINFO_PATH.into(),
            source_sha256: "c".repeat(64),
            source_bytes: 128,
        };
        assert!(verify_memory_summary(&summary).succeeded);
        summary.source_path = "/tmp/meminfo".into();
        assert_eq!(
            verify_memory_summary(&summary).reason,
            VerificationReason::InvalidMemorySummaryProvenance
        );
        summary.source_path = PROC_MEMINFO_PATH.into();
        summary.available_bytes = summary.total_bytes + 1;
        assert_eq!(
            verify_memory_summary(&summary).reason,
            VerificationReason::InvalidMemorySummarySchema
        );
    }

    #[test]
    fn verifies_storage_summary_schema_and_provenance_without_io() {
        let mut summary = StorageSummary {
            source: StorageSummarySource::RootStatvfs,
            resource_path: ROOT_FILESYSTEM_PATH.into(),
            total_bytes: 100,
            available_bytes: 25,
        };
        assert!(verify_storage_summary(&summary).succeeded);
        summary.resource_path = "/home".into();
        assert_eq!(
            verify_storage_summary(&summary).reason,
            VerificationReason::InvalidStorageSummaryProvenance
        );
        summary.resource_path = ROOT_FILESYSTEM_PATH.into();
        summary.available_bytes = 101;
        assert_eq!(
            verify_storage_summary(&summary).reason,
            VerificationReason::InvalidStorageSummarySchema
        );
    }
}
