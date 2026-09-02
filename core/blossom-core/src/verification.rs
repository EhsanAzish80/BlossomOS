use crate::executor::ExecutionResult;
use crate::file_read::{FileContent, MAX_FILE_CONTENT_BYTES, validate_selected_path};
use crate::memory_summary::{MAX_PROC_MEMINFO_BYTES, MemorySummary, PROC_MEMINFO_PATH};
use crate::os_identity::{MAX_OS_RELEASE_BYTES, MAX_OS_RELEASE_VALUE_BYTES, OsIdentity};
use crate::process_list::{
    MAX_PROCESS_NAME_BYTES, MAX_PROCESS_RESULTS, ProcessList, ProcessListSource,
};
use crate::process_self::{ProcessSelf, ProcessSelfSource};
use crate::service_status::{ServiceStatus, validate_service_status};
use crate::storage_summary::{ROOT_FILESYSTEM_PATH, StorageSummary, StorageSummarySource};
use crate::uptime::{MAX_PROC_UPTIME_BYTES, PROC_UPTIME_PATH, SystemUptime};
use crate::workspace_create::{
    WORKSPACE_FILE_MODE, WorkspaceCreateSelection, WorkspaceCreateState, WorkspaceFileCreated,
    validate_relative_destination,
};
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
    ValidProcessSelf,
    InvalidProcessSelfProvenance,
    InvalidProcessSelfSchema,
    ValidProcessList,
    InvalidProcessListProvenance,
    InvalidProcessListSchema,
    ValidFileContent,
    InvalidFileContentProvenance,
    InvalidFileContentSchema,
    ValidWorkspaceFileCreated,
    WorkspaceFileDurabilityUncertain,
    InvalidWorkspaceFileProvenance,
    InvalidWorkspaceFileSchema,
    ValidServiceStatus,
    InvalidServiceStatusProvenance,
    InvalidServiceStatusSchema,
}

pub fn verify_service_status(status: &ServiceStatus, expected_unit: &str) -> Verification {
    let reason = match validate_service_status(status, expected_unit) {
        Ok(()) => VerificationReason::ValidServiceStatus,
        Err(crate::service_status::ServiceStatusError::ProtocolViolation) => {
            VerificationReason::InvalidServiceStatusProvenance
        }
        Err(_) => VerificationReason::InvalidServiceStatusSchema,
    };
    Verification {
        succeeded: reason == VerificationReason::ValidServiceStatus,
        reason,
    }
}

pub fn verify_workspace_file_created(
    result: &WorkspaceFileCreated,
    expected: &WorkspaceCreateSelection,
) -> Verification {
    let digest_valid = result.source_sha256.len() == 64
        && result
            .source_sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase());
    let provenance_valid = crate::file_read::validate_selected_path(&result.workspace_root).is_ok()
        && validate_relative_destination(&result.relative_destination).is_ok()
        && result.root_identity.is_valid()
        && result.parent_identity.is_valid()
        && result.root_identity.device == result.parent_identity.device
        && result.created_device == result.parent_identity.device
        && result.created_inode > 0
        && result.workspace_root == expected.workspace_root
        && result.relative_destination == expected.relative_destination
        && result.root_identity == expected.root_identity
        && result.parent_identity == expected.parent_identity
        && result.source_bytes == expected.content.len()
        && result.source_sha256 == expected.content_sha256
        && result.mode == expected.mode;
    let schema_valid = result.source_bytes <= MAX_FILE_CONTENT_BYTES
        && digest_valid
        && result.mode == WORKSPACE_FILE_MODE;
    let reason = if !provenance_valid {
        VerificationReason::InvalidWorkspaceFileProvenance
    } else if !schema_valid {
        VerificationReason::InvalidWorkspaceFileSchema
    } else if result.state == WorkspaceCreateState::PublishedDurabilityUncertain {
        VerificationReason::WorkspaceFileDurabilityUncertain
    } else {
        VerificationReason::ValidWorkspaceFileCreated
    };
    Verification {
        succeeded: reason == VerificationReason::ValidWorkspaceFileCreated,
        reason,
    }
}

pub fn verify_file_content(result: &FileContent) -> Verification {
    use sha2::{Digest, Sha256};
    let bytes = result.content.as_bytes();
    let expected_digest = Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let provenance_valid = validate_selected_path(&result.selection.absolute_path).is_ok()
        && result.selection.identity.is_valid()
        && result.selection.identity.size == result.source_bytes as u64
        && result.source_sha256 == expected_digest;
    let schema_valid =
        result.source_bytes == bytes.len() && result.source_bytes <= MAX_FILE_CONTENT_BYTES;
    let reason = if !provenance_valid {
        VerificationReason::InvalidFileContentProvenance
    } else if !schema_valid {
        VerificationReason::InvalidFileContentSchema
    } else {
        VerificationReason::ValidFileContent
    };
    Verification {
        succeeded: reason == VerificationReason::ValidFileContent,
        reason,
    }
}

pub fn verify_process_list(list: &ProcessList) -> Verification {
    let provenance_valid = list.source == ProcessListSource::ProcStatusSameEffectiveUser;
    let schema_valid = list.processes.len() <= MAX_PROCESS_RESULTS
        && list.processes.iter().all(|entry| {
            entry.process_id > 0
                && !entry.name.is_empty()
                && entry.name.len() <= MAX_PROCESS_NAME_BYTES
                && !entry.name.chars().any(char::is_control)
        })
        && list
            .processes
            .windows(2)
            .all(|pair| pair[0].process_id < pair[1].process_id);
    let reason = if !provenance_valid {
        VerificationReason::InvalidProcessListProvenance
    } else if !schema_valid {
        VerificationReason::InvalidProcessListSchema
    } else {
        VerificationReason::ValidProcessList
    };
    Verification {
        succeeded: reason == VerificationReason::ValidProcessList,
        reason,
    }
}

pub fn verify_process_self(identity: &ProcessSelf) -> Verification {
    let provenance_valid = identity.source == ProcessSelfSource::NativeProcessIdentity;
    let schema_valid = identity.process_id > 0;
    let reason = if !provenance_valid {
        VerificationReason::InvalidProcessSelfProvenance
    } else if !schema_valid {
        VerificationReason::InvalidProcessSelfSchema
    } else {
        VerificationReason::ValidProcessSelf
    };
    Verification {
        succeeded: reason == VerificationReason::ValidProcessSelf,
        reason,
    }
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
    fn verifies_file_content_and_exact_provenance_without_reopening() {
        use crate::file_read::{FileIdentity, FileSelection};
        use sha2::{Digest, Sha256};
        let content = "hello".to_string();
        let digest = Sha256::digest(content.as_bytes())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        let mut result = FileContent {
            selection: FileSelection {
                absolute_path: "/home/user/note.txt".into(),
                identity: FileIdentity {
                    device: 1,
                    inode: 2,
                    size: 5,
                    modified_seconds: 3,
                    modified_nanoseconds: 4,
                    changed_seconds: 5,
                    changed_nanoseconds: 6,
                },
            },
            content,
            source_bytes: 5,
            source_sha256: digest,
        };
        assert!(verify_file_content(&result).succeeded);
        result.selection.absolute_path = "/home/user/../secret".into();
        assert_eq!(
            verify_file_content(&result).reason,
            VerificationReason::InvalidFileContentProvenance
        );
    }

    #[test]
    fn verifies_durable_workspace_creation_and_rejects_uncertain_durability() {
        use crate::workspace_create::{
            DirectoryIdentity, WorkspaceCreateSelection, WorkspaceCreateState,
        };
        let mut result = WorkspaceFileCreated {
            workspace_root: "/home/user/workspace".into(),
            relative_destination: "new.txt".into(),
            root_identity: DirectoryIdentity {
                device: 1,
                inode: 2,
            },
            parent_identity: DirectoryIdentity {
                device: 1,
                inode: 3,
            },
            created_device: 1,
            created_inode: 4,
            source_bytes: 5,
            source_sha256: "a".repeat(64),
            mode: WORKSPACE_FILE_MODE,
            state: WorkspaceCreateState::DurableCreated,
        };
        let expected = WorkspaceCreateSelection {
            workspace_root: result.workspace_root.clone(),
            root_identity: result.root_identity.clone(),
            parent_identity: result.parent_identity.clone(),
            relative_destination: result.relative_destination.clone(),
            content: "hello".into(),
            content_sha256: "a".repeat(64),
            mode: WORKSPACE_FILE_MODE,
        };
        assert!(verify_workspace_file_created(&result, &expected).succeeded);
        result.state = WorkspaceCreateState::PublishedDurabilityUncertain;
        assert_eq!(
            verify_workspace_file_created(&result, &expected).reason,
            VerificationReason::WorkspaceFileDurabilityUncertain
        );
        assert!(!verify_workspace_file_created(&result, &expected).succeeded);
    }

    #[test]
    fn verifies_process_list_schema_and_provenance_without_io() {
        let mut list = ProcessList {
            source: ProcessListSource::ProcStatusSameEffectiveUser,
            processes: vec![crate::process_list::ProcessListEntry {
                process_id: 42,
                name: "blossom".into(),
                state: crate::process_list::ProcessState::Sleeping,
            }],
            skipped_entries: 0,
            truncated: false,
        };
        assert!(verify_process_list(&list).succeeded);
        list.processes.push(crate::process_list::ProcessListEntry {
            process_id: 41,
            name: "out-of-order".into(),
            state: crate::process_list::ProcessState::Running,
        });
        assert_eq!(
            verify_process_list(&list).reason,
            VerificationReason::InvalidProcessListSchema
        );
    }

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

    #[test]
    fn verifies_process_self_schema_and_provenance_without_io() {
        let mut identity = ProcessSelf {
            source: ProcessSelfSource::NativeProcessIdentity,
            process_id: 42,
            parent_process_id: 7,
            effective_user_id: 1000,
            effective_group_id: 1000,
        };
        assert!(verify_process_self(&identity).succeeded);
        identity.process_id = 0;
        assert_eq!(
            verify_process_self(&identity).reason,
            VerificationReason::InvalidProcessSelfSchema
        );
    }

    #[test]
    fn verifies_exact_service_status_without_a_second_observation() {
        let mut status = crate::service_status::ServiceStatus {
            requested_unit: "sshd.service".into(),
            scope: "system".into(),
            canonical_unit: "sshd.service".into(),
            load_state: "loaded".into(),
            active_state: "future-state".into(),
            sub_state: "future-substate".into(),
            destination: crate::service_status::SYSTEMD_DESTINATION.into(),
            manager_interface: crate::service_status::SYSTEMD_MANAGER_INTERFACE.into(),
            unit_interface: crate::service_status::SYSTEMD_UNIT_INTERFACE.into(),
        };
        assert!(verify_service_status(&status, "sshd.service").succeeded);
        status.requested_unit = "other.service".into();
        assert!(!verify_service_status(&status, "sshd.service").succeeded);
    }
}
