use crate::{
    ApprovalError, AuditEvent, AuditLog, Capability, PolicyDecision, RequestId,
    SHELL_PROTOCOL_VERSION, ShellActivityCategory, ShellActivityKind, ShellActivityProjection,
};
use std::fmt;

pub fn project_shell_activity(
    audit: &AuditLog,
    after_sequence: Option<u64>,
    limit: u16,
) -> Result<Vec<ShellActivityProjection>, ShellActivityError> {
    if !(1..=crate::MAX_ACTIVITY_BATCH).contains(&limit) {
        return Err(ShellActivityError::InvalidLimit);
    }
    if !audit.verify_chain() {
        return Err(ShellActivityError::InvalidAuditChain);
    }
    let cursor = after_sequence.unwrap_or(0);
    let last = audit.records().last().map_or(0, |record| record.sequence);
    if cursor > last {
        return Err(ShellActivityError::CursorAhead);
    }

    audit
        .records()
        .iter()
        .filter(|record| record.sequence > cursor)
        .take(limit.into())
        .map(|record| {
            let (request_id, kind, category) = project_event(&record.event)?;
            Ok(ShellActivityProjection {
                version: SHELL_PROTOCOL_VERSION,
                sequence: record.sequence,
                request_id,
                kind,
                category,
                content_sha256: match &record.event {
                    AuditEvent::WorkspaceCreateFinished { source_sha256, .. } => {
                        Some(source_sha256.clone())
                    }
                    _ => None,
                },
                content_bytes: match &record.event {
                    AuditEvent::WorkspaceCreateFinished { source_bytes, .. } => Some(*source_bytes),
                    _ => None,
                },
            })
        })
        .collect()
}

fn project_event(
    event: &AuditEvent,
) -> Result<(RequestId, ShellActivityKind, ShellActivityCategory), ShellActivityError> {
    let (id, kind, category) = match event {
        AuditEvent::RequestAccepted {
            request_id, tool, ..
        } if tool == "system.uname" => (
            request_id,
            ShellActivityKind::Request,
            ShellActivityCategory::Accepted,
        ),
        AuditEvent::RequestAccepted {
            request_id, tool, ..
        } if tool == "system.battery.summary" => (
            request_id,
            ShellActivityKind::Request,
            ShellActivityCategory::Accepted,
        ),
        AuditEvent::RequestAccepted {
            request_id, tool, ..
        } if tool == "files.write.create" => (
            request_id,
            ShellActivityKind::Request,
            ShellActivityCategory::Accepted,
        ),
        AuditEvent::RequestAccepted {
            request_id, tool, ..
        } if tool == "files.read.content" => (
            request_id,
            ShellActivityKind::Request,
            ShellActivityCategory::Accepted,
        ),
        AuditEvent::PolicyEvaluated {
            request_id,
            capability: Capability::SystemReadKernelIdentity,
            decision: PolicyDecision::Ask,
        } => (
            request_id,
            ShellActivityKind::Policy,
            ShellActivityCategory::PolicyAsk,
        ),
        AuditEvent::PolicyEvaluated {
            request_id,
            capability: Capability::SystemReadBatterySummary,
            decision: PolicyDecision::Allow,
        } => (
            request_id,
            ShellActivityKind::Policy,
            ShellActivityCategory::PolicyAllow,
        ),
        AuditEvent::PolicyEvaluated {
            request_id,
            capability: Capability::FilesWriteCreate,
            decision: PolicyDecision::Ask,
        } => (
            request_id,
            ShellActivityKind::Policy,
            ShellActivityCategory::PolicyAsk,
        ),
        AuditEvent::PolicyEvaluated {
            request_id,
            capability: Capability::FilesReadContent,
            decision: PolicyDecision::Allow,
        } => (
            request_id,
            ShellActivityKind::Policy,
            ShellActivityCategory::PolicyAllow,
        ),
        AuditEvent::NativeReadStarted {
            request_id,
            resource,
        } if resource == "system.battery.summary" => (
            request_id,
            ShellActivityKind::Context,
            ShellActivityCategory::ReadStarted,
        ),
        AuditEvent::NativeReadStarted {
            request_id,
            resource,
        } if resource.starts_with("file.content:sha256:") => (
            request_id,
            ShellActivityKind::Context,
            ShellActivityCategory::ReadStarted,
        ),
        AuditEvent::BatterySummaryReadFinished { request_id, .. } => (
            request_id,
            ShellActivityKind::Context,
            ShellActivityCategory::ReadFinished,
        ),
        AuditEvent::BatterySummaryReadFailed { request_id, .. } => (
            request_id,
            ShellActivityKind::Context,
            ShellActivityCategory::ReadFailed,
        ),
        AuditEvent::FileContentReadFinished { request_id, .. } => (
            request_id,
            ShellActivityKind::Context,
            ShellActivityCategory::ReadFinished,
        ),
        AuditEvent::FileContentReadFailed { request_id, .. } => (
            request_id,
            ShellActivityKind::Context,
            ShellActivityCategory::ReadFailed,
        ),
        AuditEvent::ApprovalIssued { request_id } => (
            request_id,
            ShellActivityKind::Approval,
            ShellActivityCategory::ApprovalIssued,
        ),
        AuditEvent::ApprovalRejected {
            request_id,
            error: ApprovalError::Expired,
        } => (
            request_id,
            ShellActivityKind::Approval,
            ShellActivityCategory::Expired,
        ),
        AuditEvent::ApprovalRejected { request_id, .. } => (
            request_id,
            ShellActivityKind::Approval,
            ShellActivityCategory::ApprovalRejected,
        ),
        AuditEvent::ApprovalConsumed { request_id } => (
            request_id,
            ShellActivityKind::Approval,
            ShellActivityCategory::ApprovedOnce,
        ),
        AuditEvent::ApprovalDenied { request_id } => (
            request_id,
            ShellActivityKind::Terminal,
            ShellActivityCategory::Denied,
        ),
        AuditEvent::ApprovalCancelled { request_id } => (
            request_id,
            ShellActivityKind::Terminal,
            ShellActivityCategory::Cancelled,
        ),
        AuditEvent::WorkspaceCreateStarted { request_id, .. } => (
            request_id,
            ShellActivityKind::Request,
            ShellActivityCategory::PublicationStarted,
        ),
        AuditEvent::WorkspaceCreateFinished { request_id, .. } => (
            request_id,
            ShellActivityKind::Effect,
            ShellActivityCategory::PublicationFinished,
        ),
        AuditEvent::WorkspaceCreateFailed { request_id, .. } => (
            request_id,
            ShellActivityKind::Terminal,
            ShellActivityCategory::PublicationFailed,
        ),
        AuditEvent::ExecutionStarted {
            request_id,
            program,
        } if program == "/usr/bin/uname" => (
            request_id,
            ShellActivityKind::Execution,
            ShellActivityCategory::Started,
        ),
        AuditEvent::ExecutionFinished { request_id, .. } => (
            request_id,
            ShellActivityKind::Execution,
            ShellActivityCategory::ExecutionFinished,
        ),
        AuditEvent::ExecutionFailed { request_id, .. } => (
            request_id,
            ShellActivityKind::Terminal,
            ShellActivityCategory::ExecutionFailed,
        ),
        AuditEvent::VerificationFinished {
            request_id,
            verification,
        } => (
            request_id,
            ShellActivityKind::Terminal,
            if verification.succeeded {
                ShellActivityCategory::Verified
            } else {
                ShellActivityCategory::VerificationFailed
            },
        ),
        AuditEvent::Denied { request_id } => (
            request_id,
            ShellActivityKind::Terminal,
            ShellActivityCategory::Denied,
        ),
        _ => return Err(ShellActivityError::UnexpectedAuditEvent),
    };
    let request_id =
        RequestId::parse(id.clone()).map_err(|_| ShellActivityError::InvalidRequestId)?;
    Ok((request_id, kind, category))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShellActivityError {
    InvalidLimit,
    InvalidAuditChain,
    CursorAhead,
    InvalidRequestId,
    UnexpectedAuditEvent,
}

impl fmt::Display for ShellActivityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidLimit => "shell activity limit is invalid",
            Self::InvalidAuditChain => "shell activity audit chain is invalid",
            Self::CursorAhead => "shell activity cursor is ahead of the audit log",
            Self::InvalidRequestId => "shell activity request identifier is invalid",
            Self::UnexpectedAuditEvent => "shell activity encountered an unsupported audit event",
        })
    }
}

impl std::error::Error for ShellActivityError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PolicyDecision, Verification, verification::VerificationReason};

    fn audit() -> AuditLog {
        let mut audit = AuditLog::default();
        audit.append(AuditEvent::RequestAccepted {
            request_id: "shell-0000000000000007-1".into(),
            tool: "system.uname".into(),
            origin: crate::RequestOrigin::InternalFixed,
        });
        audit.append(AuditEvent::PolicyEvaluated {
            request_id: "shell-0000000000000007-1".into(),
            capability: Capability::SystemReadKernelIdentity,
            decision: PolicyDecision::Ask,
        });
        audit.append(AuditEvent::ApprovalIssued {
            request_id: "shell-0000000000000007-1".into(),
        });
        audit.append(AuditEvent::ApprovalConsumed {
            request_id: "shell-0000000000000007-1".into(),
        });
        audit.append(AuditEvent::ExecutionStarted {
            request_id: "shell-0000000000000007-1".into(),
            program: "/usr/bin/uname".into(),
        });
        audit.append(AuditEvent::VerificationFinished {
            request_id: "shell-0000000000000007-1".into(),
            verification: Verification {
                succeeded: true,
                reason: VerificationReason::ValidSystemName,
            },
        });
        audit
    }

    #[test]
    fn projects_only_closed_content_free_fields() {
        let projected = project_shell_activity(&audit(), None, 16).expect("projection");
        assert_eq!(projected.len(), 6);
        assert_eq!(
            projected.last().expect("last").category,
            ShellActivityCategory::Verified
        );
        let encoded = serde_json::to_string(&projected).expect("serialize");
        for forbidden in ["stdout", "stderr", "token", "Linux", "prompt", "reasoning"] {
            assert!(!encoded.contains(forbidden));
        }
    }

    #[test]
    fn cursor_and_limit_are_exact_and_bounded() {
        let projected = project_shell_activity(&audit(), Some(3), 2).expect("projection");
        assert_eq!(
            projected
                .iter()
                .map(|item| item.sequence)
                .collect::<Vec<_>>(),
            vec![4, 5]
        );
        assert_eq!(
            project_shell_activity(&audit(), Some(7), 1),
            Err(ShellActivityError::CursorAhead)
        );
        assert_eq!(
            project_shell_activity(&audit(), None, 0),
            Err(ShellActivityError::InvalidLimit)
        );
    }

    #[test]
    fn unsupported_events_fail_instead_of_leaking_fields_or_hiding_gaps() {
        let mut audit = AuditLog::default();
        audit.append(AuditEvent::RequestRejected {
            category: "private parser detail".into(),
        });
        assert_eq!(
            project_shell_activity(&audit, None, 1),
            Err(ShellActivityError::UnexpectedAuditEvent)
        );
    }

    #[test]
    fn workspace_publication_is_visible_without_becoming_an_executor_event() {
        let request_id = "shell-0000000000000007-2";
        let mut audit = AuditLog::default();
        audit.append(AuditEvent::RequestAccepted {
            request_id: request_id.into(),
            tool: "files.write.create".into(),
            origin: crate::RequestOrigin::ModelProposed,
        });
        audit.append(AuditEvent::PolicyEvaluated {
            request_id: request_id.into(),
            capability: Capability::FilesWriteCreate,
            decision: PolicyDecision::Ask,
        });
        audit.append(AuditEvent::WorkspaceCreateStarted {
            request_id: request_id.into(),
            workspace_sha256: "1".repeat(64),
            destination_sha256: "2".repeat(64),
            root_device: 1,
            root_inode: 2,
            parent_device: 1,
            parent_inode: 2,
            source_bytes: 7,
            source_sha256: "3".repeat(64),
        });
        audit.append(AuditEvent::WorkspaceCreateFinished {
            request_id: request_id.into(),
            workspace_sha256: "1".repeat(64),
            destination_sha256: "2".repeat(64),
            created_device: 1,
            created_inode: 3,
            source_bytes: 7,
            source_sha256: "3".repeat(64),
            state: crate::WorkspaceCreateState::DurableCreated,
        });
        let projected = project_shell_activity(&audit, None, 16).expect("projection");
        assert_eq!(projected.len(), 4);
        assert_eq!(projected[2].kind, ShellActivityKind::Request);
        assert_eq!(
            projected[2].category,
            ShellActivityCategory::PublicationStarted
        );
        assert_eq!(projected[3].kind, ShellActivityKind::Effect);
        assert_eq!(projected[3].content_sha256, Some("3".repeat(64)));
        assert_eq!(projected[3].content_bytes, Some(7));
        assert!(
            projected
                .iter()
                .all(|item| item.kind != ShellActivityKind::Execution)
        );
    }
}
