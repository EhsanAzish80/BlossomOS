#![forbid(unsafe_code)]

use blossom_core::{
    ApprovalError, ApprovalStore, AuditEvent, AuditLog, BeginOutcome, BlossomEngine, Capability,
    EngineError, Executor, PolicyDecision, PolicyEngine, PolicyRule, RequestId, ToolRequest,
    command_for,
};
use std::fmt::Write as _;

pub const APPROVAL_TTL_MS: u64 = 30_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApprovalChoice {
    ApproveOnce,
    Deny,
    Cancel,
}

pub trait Interaction {
    fn is_interactive(&self) -> bool;
    fn choose(&mut self, preview: &str) -> ApprovalChoice;
}

pub trait Clock {
    fn now_ms(&mut self) -> u64;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunOutcome {
    pub exit_code: i32,
    pub activity: String,
}

pub fn run_fixed_diagnostic<E, I, C>(
    executor: E,
    interaction: &mut I,
    clock: &mut C,
    request_id: RequestId,
) -> RunOutcome
where
    E: Executor,
    I: Interaction,
    C: Clock,
{
    let policy = PolicyEngine::new(vec![PolicyRule {
        capability: Capability::SystemReadKernelIdentity,
        decision: PolicyDecision::Ask,
    }]);
    let mut engine = BlossomEngine::new(policy, ApprovalStore::new(APPROVAL_TTL_MS), executor);
    let request = ToolRequest::SystemUname { request_id };
    let request_json = format!(
        r#"{{"request_id":"{}","tool":"system.uname","arguments":{{}}}}"#,
        request.request_id().as_str()
    );

    let begun_at = clock.now_ms();
    let (request, token) = match engine.begin(&request_json, begun_at) {
        Ok(BeginOutcome::ApprovalRequired { request, token }) => (request, token),
        Ok(BeginOutcome::Denied) => return outcome(2, &engine),
        Ok(BeginOutcome::Completed(_)) => return outcome(0, &engine),
        Err(_) => return outcome(1, &engine),
    };

    let preview = exact_preview(&request);
    let choice = if interaction.is_interactive() {
        interaction.choose(&preview)
    } else {
        ApprovalChoice::Deny
    };
    let decided_at = clock.now_ms();
    let result = match choice {
        ApprovalChoice::ApproveOnce => engine.approve(token, request, decided_at).map(|_| 0),
        ApprovalChoice::Deny => engine.deny_approval(token, request, decided_at).map(|()| 2),
        ApprovalChoice::Cancel => engine
            .cancel_approval(token, request, decided_at)
            .map(|()| 2),
    };
    match result {
        Ok(exit_code) => outcome(exit_code, &engine),
        Err(EngineError::Approval(ApprovalError::Expired)) => outcome(3, &engine),
        Err(_) => outcome(1, &engine),
    }
}

pub fn exact_preview(request: &ToolRequest) -> String {
    let command = command_for(request);
    let command_line = std::iter::once(command.program.display().to_string())
        .chain(command.arguments.iter().cloned())
        .collect::<Vec<_>>()
        .join(" ");
    let capability = PolicyEngine::required_capability(request);
    format!(
        "Request: {}\nReason: Phase 1 fixed kernel identity diagnostic\nPolicy decision: ask\nApproval: once only\nCapability: {}\nCommand: {}\nPrivilege: unprivileged user\nWorking directory: {}\nFilesystem: /usr read-only; isolated /proc, /dev and /tmp\nNetwork: denied\nTimeout: {} ms\nMaximum output: {} bytes",
        request.request_id().as_str(),
        capability.as_str(),
        command_line,
        command.working_directory.display(),
        command.timeout_ms,
        command.max_output_bytes,
    )
}

pub fn render_activity(audit: &AuditLog) -> String {
    let mut output = String::from("Activity\n");
    for record in audit.records() {
        let id = &record.record_hash[..12];
        let description = describe_event(&record.event);
        writeln!(
            &mut output,
            "  audit-{}-{}: {}",
            record.sequence, id, description
        )
        .expect("writing to a String cannot fail");
    }
    output
}

fn outcome<E: Executor>(exit_code: i32, engine: &BlossomEngine<E>) -> RunOutcome {
    RunOutcome {
        exit_code,
        activity: render_activity(engine.audit()),
    }
}

fn describe_event(event: &AuditEvent) -> String {
    match event {
        AuditEvent::RequestRejected { category } => format!("request rejected ({category})"),
        AuditEvent::RequestAccepted { request_id, tool } => {
            format!("request {request_id} accepted for {tool}")
        }
        AuditEvent::PolicyEvaluated {
            request_id,
            capability,
            decision,
        } => format!(
            "request {request_id} policy {:?} for {}",
            decision,
            capability.as_str()
        ),
        AuditEvent::ApprovalIssued { request_id } => {
            format!("request {request_id} awaiting one-time approval")
        }
        AuditEvent::ApprovalRejected { request_id, error } => {
            format!("request {request_id} approval rejected ({error})")
        }
        AuditEvent::ApprovalConsumed { request_id } => {
            format!("request {request_id} approved once")
        }
        AuditEvent::ApprovalDenied { request_id } => format!("request {request_id} denied"),
        AuditEvent::ApprovalCancelled { request_id } => {
            format!("request {request_id} cancelled")
        }
        AuditEvent::ExecutionStarted {
            request_id,
            program,
        } => format!("request {request_id} started {program}"),
        AuditEvent::ExecutionFinished {
            request_id,
            exit_code,
            stdout_bytes,
            stderr_bytes,
            timed_out,
            output_truncated,
            ..
        } => format!(
            "request {request_id} finished: exit={exit_code:?}, stdout={stdout_bytes} bytes, stderr={stderr_bytes} bytes, timed_out={timed_out}, truncated={output_truncated}"
        ),
        AuditEvent::ExecutionFailed { request_id, error } => {
            format!("request {request_id} execution failed ({error})")
        }
        AuditEvent::VerificationFinished {
            request_id,
            verification,
        } => format!(
            "request {request_id} verification: succeeded={}, reason={}",
            verification.succeeded,
            format_args!("{:?}", verification.reason)
        ),
        AuditEvent::Denied { request_id } => format!("request {request_id} denied by policy"),
    }
}
