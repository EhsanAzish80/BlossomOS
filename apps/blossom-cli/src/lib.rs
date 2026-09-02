#![forbid(unsafe_code)]

use blossom_core::{
    ApprovalError, ApprovalStore, AuditEvent, AuditLog, BeginOutcome, BlossomEngine, Capability,
    EngineError, Executor, FileContent, FileContentProvider, MemorySummary, MemorySummaryProvider,
    OsIdentity, OsIdentityProvider, PolicyDecision, PolicyEngine, PolicyRule, ProcessList,
    ProcessListProvider, ProcessSelf, ProcessSelfProvider, RequestId, ServiceStatus,
    ServiceStatusProvider, StorageSummary, StorageSummaryProvider, SystemUptime, ToolOutput,
    ToolRequest, UptimeProvider, WorkspaceCreateProvider, WorkspaceCreateState,
    WorkspaceFileCreated, command_for,
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
    pub result: Option<String>,
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

pub fn run_os_identity<E, O, C>(
    executor: E,
    os_identity: O,
    clock: &mut C,
    request_id: RequestId,
) -> RunOutcome
where
    E: Executor,
    O: OsIdentityProvider,
    C: Clock,
{
    let policy = PolicyEngine::new(vec![PolicyRule {
        capability: Capability::SystemReadOsIdentity,
        decision: PolicyDecision::Allow,
    }]);
    let mut engine = BlossomEngine::with_os_identity(
        policy,
        ApprovalStore::new(APPROVAL_TTL_MS),
        executor,
        os_identity,
    );
    let request_json = format!(
        r#"{{"request_id":"{}","tool":"system.os.identity","arguments":{{}}}}"#,
        request_id.as_str()
    );
    let result = match engine.begin(&request_json, clock.now_ms()) {
        Ok(BeginOutcome::Completed(completed)) => match completed.output {
            ToolOutput::OsIdentity(identity) if completed.verification.succeeded => {
                Some(render_os_identity(&identity))
            }
            _ => return outcome_with_result(1, None, &engine),
        },
        _ => return outcome_with_result(1, None, &engine),
    };
    outcome_with_result(0, result, &engine)
}

pub fn run_uptime<E, U, C>(
    executor: E,
    uptime: U,
    clock: &mut C,
    request_id: RequestId,
) -> RunOutcome
where
    E: Executor,
    U: UptimeProvider,
    C: Clock,
{
    let policy = PolicyEngine::new(vec![PolicyRule {
        capability: Capability::SystemReadUptime,
        decision: PolicyDecision::Allow,
    }]);
    let mut engine = BlossomEngine::with_uptime(
        policy,
        ApprovalStore::new(APPROVAL_TTL_MS),
        executor,
        uptime,
    );
    let request_json = format!(
        r#"{{"request_id":"{}","tool":"system.uptime","arguments":{{}}}}"#,
        request_id.as_str()
    );
    let result = match engine.begin(&request_json, clock.now_ms()) {
        Ok(BeginOutcome::Completed(completed)) => match completed.output {
            ToolOutput::Uptime(uptime) if completed.verification.succeeded => {
                Some(render_uptime(&uptime))
            }
            _ => return outcome_with_result(1, None, &engine),
        },
        _ => return outcome_with_result(1, None, &engine),
    };
    outcome_with_result(0, result, &engine)
}

pub fn run_memory_summary<E, M, C>(
    executor: E,
    memory_summary: M,
    clock: &mut C,
    request_id: RequestId,
) -> RunOutcome
where
    E: Executor,
    M: MemorySummaryProvider,
    C: Clock,
{
    let policy = PolicyEngine::new(vec![PolicyRule {
        capability: Capability::SystemReadMemorySummary,
        decision: PolicyDecision::Allow,
    }]);
    let mut engine = BlossomEngine::with_memory_summary(
        policy,
        ApprovalStore::new(APPROVAL_TTL_MS),
        executor,
        memory_summary,
    );
    let request_json = format!(
        r#"{{"request_id":"{}","tool":"system.memory.summary","arguments":{{}}}}"#,
        request_id.as_str()
    );
    let result = match engine.begin(&request_json, clock.now_ms()) {
        Ok(BeginOutcome::Completed(completed)) => match completed.output {
            ToolOutput::MemorySummary(summary) if completed.verification.succeeded => {
                Some(render_memory_summary(&summary))
            }
            _ => return outcome_with_result(1, None, &engine),
        },
        _ => return outcome_with_result(1, None, &engine),
    };
    outcome_with_result(0, result, &engine)
}

pub fn run_storage_summary<E, S, C>(
    executor: E,
    storage_summary: S,
    clock: &mut C,
    request_id: RequestId,
) -> RunOutcome
where
    E: Executor,
    S: StorageSummaryProvider,
    C: Clock,
{
    let policy = PolicyEngine::new(vec![PolicyRule {
        capability: Capability::SystemReadStorageSummary,
        decision: PolicyDecision::Allow,
    }]);
    let mut engine = BlossomEngine::with_storage_summary(
        policy,
        ApprovalStore::new(APPROVAL_TTL_MS),
        executor,
        storage_summary,
    );
    let request_json = format!(
        r#"{{"request_id":"{}","tool":"system.storage.summary","arguments":{{}}}}"#,
        request_id.as_str()
    );
    let result = match engine.begin(&request_json, clock.now_ms()) {
        Ok(BeginOutcome::Completed(completed)) => match completed.output {
            ToolOutput::StorageSummary(summary) if completed.verification.succeeded => {
                Some(render_storage_summary(&summary))
            }
            _ => return outcome_with_result(1, None, &engine),
        },
        _ => return outcome_with_result(1, None, &engine),
    };
    outcome_with_result(0, result, &engine)
}

pub fn run_process_self<E, P, C>(
    executor: E,
    process_self: P,
    clock: &mut C,
    request_id: RequestId,
) -> RunOutcome
where
    E: Executor,
    P: ProcessSelfProvider,
    C: Clock,
{
    let policy = PolicyEngine::new(vec![PolicyRule {
        capability: Capability::ProcessReadSelf,
        decision: PolicyDecision::Allow,
    }]);
    let mut engine = BlossomEngine::with_process_self(
        policy,
        ApprovalStore::new(APPROVAL_TTL_MS),
        executor,
        process_self,
    );
    let request_json = format!(
        r#"{{"request_id":"{}","tool":"process.self","arguments":{{}}}}"#,
        request_id.as_str()
    );
    let result = match engine.begin(&request_json, clock.now_ms()) {
        Ok(BeginOutcome::Completed(completed)) => match completed.output {
            ToolOutput::ProcessSelf(identity) if completed.verification.succeeded => {
                Some(render_process_self(&identity))
            }
            _ => return outcome_with_result(1, None, &engine),
        },
        _ => return outcome_with_result(1, None, &engine),
    };
    outcome_with_result(0, result, &engine)
}

pub fn run_process_list<E, L, I, C>(
    executor: E,
    process_list: L,
    interaction: &mut I,
    clock: &mut C,
    request_id: RequestId,
) -> RunOutcome
where
    E: Executor,
    L: ProcessListProvider,
    I: Interaction,
    C: Clock,
{
    let policy = PolicyEngine::new(vec![PolicyRule {
        capability: Capability::ProcessReadList,
        decision: PolicyDecision::Ask,
    }]);
    let mut engine = BlossomEngine::with_process_list(
        policy,
        ApprovalStore::new(APPROVAL_TTL_MS),
        executor,
        process_list,
    );
    let request = ToolRequest::ProcessList { request_id };
    let request_json = format!(
        r#"{{"request_id":"{}","tool":"process.list","arguments":{{}}}}"#,
        request.request_id().as_str()
    );
    let (request, token) = match engine.begin(&request_json, clock.now_ms()) {
        Ok(BeginOutcome::ApprovalRequired { request, token }) => (request, token),
        Ok(BeginOutcome::Denied) => return outcome(2, &engine),
        _ => return outcome(1, &engine),
    };
    let preview = process_list_preview(&request);
    let choice = if interaction.is_interactive() {
        interaction.choose(&preview)
    } else {
        ApprovalChoice::Deny
    };
    let decided_at = clock.now_ms();
    match choice {
        ApprovalChoice::ApproveOnce => match engine.approve(token, request, decided_at) {
            Ok(completed) => match completed.output {
                ToolOutput::ProcessList(list) if completed.verification.succeeded => {
                    outcome_with_result(0, Some(render_process_list(&list)), &engine)
                }
                _ => outcome(1, &engine),
            },
            Err(EngineError::Approval(ApprovalError::Expired)) => outcome(3, &engine),
            Err(_) => outcome(1, &engine),
        },
        ApprovalChoice::Deny => match engine.deny_approval(token, request, decided_at) {
            Ok(()) => outcome(2, &engine),
            Err(EngineError::Approval(ApprovalError::Expired)) => outcome(3, &engine),
            Err(_) => outcome(1, &engine),
        },
        ApprovalChoice::Cancel => match engine.cancel_approval(token, request, decided_at) {
            Ok(()) => outcome(2, &engine),
            Err(_) => outcome(1, &engine),
        },
    }
}

pub fn process_list_preview(request: &ToolRequest) -> String {
    let capability = PolicyEngine::required_capability(request);
    format!(
        "Request: {}\nReason: inspect same-user process activity\nPolicy decision: ask\nApproval: once only\nCapability: {}\nScope: processes with Blossom's effective user ID\nFields: PID, short kernel name, coarse state\nMaximum results: 256\nSource: /proc/<pid>/status through a pinned PID directory descriptor\nCommand: none (native Linux read)\nCommand line, environment, open files, sockets, and memory: not read\nPrivilege: unprivileged user\nNetwork: not used",
        request.request_id().as_str(),
        capability.as_str()
    )
}

pub fn run_file_read<E, F, I, C>(
    executor: E,
    file_content: F,
    interaction: &mut I,
    clock: &mut C,
    request_id: RequestId,
) -> RunOutcome
where
    E: Executor,
    F: FileContentProvider,
    I: Interaction,
    C: Clock,
{
    let selection = file_content.selection().clone();
    let policy = PolicyEngine::new(vec![PolicyRule {
        capability: Capability::FilesReadContent,
        decision: PolicyDecision::Ask,
    }]);
    let mut engine = BlossomEngine::with_file_content(
        policy,
        ApprovalStore::new(APPROVAL_TTL_MS),
        executor,
        file_content,
    );
    let request_json = serde_json::json!({
        "request_id": request_id.as_str(), "tool": "files.read.content",
        "arguments": { "selection": selection }
    })
    .to_string();
    let (request, token) = match engine.begin(&request_json, clock.now_ms()) {
        Ok(BeginOutcome::ApprovalRequired { request, token }) => (request, token),
        _ => return outcome(1, &engine),
    };
    let preview = file_read_preview(&request);
    let choice = if interaction.is_interactive() {
        interaction.choose(&preview)
    } else {
        ApprovalChoice::Deny
    };
    let decided_at = clock.now_ms();
    match choice {
        ApprovalChoice::ApproveOnce => match engine.approve(token, request, decided_at) {
            Ok(completed) => match completed.output {
                ToolOutput::FileContent(result) if completed.verification.succeeded => {
                    outcome_with_result(0, Some(render_file_content(&result)), &engine)
                }
                _ => outcome(1, &engine),
            },
            Err(EngineError::Approval(ApprovalError::Expired)) => outcome(3, &engine),
            Err(_) => outcome(1, &engine),
        },
        ApprovalChoice::Deny => match engine.deny_approval(token, request, decided_at) {
            Ok(()) => outcome(2, &engine),
            Err(EngineError::Approval(ApprovalError::Expired)) => outcome(3, &engine),
            Err(_) => outcome(1, &engine),
        },
        ApprovalChoice::Cancel => match engine.cancel_approval(token, request, decided_at) {
            Ok(()) => outcome(2, &engine),
            Err(_) => outcome(1, &engine),
        },
    }
}

pub fn file_read_preview(request: &ToolRequest) -> String {
    let ToolRequest::FilesReadContent { selection, .. } = request else {
        return "Invalid file-read request".into();
    };
    format!(
        "Request: {}\nReason: read one user-selected text file\nPolicy decision: ask\nApproval: once only\nCapability: files.read:content\nExact path: {}\nSelected identity: device={}, inode={}, size={} bytes\nMaximum content: 65536 bytes\nResolution: Linux openat2, no symlinks or magic links\nRead source: retained selected descriptor\nCommand: none (native Linux read)\nDirectories, globbing, relative paths, writes, network, and privilege: denied",
        request.request_id().as_str(),
        selection.absolute_path,
        selection.identity.device,
        selection.identity.inode,
        selection.identity.size,
    )
}

pub fn run_workspace_create<E, W, I, C>(
    executor: E,
    workspace_create: W,
    interaction: &mut I,
    clock: &mut C,
    request_id: RequestId,
) -> RunOutcome
where
    E: Executor,
    W: WorkspaceCreateProvider,
    I: Interaction,
    C: Clock,
{
    let selection = workspace_create.selection().clone();
    let policy = PolicyEngine::new(vec![PolicyRule {
        capability: Capability::FilesWriteCreate,
        decision: PolicyDecision::Ask,
    }]);
    let mut engine = BlossomEngine::with_workspace_create(
        policy,
        ApprovalStore::new(APPROVAL_TTL_MS),
        executor,
        workspace_create,
    );
    let request_json = serde_json::json!({
        "request_id": request_id.as_str(), "tool": "files.write.create",
        "arguments": { "selection": selection }
    })
    .to_string();
    let (request, token) = match engine.begin(&request_json, clock.now_ms()) {
        Ok(BeginOutcome::ApprovalRequired { request, token }) => (request, token),
        _ => return outcome(1, &engine),
    };
    let preview = workspace_create_preview(&request);
    let choice = if interaction.is_interactive() {
        interaction.choose(&preview)
    } else {
        ApprovalChoice::Deny
    };
    let decided_at = clock.now_ms();
    match choice {
        ApprovalChoice::ApproveOnce => match engine.approve(token, request, decided_at) {
            Ok(completed) => match completed.output {
                ToolOutput::WorkspaceFileCreated(result) => {
                    let exit_code = if result.state == WorkspaceCreateState::DurableCreated
                        && completed.verification.succeeded
                    {
                        0
                    } else {
                        4
                    };
                    outcome_with_result(exit_code, Some(render_workspace_created(&result)), &engine)
                }
                _ => outcome(1, &engine),
            },
            Err(EngineError::Approval(ApprovalError::Expired)) => outcome(3, &engine),
            Err(_) => outcome(1, &engine),
        },
        ApprovalChoice::Deny => match engine.deny_approval(token, request, decided_at) {
            Ok(()) => outcome(2, &engine),
            Err(EngineError::Approval(ApprovalError::Expired)) => outcome(3, &engine),
            Err(_) => outcome(1, &engine),
        },
        ApprovalChoice::Cancel => match engine.cancel_approval(token, request, decided_at) {
            Ok(()) => outcome(2, &engine),
            Err(_) => outcome(1, &engine),
        },
    }
}

pub fn workspace_create_preview(request: &ToolRequest) -> String {
    let ToolRequest::FilesWriteCreate { selection, .. } = request else {
        return "Invalid workspace-create request".into();
    };
    let escaped = serde_json::to_string(&selection.content).expect("UTF-8 content serializes");
    format!(
        "Request: {}\nReason: atomically create one new workspace text file\nPolicy decision: ask\nApproval: once only\nCapability: files.write:create\nWorkspace root: {}\nExact relative destination: {}\nRoot identity: device={}, inode={}\nParent identity: device={}, inode={}\nPermissions: 0600\nContent bytes: {}\nContent SHA-256: {}\nExact content (JSON escaped): {}\nPublication: verified unnamed O_TMPFILE inode via linkat AT_EMPTY_PATH\nDurability: fsync file and parent directory\nOverwrite, append, delete, symlinks, mount crossing, commands, network, and privilege: denied",
        request.request_id().as_str(),
        selection.workspace_root,
        selection.relative_destination,
        selection.root_identity.device,
        selection.root_identity.inode,
        selection.parent_identity.device,
        selection.parent_identity.inode,
        selection.content.len(),
        selection.content_sha256,
        escaped,
    )
}

pub fn run_service_status<E, V, I, C>(
    executor: E,
    service_status: V,
    interaction: &mut I,
    clock: &mut C,
    request_id: RequestId,
    unit: String,
) -> RunOutcome
where
    E: Executor,
    V: ServiceStatusProvider,
    I: Interaction,
    C: Clock,
{
    let policy = PolicyEngine::new(vec![PolicyRule {
        capability: Capability::ServicesReadStatus,
        decision: PolicyDecision::Ask,
    }]);
    let mut engine = BlossomEngine::with_service_status(
        policy,
        ApprovalStore::new(APPROVAL_TTL_MS),
        executor,
        service_status,
    );
    let request_json = serde_json::json!({
        "request_id": request_id.as_str(), "tool": "services.read.status",
        "arguments": { "selection": { "unit": unit } }
    })
    .to_string();
    let (request, token) = match engine.begin(&request_json, clock.now_ms()) {
        Ok(BeginOutcome::ApprovalRequired { request, token }) => (request, token),
        _ => return outcome(1, &engine),
    };
    let preview = service_status_preview(&request);
    let choice = if interaction.is_interactive() {
        interaction.choose(&preview)
    } else {
        ApprovalChoice::Deny
    };
    let decided_at = clock.now_ms();
    match choice {
        ApprovalChoice::ApproveOnce => match engine.approve(token, request, decided_at) {
            Ok(completed) => match completed.output {
                ToolOutput::ServiceStatus(result) if completed.verification.succeeded => {
                    outcome_with_result(0, Some(render_service_status(&result)), &engine)
                }
                _ => outcome(1, &engine),
            },
            Err(EngineError::Approval(ApprovalError::Expired)) => outcome(3, &engine),
            Err(_) => outcome(1, &engine),
        },
        ApprovalChoice::Deny => match engine.deny_approval(token, request, decided_at) {
            Ok(()) => outcome(2, &engine),
            Err(EngineError::Approval(ApprovalError::Expired)) => outcome(3, &engine),
            Err(_) => outcome(1, &engine),
        },
        ApprovalChoice::Cancel => match engine.cancel_approval(token, request, decided_at) {
            Ok(()) => outcome(2, &engine),
            Err(_) => outcome(1, &engine),
        },
    }
}

pub fn service_status_preview(request: &ToolRequest) -> String {
    let ToolRequest::ServicesReadStatus { selection, .. } = request else {
        return "Invalid service-status request".into();
    };
    format!(
        "Request: {}\nReason: read one exact loaded system service status\nPolicy decision: ask\nApproval: once only\nCapability: services.read:status\nExact unit: {}\nScope: local system manager\nDestination: org.freedesktop.systemd1\nCalls: GetUnit, then Id, LoadState, ActiveState, SubState only\nDeadline: 3000 ms for the complete D-Bus operation\nCommand: none (native D-Bus)\nListing, loading, GetAll, mutation, service logs, process data, network, and privilege: denied",
        request.request_id().as_str(),
        selection.unit,
    )
}

pub fn exact_preview(request: &ToolRequest) -> String {
    let command = command_for(request).expect("approval preview is command-backed");
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

fn outcome<
    E: Executor,
    O: OsIdentityProvider,
    U: UptimeProvider,
    M: MemorySummaryProvider,
    S: StorageSummaryProvider,
    P: ProcessSelfProvider,
    L: ProcessListProvider,
    F: FileContentProvider,
    W: WorkspaceCreateProvider,
    V: ServiceStatusProvider,
>(
    exit_code: i32,
    engine: &BlossomEngine<E, O, U, M, S, P, L, F, W, V>,
) -> RunOutcome {
    outcome_with_result(exit_code, None, engine)
}

fn outcome_with_result<
    E: Executor,
    O: OsIdentityProvider,
    U: UptimeProvider,
    M: MemorySummaryProvider,
    S: StorageSummaryProvider,
    P: ProcessSelfProvider,
    L: ProcessListProvider,
    F: FileContentProvider,
    W: WorkspaceCreateProvider,
    V: ServiceStatusProvider,
>(
    exit_code: i32,
    result: Option<String>,
    engine: &BlossomEngine<E, O, U, M, S, P, L, F, W, V>,
) -> RunOutcome {
    RunOutcome {
        exit_code,
        result,
        activity: render_activity(engine.audit()),
    }
}

fn render_os_identity(identity: &OsIdentity) -> String {
    let mut output = String::from("OS identity\n");
    for (key, value) in [
        ("ID", &identity.id),
        ("NAME", &identity.name),
        ("PRETTY_NAME", &identity.pretty_name),
        ("VERSION_ID", &identity.version_id),
        ("VERSION_CODENAME", &identity.version_codename),
        ("BUILD_ID", &identity.build_id),
        ("VARIANT_ID", &identity.variant_id),
    ] {
        if let Some(value) = value {
            writeln!(&mut output, "  {key}: {value}").expect("writing to a String cannot fail");
        }
    }
    writeln!(&mut output, "  Source: {}", identity.source_path)
        .expect("writing to a String cannot fail");
    writeln!(&mut output, "  SHA-256: {}", identity.source_sha256)
        .expect("writing to a String cannot fail");
    output
}

fn render_uptime(uptime: &SystemUptime) -> String {
    let days = uptime.seconds / 86_400;
    let hours = (uptime.seconds % 86_400) / 3_600;
    let minutes = (uptime.seconds % 3_600) / 60;
    let seconds = uptime.seconds % 60;
    let milliseconds = uptime.nanoseconds / 1_000_000;
    format!(
        "System uptime\n  Duration: {days} days {hours:02}:{minutes:02}:{seconds:02}.{milliseconds:03}\n  Source: {}\n  SHA-256: {}\n",
        uptime.source_path, uptime.source_sha256
    )
}

fn render_memory_summary(summary: &MemorySummary) -> String {
    let gib = |bytes: u64| bytes as f64 / 1_073_741_824_f64;
    format!(
        "Memory summary\n  Total: {:.2} GiB\n  Available: {:.2} GiB\n  Swap total: {:.2} GiB\n  Swap free: {:.2} GiB\n  Source: {}\n  SHA-256: {}\n",
        gib(summary.total_bytes),
        gib(summary.available_bytes),
        gib(summary.swap_total_bytes),
        gib(summary.swap_free_bytes),
        summary.source_path,
        summary.source_sha256
    )
}

fn render_storage_summary(summary: &StorageSummary) -> String {
    let gib = |bytes: u64| bytes as f64 / 1_073_741_824_f64;
    format!(
        "Root storage summary\n  Total: {:.2} GiB\n  Available to this user: {:.2} GiB\n  Scope: {}\n  Source: statvfs\n",
        gib(summary.total_bytes),
        gib(summary.available_bytes),
        summary.resource_path
    )
}

fn render_process_self(identity: &ProcessSelf) -> String {
    format!(
        "Blossom process identity\n  PID: {}\n  Parent PID: {}\n  Effective user ID: {}\n  Effective group ID: {}\n  Source: native process identity APIs\n",
        identity.process_id,
        identity.parent_process_id,
        identity.effective_user_id,
        identity.effective_group_id
    )
}

fn render_process_list(list: &ProcessList) -> String {
    let mut output = String::from("Same-user processes\n");
    for entry in &list.processes {
        writeln!(
            &mut output,
            "  {}  {}  {:?}",
            entry.process_id, entry.name, entry.state
        )
        .expect("writing to a String cannot fail");
    }
    writeln!(&mut output, "  Returned: {}", list.processes.len()).expect("string write");
    writeln!(
        &mut output,
        "  Skipped during bounded read: {}",
        list.skipped_entries
    )
    .expect("string write");
    writeln!(&mut output, "  Truncated: {}", list.truncated).expect("string write");
    output
}

fn render_file_content(result: &FileContent) -> String {
    let escaped = serde_json::to_string(&result.content).expect("UTF-8 content is serializable");
    format!(
        "Selected text file\n  Path: {}\n  Bytes: {}\n  SHA-256: {}\n  Content (JSON escaped): {}\n",
        result.selection.absolute_path, result.source_bytes, result.source_sha256, escaped
    )
}

fn render_workspace_created(result: &WorkspaceFileCreated) -> String {
    format!(
        "Workspace file creation\n  Workspace: {}\n  Destination: {}\n  Bytes: {}\n  SHA-256: {}\n  Permissions: {:04o}\n  State: {:?}\n",
        result.workspace_root,
        result.relative_destination,
        result.source_bytes,
        result.source_sha256,
        result.mode,
        result.state,
    )
}

fn render_service_status(result: &ServiceStatus) -> String {
    format!(
        "Service status observation\n  Requested unit: {}\n  Canonical unit: {}\n  Load state: {}\n  Active state: {}\n  Sub-state: {}\n  Scope: system\n  Source: systemd D-Bus\n  Note: observed state may change after this response\n",
        result.requested_unit,
        result.canonical_unit,
        result.load_state,
        result.active_state,
        result.sub_state,
    )
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
        AuditEvent::NativeReadStarted {
            request_id,
            resource,
        } => format!("request {request_id} started native read of {resource}"),
        AuditEvent::OsIdentityReadFinished {
            request_id,
            source_path,
            source_sha256,
            source_bytes,
        } => format!(
            "request {request_id} read {source_bytes} bytes from {source_path}, sha256={source_sha256}"
        ),
        AuditEvent::UptimeReadFinished {
            request_id,
            source_path,
            source_sha256,
            source_bytes,
        } => format!(
            "request {request_id} read {source_bytes} bytes from {source_path}, sha256={source_sha256}"
        ),
        AuditEvent::MemorySummaryReadFinished {
            request_id,
            source_path,
            source_sha256,
            source_bytes,
        } => format!(
            "request {request_id} read {source_bytes} bytes from {source_path}, sha256={source_sha256}"
        ),
        AuditEvent::StorageSummaryReadFinished {
            request_id,
            resource_path,
            source,
        } => format!("request {request_id} read {resource_path} storage summary via {source}"),
        AuditEvent::ProcessSelfReadFinished { request_id, source } => {
            format!("request {request_id} read its own process identity via {source}")
        }
        AuditEvent::ProcessListReadFinished {
            request_id,
            source,
            returned_entries,
            skipped_entries,
            truncated,
        } => {
            format!(
                "request {request_id} read {returned_entries} same-user process records via {source}; skipped={skipped_entries}, truncated={truncated}"
            )
        }
        AuditEvent::FileContentReadFinished {
            request_id,
            path_sha256,
            device,
            inode,
            source_bytes,
            source_sha256,
        } => {
            format!(
                "request {request_id} read {source_bytes} bytes from selected file path_sha256={path_sha256}, device={device}, inode={inode}, content_sha256={source_sha256}"
            )
        }
        AuditEvent::WorkspaceCreateStarted {
            request_id,
            workspace_sha256,
            destination_sha256,
            root_device,
            root_inode,
            parent_device,
            parent_inode,
            source_bytes,
            source_sha256,
        } => format!(
            "request {request_id} started workspace create root_sha256={workspace_sha256}, destination_sha256={destination_sha256}, root={root_device}:{root_inode}, parent={parent_device}:{parent_inode}, bytes={source_bytes}, content_sha256={source_sha256}"
        ),
        AuditEvent::WorkspaceCreateFinished {
            request_id,
            workspace_sha256,
            destination_sha256,
            created_device,
            created_inode,
            source_bytes,
            source_sha256,
            state,
        } => format!(
            "request {request_id} workspace create finished root_sha256={workspace_sha256}, destination_sha256={destination_sha256}, created={created_device}:{created_inode}, bytes={source_bytes}, content_sha256={source_sha256}, state={state:?}"
        ),
        AuditEvent::ServiceStatusReadFinished {
            request_id,
            requested_unit_sha256,
            canonical_unit_sha256,
            scope,
            provider,
        } => format!(
            "request {request_id} service status finished requested_sha256={requested_unit_sha256}, canonical_sha256={canonical_unit_sha256}, scope={scope}, provider={provider}"
        ),
        AuditEvent::NativeReadFailed {
            request_id,
            resource,
            error,
        } => format!("request {request_id} native read of {resource} failed ({error})"),
        AuditEvent::UptimeReadFailed {
            request_id,
            resource,
            error,
        } => format!("request {request_id} native read of {resource} failed ({error})"),
        AuditEvent::MemorySummaryReadFailed {
            request_id,
            resource,
            error,
        } => format!("request {request_id} native read of {resource} failed ({error})"),
        AuditEvent::StorageSummaryReadFailed {
            request_id,
            resource,
            error,
        } => format!("request {request_id} native read of {resource} failed ({error})"),
        AuditEvent::ProcessSelfReadFailed {
            request_id,
            resource,
            error,
        } => format!("request {request_id} native read of {resource} failed ({error})"),
        AuditEvent::ProcessListReadFailed {
            request_id,
            resource,
            error,
        } => {
            format!("request {request_id} native read of {resource} failed ({error})")
        }
        AuditEvent::FileContentReadFailed {
            request_id,
            path_sha256,
            error,
        } => {
            format!(
                "request {request_id} selected file read failed for path_sha256={path_sha256} ({error})"
            )
        }
        AuditEvent::WorkspaceCreateFailed {
            request_id,
            workspace_sha256,
            destination_sha256,
            error,
        } => format!(
            "request {request_id} workspace create failed root_sha256={workspace_sha256}, destination_sha256={destination_sha256} ({error})"
        ),
        AuditEvent::ServiceStatusReadFailed {
            request_id,
            requested_unit_sha256,
            error,
        } => format!(
            "request {request_id} service status failed unit_sha256={requested_unit_sha256} ({error})"
        ),
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
