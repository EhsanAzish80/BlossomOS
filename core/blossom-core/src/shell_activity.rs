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
            })
        })
        .collect()
}

fn project_event(
    event: &AuditEvent,
) -> Result<(RequestId, ShellActivityKind, ShellActivityCategory), ShellActivityError> {
    let (id, kind, category) = match event {
        AuditEvent::RequestAccepted { request_id, tool } if tool == "system.uname" => (
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
}
