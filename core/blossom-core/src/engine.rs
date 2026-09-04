use crate::approval::{ApprovalError, ApprovalStore, ApprovalToken};
use crate::audit::{AuditEvent, AuditLog};
use crate::executor::{CommandSpec, Executor, ExecutorError};
use crate::file_read::{
    FileContent, FileContentProvider, FileReadError, UnavailableFileContentProvider,
};
use crate::memory_summary::{
    MemorySummary, MemorySummaryError, MemorySummaryProvider, UnavailableMemorySummaryProvider,
};
use crate::orchestration::TypedRequestEngine;
use crate::os_identity::{
    OsIdentity, OsIdentityError, OsIdentityProvider, UnavailableOsIdentityProvider,
};
use crate::policy::{PolicyDecision, PolicyEngine};
use crate::process_list::{
    ProcessList, ProcessListError, ProcessListProvider, UnavailableProcessListProvider,
};
use crate::process_self::{
    ProcessSelf, ProcessSelfError, ProcessSelfProvider, UnavailableProcessSelfProvider,
};
use crate::request::{RequestError, ToolRequest};
use crate::service_status::{
    ServiceStatus, ServiceStatusError, ServiceStatusProvider, UnavailableServiceStatusProvider,
};
use crate::storage_summary::{
    StorageSummary, StorageSummaryError, StorageSummaryProvider, UnavailableStorageSummaryProvider,
};
use crate::uptime::{SystemUptime, UnavailableUptimeProvider, UptimeError, UptimeProvider};
use crate::verification::{
    Verification, verify_execution, verify_file_content, verify_memory_summary, verify_os_identity,
    verify_process_list, verify_process_self, verify_service_status, verify_storage_summary,
    verify_uptime, verify_workspace_file_created,
};
use crate::workspace_create::{
    UnavailableWorkspaceCreateProvider, WorkspaceCreateError, WorkspaceCreateProvider,
    WorkspaceFileCreated,
};

#[derive(Debug)]
pub struct BlossomEngine<
    E,
    O = UnavailableOsIdentityProvider,
    U = UnavailableUptimeProvider,
    M = UnavailableMemorySummaryProvider,
    S = UnavailableStorageSummaryProvider,
    P = UnavailableProcessSelfProvider,
    L = UnavailableProcessListProvider,
    F = UnavailableFileContentProvider,
    W = UnavailableWorkspaceCreateProvider,
    V = UnavailableServiceStatusProvider,
> {
    policy: PolicyEngine,
    approvals: ApprovalStore,
    executor: E,
    os_identity: O,
    uptime: U,
    memory_summary: M,
    storage_summary: S,
    process_self: P,
    process_list: L,
    file_content: F,
    workspace_create: W,
    service_status: V,
    audit: AuditLog,
}

impl<E: Executor>
    BlossomEngine<
        E,
        UnavailableOsIdentityProvider,
        UnavailableUptimeProvider,
        UnavailableMemorySummaryProvider,
        UnavailableStorageSummaryProvider,
        UnavailableProcessSelfProvider,
        UnavailableProcessListProvider,
        UnavailableFileContentProvider,
        UnavailableWorkspaceCreateProvider,
    >
{
    pub fn new(policy: PolicyEngine, approvals: ApprovalStore, executor: E) -> Self {
        Self {
            policy,
            approvals,
            executor,
            os_identity: UnavailableOsIdentityProvider,
            uptime: UnavailableUptimeProvider,
            memory_summary: UnavailableMemorySummaryProvider,
            storage_summary: UnavailableStorageSummaryProvider,
            process_self: UnavailableProcessSelfProvider,
            process_list: UnavailableProcessListProvider,
            file_content: UnavailableFileContentProvider::default(),
            workspace_create: UnavailableWorkspaceCreateProvider::default(),
            service_status: UnavailableServiceStatusProvider,
            audit: AuditLog::default(),
        }
    }
}

impl<E: Executor, O: OsIdentityProvider>
    BlossomEngine<E, O, UnavailableUptimeProvider, UnavailableMemorySummaryProvider>
{
    pub fn with_os_identity(
        policy: PolicyEngine,
        approvals: ApprovalStore,
        executor: E,
        os_identity: O,
    ) -> Self {
        Self {
            policy,
            approvals,
            executor,
            os_identity,
            uptime: UnavailableUptimeProvider,
            memory_summary: UnavailableMemorySummaryProvider,
            storage_summary: UnavailableStorageSummaryProvider,
            process_self: UnavailableProcessSelfProvider,
            process_list: UnavailableProcessListProvider,
            file_content: UnavailableFileContentProvider::default(),
            workspace_create: UnavailableWorkspaceCreateProvider::default(),
            service_status: UnavailableServiceStatusProvider,
            audit: AuditLog::default(),
        }
    }
}

impl<E: Executor, U: UptimeProvider>
    BlossomEngine<E, UnavailableOsIdentityProvider, U, UnavailableMemorySummaryProvider>
{
    pub fn with_uptime(
        policy: PolicyEngine,
        approvals: ApprovalStore,
        executor: E,
        uptime: U,
    ) -> Self {
        Self {
            policy,
            approvals,
            executor,
            os_identity: UnavailableOsIdentityProvider,
            uptime,
            memory_summary: UnavailableMemorySummaryProvider,
            storage_summary: UnavailableStorageSummaryProvider,
            process_self: UnavailableProcessSelfProvider,
            process_list: UnavailableProcessListProvider,
            file_content: UnavailableFileContentProvider::default(),
            workspace_create: UnavailableWorkspaceCreateProvider::default(),
            service_status: UnavailableServiceStatusProvider,
            audit: AuditLog::default(),
        }
    }
}

impl<E: Executor, M: MemorySummaryProvider>
    BlossomEngine<E, UnavailableOsIdentityProvider, UnavailableUptimeProvider, M>
{
    pub fn with_memory_summary(
        policy: PolicyEngine,
        approvals: ApprovalStore,
        executor: E,
        memory_summary: M,
    ) -> Self {
        Self {
            policy,
            approvals,
            executor,
            os_identity: UnavailableOsIdentityProvider,
            uptime: UnavailableUptimeProvider,
            memory_summary,
            storage_summary: UnavailableStorageSummaryProvider,
            process_self: UnavailableProcessSelfProvider,
            process_list: UnavailableProcessListProvider,
            file_content: UnavailableFileContentProvider::default(),
            workspace_create: UnavailableWorkspaceCreateProvider::default(),
            service_status: UnavailableServiceStatusProvider,
            audit: AuditLog::default(),
        }
    }
}

impl<E: Executor, S: StorageSummaryProvider>
    BlossomEngine<
        E,
        UnavailableOsIdentityProvider,
        UnavailableUptimeProvider,
        UnavailableMemorySummaryProvider,
        S,
    >
{
    pub fn with_storage_summary(
        policy: PolicyEngine,
        approvals: ApprovalStore,
        executor: E,
        storage_summary: S,
    ) -> Self {
        Self {
            policy,
            approvals,
            executor,
            os_identity: UnavailableOsIdentityProvider,
            uptime: UnavailableUptimeProvider,
            memory_summary: UnavailableMemorySummaryProvider,
            storage_summary,
            process_self: UnavailableProcessSelfProvider,
            process_list: UnavailableProcessListProvider,
            file_content: UnavailableFileContentProvider::default(),
            workspace_create: UnavailableWorkspaceCreateProvider::default(),
            service_status: UnavailableServiceStatusProvider,
            audit: AuditLog::default(),
        }
    }
}

impl<E: Executor, P: ProcessSelfProvider>
    BlossomEngine<
        E,
        UnavailableOsIdentityProvider,
        UnavailableUptimeProvider,
        UnavailableMemorySummaryProvider,
        UnavailableStorageSummaryProvider,
        P,
        UnavailableProcessListProvider,
        UnavailableFileContentProvider,
        UnavailableWorkspaceCreateProvider,
    >
{
    pub fn with_process_self(
        policy: PolicyEngine,
        approvals: ApprovalStore,
        executor: E,
        process_self: P,
    ) -> Self {
        Self {
            policy,
            approvals,
            executor,
            os_identity: UnavailableOsIdentityProvider,
            uptime: UnavailableUptimeProvider,
            memory_summary: UnavailableMemorySummaryProvider,
            storage_summary: UnavailableStorageSummaryProvider,
            process_self,
            process_list: UnavailableProcessListProvider,
            file_content: UnavailableFileContentProvider::default(),
            workspace_create: UnavailableWorkspaceCreateProvider::default(),
            service_status: UnavailableServiceStatusProvider,
            audit: AuditLog::default(),
        }
    }
}

impl<E: Executor, L: ProcessListProvider>
    BlossomEngine<
        E,
        UnavailableOsIdentityProvider,
        UnavailableUptimeProvider,
        UnavailableMemorySummaryProvider,
        UnavailableStorageSummaryProvider,
        UnavailableProcessSelfProvider,
        L,
        UnavailableFileContentProvider,
    >
{
    pub fn with_process_list(
        policy: PolicyEngine,
        approvals: ApprovalStore,
        executor: E,
        process_list: L,
    ) -> Self {
        Self {
            policy,
            approvals,
            executor,
            os_identity: UnavailableOsIdentityProvider,
            uptime: UnavailableUptimeProvider,
            memory_summary: UnavailableMemorySummaryProvider,
            storage_summary: UnavailableStorageSummaryProvider,
            process_self: UnavailableProcessSelfProvider,
            process_list,
            file_content: UnavailableFileContentProvider::default(),
            workspace_create: UnavailableWorkspaceCreateProvider::default(),
            service_status: UnavailableServiceStatusProvider,
            audit: AuditLog::default(),
        }
    }
}

impl<E: Executor, F: FileContentProvider>
    BlossomEngine<
        E,
        UnavailableOsIdentityProvider,
        UnavailableUptimeProvider,
        UnavailableMemorySummaryProvider,
        UnavailableStorageSummaryProvider,
        UnavailableProcessSelfProvider,
        UnavailableProcessListProvider,
        F,
        UnavailableWorkspaceCreateProvider,
    >
{
    pub fn with_file_content(
        policy: PolicyEngine,
        approvals: ApprovalStore,
        executor: E,
        file_content: F,
    ) -> Self {
        Self {
            policy,
            approvals,
            executor,
            os_identity: UnavailableOsIdentityProvider,
            uptime: UnavailableUptimeProvider,
            memory_summary: UnavailableMemorySummaryProvider,
            storage_summary: UnavailableStorageSummaryProvider,
            process_self: UnavailableProcessSelfProvider,
            process_list: UnavailableProcessListProvider,
            file_content,
            workspace_create: UnavailableWorkspaceCreateProvider::default(),
            service_status: UnavailableServiceStatusProvider,
            audit: AuditLog::default(),
        }
    }
}

impl<E: Executor, W: WorkspaceCreateProvider>
    BlossomEngine<
        E,
        UnavailableOsIdentityProvider,
        UnavailableUptimeProvider,
        UnavailableMemorySummaryProvider,
        UnavailableStorageSummaryProvider,
        UnavailableProcessSelfProvider,
        UnavailableProcessListProvider,
        UnavailableFileContentProvider,
        W,
    >
{
    pub fn with_workspace_create(
        policy: PolicyEngine,
        approvals: ApprovalStore,
        executor: E,
        workspace_create: W,
    ) -> Self {
        Self {
            policy,
            approvals,
            executor,
            os_identity: UnavailableOsIdentityProvider,
            uptime: UnavailableUptimeProvider,
            memory_summary: UnavailableMemorySummaryProvider,
            storage_summary: UnavailableStorageSummaryProvider,
            process_self: UnavailableProcessSelfProvider,
            process_list: UnavailableProcessListProvider,
            file_content: UnavailableFileContentProvider::default(),
            workspace_create,
            service_status: UnavailableServiceStatusProvider,
            audit: AuditLog::default(),
        }
    }
}

impl<E: Executor, V: ServiceStatusProvider>
    BlossomEngine<
        E,
        UnavailableOsIdentityProvider,
        UnavailableUptimeProvider,
        UnavailableMemorySummaryProvider,
        UnavailableStorageSummaryProvider,
        UnavailableProcessSelfProvider,
        UnavailableProcessListProvider,
        UnavailableFileContentProvider,
        UnavailableWorkspaceCreateProvider,
        V,
    >
{
    pub fn with_service_status(
        policy: PolicyEngine,
        approvals: ApprovalStore,
        executor: E,
        service_status: V,
    ) -> Self {
        Self {
            policy,
            approvals,
            executor,
            os_identity: UnavailableOsIdentityProvider,
            uptime: UnavailableUptimeProvider,
            memory_summary: UnavailableMemorySummaryProvider,
            storage_summary: UnavailableStorageSummaryProvider,
            process_self: UnavailableProcessSelfProvider,
            process_list: UnavailableProcessListProvider,
            file_content: UnavailableFileContentProvider::default(),
            workspace_create: UnavailableWorkspaceCreateProvider::default(),
            service_status,
            audit: AuditLog::default(),
        }
    }
}

impl<
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
> BlossomEngine<E, O, U, M, S, P, L, F, W, V>
{
    pub fn begin(&mut self, input: &str, now_ms: u64) -> Result<BeginOutcome, EngineError> {
        let request = match ToolRequest::parse_json(input) {
            Ok(request) => request,
            Err(error) => {
                self.audit.append(AuditEvent::RequestRejected {
                    category: request_error_category(&error).into(),
                });
                return Err(EngineError::InvalidRequest(error));
            }
        };
        self.begin_request(request, now_ms)
    }

    /// Starts one already-validated typed request through the same policy,
    /// approval, execution, verification, and audit path as JSON input.
    pub fn begin_request(
        &mut self,
        request: ToolRequest,
        now_ms: u64,
    ) -> Result<BeginOutcome, EngineError> {
        self.audit.append(AuditEvent::RequestAccepted {
            request_id: request.request_id().as_str().into(),
            tool: request.tool_name().into(),
        });
        let capability = PolicyEngine::required_capability(&request);
        let decision = self.policy.evaluate(&request);
        self.audit.append(AuditEvent::PolicyEvaluated {
            request_id: request.request_id().as_str().into(),
            capability,
            decision,
        });
        match decision {
            PolicyDecision::Deny => {
                self.audit.append(AuditEvent::Denied {
                    request_id: request.request_id().as_str().into(),
                });
                Ok(BeginOutcome::Denied)
            }
            PolicyDecision::Ask => {
                let token = self.approvals.issue(request.clone(), now_ms);
                self.audit.append(AuditEvent::ApprovalIssued {
                    request_id: request.request_id().as_str().into(),
                });
                Ok(BeginOutcome::ApprovalRequired { request, token })
            }
            PolicyDecision::Allow => self.execute(request).map(BeginOutcome::Completed),
        }
    }

    pub fn approve(
        &mut self,
        token: ApprovalToken,
        request: ToolRequest,
        now_ms: u64,
    ) -> Result<CompletionOutcome, EngineError> {
        if let Err(error) = self.approvals.consume(token, &request, now_ms) {
            self.audit.append(AuditEvent::ApprovalRejected {
                request_id: request.request_id().as_str().into(),
                error,
            });
            return Err(EngineError::Approval(error));
        }
        self.audit.append(AuditEvent::ApprovalConsumed {
            request_id: request.request_id().as_str().into(),
        });
        self.execute(request)
    }

    pub fn deny_approval(
        &mut self,
        token: ApprovalToken,
        request: ToolRequest,
        now_ms: u64,
    ) -> Result<(), EngineError> {
        if let Err(error) = self.approvals.consume(token, &request, now_ms) {
            self.audit.append(AuditEvent::ApprovalRejected {
                request_id: request.request_id().as_str().into(),
                error,
            });
            return Err(EngineError::Approval(error));
        }
        self.audit.append(AuditEvent::ApprovalDenied {
            request_id: request.request_id().as_str().into(),
        });
        Ok(())
    }

    pub fn cancel_approval(
        &mut self,
        token: ApprovalToken,
        request: ToolRequest,
        now_ms: u64,
    ) -> Result<(), EngineError> {
        if let Err(error) = self.approvals.consume(token, &request, now_ms) {
            self.audit.append(AuditEvent::ApprovalRejected {
                request_id: request.request_id().as_str().into(),
                error,
            });
            if error != ApprovalError::Expired {
                return Err(EngineError::Approval(error));
            }
        }
        self.audit.append(AuditEvent::ApprovalCancelled {
            request_id: request.request_id().as_str().into(),
        });
        Ok(())
    }

    pub fn audit(&self) -> &AuditLog {
        &self.audit
    }

    fn execute(&mut self, request: ToolRequest) -> Result<CompletionOutcome, EngineError> {
        match request {
            ToolRequest::SystemUname { .. } => self.execute_command(request),
            ToolRequest::SystemOsIdentity { .. } => self.execute_os_identity(request),
            ToolRequest::SystemUptime { .. } => self.execute_uptime(request),
            ToolRequest::SystemMemorySummary { .. } => self.execute_memory_summary(request),
            ToolRequest::SystemStorageSummary { .. } => self.execute_storage_summary(request),
            ToolRequest::ProcessSelf { .. } => self.execute_process_self(request),
            ToolRequest::ProcessList { .. } => self.execute_process_list(request),
            ToolRequest::FilesReadContent { .. } => self.execute_file_content(request),
            ToolRequest::FilesWriteCreate { .. } => self.execute_workspace_create(request),
            ToolRequest::ServicesReadStatus { .. } => self.execute_service_status(request),
        }
    }

    fn execute_command(&mut self, request: ToolRequest) -> Result<CompletionOutcome, EngineError> {
        let command = command_for(&request).expect("command request has a fixed command");
        self.audit.append(AuditEvent::ExecutionStarted {
            request_id: request.request_id().as_str().into(),
            program: command.program.display().to_string(),
        });
        let result = match self.executor.execute(&command) {
            Ok(result) => result,
            Err(error) => {
                self.audit.append(AuditEvent::ExecutionFailed {
                    request_id: request.request_id().as_str().into(),
                    error: error.clone(),
                });
                return Err(EngineError::Executor(error));
            }
        };
        self.audit
            .append(AuditEvent::execution_finished(&request, &result));
        let verification = verify_execution(&result);
        self.audit.append(AuditEvent::VerificationFinished {
            request_id: request.request_id().as_str().into(),
            verification: verification.clone(),
        });
        Ok(CompletionOutcome {
            request,
            verification,
            output: ToolOutput::SystemUname,
        })
    }

    fn execute_os_identity(
        &mut self,
        request: ToolRequest,
    ) -> Result<CompletionOutcome, EngineError> {
        self.audit.append(AuditEvent::NativeReadStarted {
            request_id: request.request_id().as_str().into(),
            resource: "os.identity".into(),
        });
        let identity = match self.os_identity.read_os_identity() {
            Ok(identity) => identity,
            Err(error) => {
                self.audit.append(AuditEvent::NativeReadFailed {
                    request_id: request.request_id().as_str().into(),
                    resource: "os.identity".into(),
                    error,
                });
                return Err(EngineError::OsIdentity(error));
            }
        };
        self.audit
            .append(AuditEvent::os_identity_finished(&request, &identity));
        let verification = verify_os_identity(&identity);
        self.audit.append(AuditEvent::VerificationFinished {
            request_id: request.request_id().as_str().into(),
            verification: verification.clone(),
        });
        Ok(CompletionOutcome {
            request,
            verification,
            output: ToolOutput::OsIdentity(Box::new(identity)),
        })
    }

    fn execute_uptime(&mut self, request: ToolRequest) -> Result<CompletionOutcome, EngineError> {
        self.audit.append(AuditEvent::NativeReadStarted {
            request_id: request.request_id().as_str().into(),
            resource: "uptime".into(),
        });
        let uptime = match self.uptime.read_uptime() {
            Ok(uptime) => uptime,
            Err(error) => {
                self.audit.append(AuditEvent::UptimeReadFailed {
                    request_id: request.request_id().as_str().into(),
                    resource: "uptime".into(),
                    error,
                });
                return Err(EngineError::Uptime(error));
            }
        };
        self.audit
            .append(AuditEvent::uptime_finished(&request, &uptime));
        let verification = verify_uptime(&uptime);
        self.audit.append(AuditEvent::VerificationFinished {
            request_id: request.request_id().as_str().into(),
            verification: verification.clone(),
        });
        Ok(CompletionOutcome {
            request,
            verification,
            output: ToolOutput::Uptime(uptime),
        })
    }

    fn execute_memory_summary(
        &mut self,
        request: ToolRequest,
    ) -> Result<CompletionOutcome, EngineError> {
        self.audit.append(AuditEvent::NativeReadStarted {
            request_id: request.request_id().as_str().into(),
            resource: "memory.summary".into(),
        });
        let summary = match self.memory_summary.read_memory_summary() {
            Ok(summary) => summary,
            Err(error) => {
                self.audit.append(AuditEvent::MemorySummaryReadFailed {
                    request_id: request.request_id().as_str().into(),
                    resource: "memory.summary".into(),
                    error,
                });
                return Err(EngineError::MemorySummary(error));
            }
        };
        self.audit
            .append(AuditEvent::memory_summary_finished(&request, &summary));
        let verification = verify_memory_summary(&summary);
        self.audit.append(AuditEvent::VerificationFinished {
            request_id: request.request_id().as_str().into(),
            verification: verification.clone(),
        });
        Ok(CompletionOutcome {
            request,
            verification,
            output: ToolOutput::MemorySummary(summary),
        })
    }

    fn execute_storage_summary(
        &mut self,
        request: ToolRequest,
    ) -> Result<CompletionOutcome, EngineError> {
        self.audit.append(AuditEvent::NativeReadStarted {
            request_id: request.request_id().as_str().into(),
            resource: "storage.summary:/".into(),
        });
        let summary = match self.storage_summary.read_storage_summary() {
            Ok(summary) => summary,
            Err(error) => {
                self.audit.append(AuditEvent::StorageSummaryReadFailed {
                    request_id: request.request_id().as_str().into(),
                    resource: "storage.summary:/".into(),
                    error,
                });
                return Err(EngineError::StorageSummary(error));
            }
        };
        self.audit
            .append(AuditEvent::storage_summary_finished(&request, &summary));
        let verification = verify_storage_summary(&summary);
        self.audit.append(AuditEvent::VerificationFinished {
            request_id: request.request_id().as_str().into(),
            verification: verification.clone(),
        });
        Ok(CompletionOutcome {
            request,
            verification,
            output: ToolOutput::StorageSummary(summary),
        })
    }

    fn execute_process_self(
        &mut self,
        request: ToolRequest,
    ) -> Result<CompletionOutcome, EngineError> {
        self.audit.append(AuditEvent::NativeReadStarted {
            request_id: request.request_id().as_str().into(),
            resource: "process.self".into(),
        });
        let identity = match self.process_self.read_process_self() {
            Ok(identity) => identity,
            Err(error) => {
                self.audit.append(AuditEvent::ProcessSelfReadFailed {
                    request_id: request.request_id().as_str().into(),
                    resource: "process.self".into(),
                    error,
                });
                return Err(EngineError::ProcessSelf(error));
            }
        };
        self.audit
            .append(AuditEvent::process_self_finished(&request, &identity));
        let verification = verify_process_self(&identity);
        self.audit.append(AuditEvent::VerificationFinished {
            request_id: request.request_id().as_str().into(),
            verification: verification.clone(),
        });
        Ok(CompletionOutcome {
            request,
            verification,
            output: ToolOutput::ProcessSelf(identity),
        })
    }

    fn execute_process_list(
        &mut self,
        request: ToolRequest,
    ) -> Result<CompletionOutcome, EngineError> {
        self.audit.append(AuditEvent::NativeReadStarted {
            request_id: request.request_id().as_str().into(),
            resource: "process.list:same-effective-user".into(),
        });
        let list = match self.process_list.read_process_list() {
            Ok(list) => list,
            Err(error) => {
                self.audit.append(AuditEvent::ProcessListReadFailed {
                    request_id: request.request_id().as_str().into(),
                    resource: "process.list:same-effective-user".into(),
                    error,
                });
                return Err(EngineError::ProcessList(error));
            }
        };
        self.audit
            .append(AuditEvent::process_list_finished(&request, &list));
        let verification = verify_process_list(&list);
        self.audit.append(AuditEvent::VerificationFinished {
            request_id: request.request_id().as_str().into(),
            verification: verification.clone(),
        });
        Ok(CompletionOutcome {
            request,
            verification,
            output: ToolOutput::ProcessList(list),
        })
    }

    fn execute_file_content(
        &mut self,
        request: ToolRequest,
    ) -> Result<CompletionOutcome, EngineError> {
        let selection = match &request {
            ToolRequest::FilesReadContent { selection, .. } => selection.clone(),
            _ => unreachable!("file content execution requires a file request"),
        };
        let path_sha256 = crate::audit::digest_bytes(selection.absolute_path.as_bytes());
        self.audit.append(AuditEvent::NativeReadStarted {
            request_id: request.request_id().as_str().into(),
            resource: format!("file.content:sha256:{path_sha256}"),
        });
        let result = match self.file_content.read_selected_file(&selection) {
            Ok(result) => result,
            Err(error) => {
                self.audit.append(AuditEvent::FileContentReadFailed {
                    request_id: request.request_id().as_str().into(),
                    path_sha256,
                    error,
                });
                return Err(EngineError::FileContent(error));
            }
        };
        self.audit
            .append(AuditEvent::file_content_finished(&request, &result));
        let verification = verify_file_content(&result);
        self.audit.append(AuditEvent::VerificationFinished {
            request_id: request.request_id().as_str().into(),
            verification: verification.clone(),
        });
        Ok(CompletionOutcome {
            request,
            verification,
            output: ToolOutput::FileContent(Box::new(result)),
        })
    }

    fn execute_workspace_create(
        &mut self,
        request: ToolRequest,
    ) -> Result<CompletionOutcome, EngineError> {
        let selection = match &request {
            ToolRequest::FilesWriteCreate { selection, .. } => selection.clone(),
            _ => unreachable!("workspace creation requires a create request"),
        };
        self.audit
            .append(AuditEvent::workspace_create_started(&request, &selection));
        let result = match self.workspace_create.create_selected_file(&selection) {
            Ok(result) => result,
            Err(error) => {
                self.audit.append(AuditEvent::WorkspaceCreateFailed {
                    request_id: request.request_id().as_str().into(),
                    workspace_sha256: crate::audit::digest_bytes(
                        selection.workspace_root.as_bytes(),
                    ),
                    destination_sha256: crate::audit::digest_bytes(
                        selection.relative_destination.as_bytes(),
                    ),
                    error,
                });
                return Err(EngineError::WorkspaceCreate(error));
            }
        };
        self.audit
            .append(AuditEvent::workspace_create_finished(&request, &result));
        let verification = verify_workspace_file_created(&result, &selection);
        self.audit.append(AuditEvent::VerificationFinished {
            request_id: request.request_id().as_str().into(),
            verification: verification.clone(),
        });
        Ok(CompletionOutcome {
            request,
            verification,
            output: ToolOutput::WorkspaceFileCreated(Box::new(result)),
        })
    }

    fn execute_service_status(
        &mut self,
        request: ToolRequest,
    ) -> Result<CompletionOutcome, EngineError> {
        let unit = match &request {
            ToolRequest::ServicesReadStatus { selection, .. } => selection.unit.clone(),
            _ => unreachable!("service status requires an exact service request"),
        };
        let unit_sha256 = crate::audit::digest_bytes(unit.as_bytes());
        self.audit.append(AuditEvent::NativeReadStarted {
            request_id: request.request_id().as_str().into(),
            resource: format!("service.status:sha256:{unit_sha256}"),
        });
        let result = match self.service_status.read_status(&unit) {
            Ok(result) => result,
            Err(error) => {
                self.audit.append(AuditEvent::ServiceStatusReadFailed {
                    request_id: request.request_id().as_str().into(),
                    requested_unit_sha256: unit_sha256,
                    error,
                });
                return Err(EngineError::ServiceStatus(error));
            }
        };
        self.audit
            .append(AuditEvent::service_status_finished(&request, &result));
        let verification = verify_service_status(&result, &unit);
        self.audit.append(AuditEvent::VerificationFinished {
            request_id: request.request_id().as_str().into(),
            verification: verification.clone(),
        });
        Ok(CompletionOutcome {
            request,
            verification,
            output: ToolOutput::ServiceStatus(Box::new(result)),
        })
    }
}

impl<
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
> TypedRequestEngine for BlossomEngine<E, O, U, M, S, P, L, F, W, V>
{
    fn record_orchestration(&mut self, event: AuditEvent) {
        self.audit.append(event);
    }

    fn begin_typed(
        &mut self,
        request: ToolRequest,
        now_ms: u64,
    ) -> Result<BeginOutcome, EngineError> {
        self.begin_request(request, now_ms)
    }

    fn approve_typed(
        &mut self,
        token: ApprovalToken,
        request: ToolRequest,
        now_ms: u64,
    ) -> Result<CompletionOutcome, EngineError> {
        self.approve(token, request, now_ms)
    }

    fn deny_typed(
        &mut self,
        token: ApprovalToken,
        request: ToolRequest,
        now_ms: u64,
    ) -> Result<(), EngineError> {
        self.deny_approval(token, request, now_ms)
    }

    fn cancel_typed(
        &mut self,
        token: ApprovalToken,
        request: ToolRequest,
        now_ms: u64,
    ) -> Result<(), EngineError> {
        self.cancel_approval(token, request, now_ms)
    }
}

pub fn command_for(request: &ToolRequest) -> Option<CommandSpec> {
    match request {
        ToolRequest::SystemUname { .. } => Some(CommandSpec::system_uname()),
        ToolRequest::SystemOsIdentity { .. } => None,
        ToolRequest::SystemUptime { .. } => None,
        ToolRequest::SystemMemorySummary { .. } => None,
        ToolRequest::SystemStorageSummary { .. } => None,
        ToolRequest::ProcessSelf { .. } => None,
        ToolRequest::ProcessList { .. } => None,
        ToolRequest::FilesReadContent { .. } => None,
        ToolRequest::FilesWriteCreate { .. } => None,
        ToolRequest::ServicesReadStatus { .. } => None,
    }
}

fn request_error_category(error: &RequestError) -> &'static str {
    match error {
        RequestError::RequestTooLarge => "request_too_large",
        RequestError::MalformedJson { .. } => "malformed_json",
        RequestError::InvalidRequestId => "invalid_request_id",
        RequestError::InvalidToolName => "invalid_tool_name",
        RequestError::UnknownTool { .. } => "unknown_tool",
        RequestError::InvalidArguments { .. } => "invalid_arguments",
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BeginOutcome {
    Denied,
    ApprovalRequired {
        request: ToolRequest,
        token: ApprovalToken,
    },
    Completed(CompletionOutcome),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompletionOutcome {
    pub request: ToolRequest,
    pub verification: Verification,
    pub output: ToolOutput,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToolOutput {
    SystemUname,
    OsIdentity(Box<OsIdentity>),
    Uptime(SystemUptime),
    MemorySummary(MemorySummary),
    StorageSummary(StorageSummary),
    ProcessSelf(ProcessSelf),
    ProcessList(ProcessList),
    FileContent(Box<FileContent>),
    WorkspaceFileCreated(Box<WorkspaceFileCreated>),
    ServiceStatus(Box<ServiceStatus>),
}

#[derive(Debug)]
pub enum EngineError {
    InvalidRequest(RequestError),
    Approval(ApprovalError),
    Executor(ExecutorError),
    OsIdentity(OsIdentityError),
    Uptime(UptimeError),
    MemorySummary(MemorySummaryError),
    StorageSummary(StorageSummaryError),
    ProcessSelf(ProcessSelfError),
    ProcessList(ProcessListError),
    FileContent(FileReadError),
    WorkspaceCreate(WorkspaceCreateError),
    ServiceStatus(ServiceStatusError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::ExecutionResult;
    use crate::policy::{Capability, PolicyRule};
    use std::collections::VecDeque;

    const REQUEST: &str = r#"{"request_id":"req-1","tool":"system.uname","arguments":{}}"#;

    #[derive(Debug)]
    struct ScriptedExecutor {
        outcomes: VecDeque<Result<ExecutionResult, ExecutorError>>,
        calls: Vec<CommandSpec>,
    }

    impl ScriptedExecutor {
        fn successful() -> Self {
            Self {
                outcomes: VecDeque::from([Ok(ExecutionResult {
                    exit_code: Some(0),
                    stdout: b"Linux\n".to_vec(),
                    stderr: Vec::new(),
                    timed_out: false,
                    output_truncated: false,
                })]),
                calls: Vec::new(),
            }
        }
    }

    impl Executor for ScriptedExecutor {
        fn execute(&mut self, command: &CommandSpec) -> Result<ExecutionResult, ExecutorError> {
            self.calls.push(command.clone());
            self.outcomes
                .pop_front()
                .unwrap_or(Err(ExecutorError::Failed))
        }
    }

    #[derive(Debug)]
    struct ScriptedOsIdentity {
        result: Option<Result<OsIdentity, OsIdentityError>>,
        calls: usize,
    }

    impl OsIdentityProvider for ScriptedOsIdentity {
        fn read_os_identity(&mut self) -> Result<OsIdentity, OsIdentityError> {
            self.calls += 1;
            self.result
                .take()
                .unwrap_or(Err(OsIdentityError::ReadFailed))
        }
    }

    #[derive(Debug)]
    struct ScriptedUptime {
        result: Option<Result<SystemUptime, UptimeError>>,
        calls: usize,
    }

    impl UptimeProvider for ScriptedUptime {
        fn read_uptime(&mut self) -> Result<SystemUptime, UptimeError> {
            self.calls += 1;
            self.result.take().unwrap_or(Err(UptimeError::ReadFailed))
        }
    }

    #[derive(Debug)]
    struct ScriptedMemorySummary {
        result: Option<Result<MemorySummary, MemorySummaryError>>,
        calls: usize,
    }

    impl MemorySummaryProvider for ScriptedMemorySummary {
        fn read_memory_summary(&mut self) -> Result<MemorySummary, MemorySummaryError> {
            self.calls += 1;
            self.result
                .take()
                .unwrap_or(Err(MemorySummaryError::ReadFailed))
        }
    }

    #[derive(Debug)]
    struct ScriptedStorageSummary {
        result: Option<Result<StorageSummary, StorageSummaryError>>,
        calls: usize,
    }

    impl StorageSummaryProvider for ScriptedStorageSummary {
        fn read_storage_summary(&mut self) -> Result<StorageSummary, StorageSummaryError> {
            self.calls += 1;
            self.result
                .take()
                .unwrap_or(Err(StorageSummaryError::StatFailed))
        }
    }

    #[derive(Debug)]
    struct ScriptedProcessSelf {
        result: Option<Result<ProcessSelf, ProcessSelfError>>,
        calls: usize,
    }

    impl ProcessSelfProvider for ScriptedProcessSelf {
        fn read_process_self(&mut self) -> Result<ProcessSelf, ProcessSelfError> {
            self.calls += 1;
            self.result
                .take()
                .unwrap_or(Err(ProcessSelfError::InvalidProcessId))
        }
    }

    fn process_self() -> ProcessSelf {
        ProcessSelf {
            source: crate::process_self::ProcessSelfSource::NativeProcessIdentity,
            process_id: 42,
            parent_process_id: 7,
            effective_user_id: 1000,
            effective_group_id: 1000,
        }
    }

    fn storage_summary() -> StorageSummary {
        StorageSummary {
            source: crate::storage_summary::StorageSummarySource::RootStatvfs,
            resource_path: "/".into(),
            total_bytes: 100,
            available_bytes: 25,
        }
    }

    fn memory_summary() -> MemorySummary {
        MemorySummary {
            total_bytes: 16 * 1024,
            available_bytes: 8 * 1024,
            swap_total_bytes: 4 * 1024,
            swap_free_bytes: 2 * 1024,
            source_path: "/proc/meminfo".into(),
            source_sha256: "c".repeat(64),
            source_bytes: 128,
        }
    }

    fn uptime() -> SystemUptime {
        SystemUptime {
            seconds: 42,
            nanoseconds: 250_000_000,
            source_path: "/proc/uptime".into(),
            source_sha256: "b".repeat(64),
            source_bytes: 16,
        }
    }

    fn os_identity() -> OsIdentity {
        OsIdentity {
            source: crate::os_identity::OsReleaseSource::EtcOsRelease,
            source_path: "/etc/os-release".into(),
            source_sha256: "a".repeat(64),
            source_bytes: 8,
            id: Some("arch".into()),
            name: Some("Arch Linux".into()),
            pretty_name: Some("Arch Linux".into()),
            version_id: None,
            version_codename: None,
            build_id: Some("rolling".into()),
            variant_id: None,
        }
    }

    fn engine(
        decision: PolicyDecision,
        executor: ScriptedExecutor,
    ) -> BlossomEngine<ScriptedExecutor> {
        BlossomEngine::new(
            PolicyEngine::new(vec![PolicyRule {
                capability: Capability::SystemReadKernelIdentity,
                decision,
            }]),
            ApprovalStore::new(100),
            executor,
        )
    }

    #[test]
    fn completes_ask_approve_execute_verify_audit_flow() {
        let mut engine = engine(PolicyDecision::Ask, ScriptedExecutor::successful());
        let (request, token) = match engine.begin(REQUEST, 1_000).expect("begin should work") {
            BeginOutcome::ApprovalRequired { request, token } => (request, token),
            outcome => panic!("unexpected begin outcome: {outcome:?}"),
        };
        let completed = engine
            .approve(token, request, 1_001)
            .expect("approval should work");
        assert!(completed.verification.succeeded);
        assert!(engine.audit().verify_chain());
        assert_eq!(engine.executor.calls.len(), 1);
        assert_eq!(
            engine.executor.calls[0].program.to_string_lossy(),
            "/usr/bin/uname"
        );
        assert!(!engine.executor.calls[0].network_allowed);
    }

    #[test]
    fn deny_never_calls_executor() {
        let mut engine = engine(PolicyDecision::Deny, ScriptedExecutor::successful());
        assert_eq!(
            engine.begin(REQUEST, 1_000).expect("begin should work"),
            BeginOutcome::Denied
        );
        assert!(engine.executor.calls.is_empty());
        assert!(engine.audit().verify_chain());
    }

    #[test]
    fn user_denial_consumes_approval_without_execution() {
        let mut engine = engine(PolicyDecision::Ask, ScriptedExecutor::successful());
        let (request, token) = match engine.begin(REQUEST, 1_000).expect("begin should work") {
            BeginOutcome::ApprovalRequired { request, token } => (request, token),
            outcome => panic!("unexpected begin outcome: {outcome:?}"),
        };
        engine
            .deny_approval(token, request.clone(), 1_001)
            .expect("denial should consume approval");
        assert!(engine.executor.calls.is_empty());
        assert!(matches!(
            engine.approve(token, request, 1_002),
            Err(EngineError::Approval(ApprovalError::Replay))
        ));
        assert!(engine.audit().verify_chain());
    }

    #[test]
    fn cancellation_consumes_approval_without_execution() {
        let mut engine = engine(PolicyDecision::Ask, ScriptedExecutor::successful());
        let (request, token) = match engine.begin(REQUEST, 1_000).expect("begin should work") {
            BeginOutcome::ApprovalRequired { request, token } => (request, token),
            outcome => panic!("unexpected begin outcome: {outcome:?}"),
        };
        engine
            .cancel_approval(token, request.clone(), 1_001)
            .expect("cancellation should consume approval");
        assert!(engine.executor.calls.is_empty());
        assert!(matches!(
            engine.approve(token, request, 1_002),
            Err(EngineError::Approval(ApprovalError::Replay))
        ));
        assert!(matches!(
            engine.audit().records()[3].event,
            AuditEvent::ApprovalCancelled { .. }
        ));
    }

    #[test]
    fn cancellation_is_recorded_even_after_approval_expires() {
        let mut engine = engine(PolicyDecision::Ask, ScriptedExecutor::successful());
        let (request, token) = match engine.begin(REQUEST, 1_000).expect("begin should work") {
            BeginOutcome::ApprovalRequired { request, token } => (request, token),
            outcome => panic!("unexpected begin outcome: {outcome:?}"),
        };
        engine
            .cancel_approval(token, request, 1_101)
            .expect("cancellation should remain auditable after expiry");
        assert!(engine.executor.calls.is_empty());
        assert!(matches!(
            engine.audit().records().last().map(|record| &record.event),
            Some(AuditEvent::ApprovalCancelled { .. })
        ));
    }

    #[test]
    fn explicit_allow_executes_without_approval() {
        let mut engine = engine(PolicyDecision::Allow, ScriptedExecutor::successful());
        let outcome = engine.begin(REQUEST, 1_000).expect("begin should work");
        assert!(matches!(outcome, BeginOutcome::Completed(_)));
        assert_eq!(engine.executor.calls.len(), 1);
    }

    #[test]
    fn os_identity_allow_uses_native_provider_not_executor() {
        let policy = PolicyEngine::new(vec![PolicyRule {
            capability: Capability::SystemReadOsIdentity,
            decision: PolicyDecision::Allow,
        }]);
        let provider = ScriptedOsIdentity {
            result: Some(Ok(os_identity())),
            calls: 0,
        };
        let mut engine = BlossomEngine::with_os_identity(
            policy,
            ApprovalStore::new(100),
            ScriptedExecutor::successful(),
            provider,
        );
        let outcome = engine
            .begin(
                r#"{"request_id":"req-os","tool":"system.os.identity","arguments":{}}"#,
                1_000,
            )
            .expect("native read should complete");
        let completed = match outcome {
            BeginOutcome::Completed(completed) => completed,
            other => panic!("unexpected outcome: {other:?}"),
        };
        assert!(completed.verification.succeeded);
        assert!(matches!(completed.output, ToolOutput::OsIdentity(_)));
        assert!(engine.executor.calls.is_empty());
        assert_eq!(engine.os_identity.calls, 1);
        assert!(engine.audit().verify_chain());
        assert!(
            engine
                .audit()
                .records()
                .iter()
                .any(|record| matches!(record.event, AuditEvent::OsIdentityReadFinished { .. }))
        );
    }

    #[test]
    fn os_identity_failure_is_audited_without_executor_fallback() {
        let policy = PolicyEngine::new(vec![PolicyRule {
            capability: Capability::SystemReadOsIdentity,
            decision: PolicyDecision::Allow,
        }]);
        let provider = ScriptedOsIdentity {
            result: Some(Err(OsIdentityError::Missing)),
            calls: 0,
        };
        let mut engine = BlossomEngine::with_os_identity(
            policy,
            ApprovalStore::new(100),
            ScriptedExecutor::successful(),
            provider,
        );
        assert!(matches!(
            engine.begin(
                r#"{"request_id":"req-os","tool":"system.os.identity","arguments":{}}"#,
                1_000,
            ),
            Err(EngineError::OsIdentity(OsIdentityError::Missing))
        ));
        assert!(engine.executor.calls.is_empty());
        assert!(matches!(
            engine.audit().records().last().map(|record| &record.event),
            Some(AuditEvent::NativeReadFailed { .. })
        ));
    }

    #[test]
    fn uptime_allow_uses_native_provider_not_executor() {
        let policy = PolicyEngine::new(vec![PolicyRule {
            capability: Capability::SystemReadUptime,
            decision: PolicyDecision::Allow,
        }]);
        let provider = ScriptedUptime {
            result: Some(Ok(uptime())),
            calls: 0,
        };
        let mut engine = BlossomEngine::with_uptime(
            policy,
            ApprovalStore::new(100),
            ScriptedExecutor::successful(),
            provider,
        );
        let outcome = engine
            .begin(
                r#"{"request_id":"req-up","tool":"system.uptime","arguments":{}}"#,
                1_000,
            )
            .expect("native read should complete");
        let completed = match outcome {
            BeginOutcome::Completed(completed) => completed,
            other => panic!("unexpected outcome: {other:?}"),
        };
        assert!(completed.verification.succeeded);
        assert!(matches!(completed.output, ToolOutput::Uptime(_)));
        assert!(engine.executor.calls.is_empty());
        assert_eq!(engine.uptime.calls, 1);
        assert!(engine.audit().verify_chain());
        assert!(
            engine
                .audit()
                .records()
                .iter()
                .any(|record| matches!(record.event, AuditEvent::UptimeReadFinished { .. }))
        );
    }

    #[test]
    fn uptime_failure_is_audited_without_executor_fallback() {
        let policy = PolicyEngine::new(vec![PolicyRule {
            capability: Capability::SystemReadUptime,
            decision: PolicyDecision::Allow,
        }]);
        let provider = ScriptedUptime {
            result: Some(Err(UptimeError::Missing)),
            calls: 0,
        };
        let mut engine = BlossomEngine::with_uptime(
            policy,
            ApprovalStore::new(100),
            ScriptedExecutor::successful(),
            provider,
        );
        assert!(matches!(
            engine.begin(
                r#"{"request_id":"req-up","tool":"system.uptime","arguments":{}}"#,
                1_000,
            ),
            Err(EngineError::Uptime(UptimeError::Missing))
        ));
        assert!(engine.executor.calls.is_empty());
        assert!(matches!(
            engine.audit().records().last().map(|record| &record.event),
            Some(AuditEvent::UptimeReadFailed { .. })
        ));
    }

    #[test]
    fn memory_summary_allow_uses_native_provider_not_executor() {
        let policy = PolicyEngine::new(vec![PolicyRule {
            capability: Capability::SystemReadMemorySummary,
            decision: PolicyDecision::Allow,
        }]);
        let provider = ScriptedMemorySummary {
            result: Some(Ok(memory_summary())),
            calls: 0,
        };
        let mut engine = BlossomEngine::with_memory_summary(
            policy,
            ApprovalStore::new(100),
            ScriptedExecutor::successful(),
            provider,
        );
        let outcome = engine
            .begin(
                r#"{"request_id":"req-memory","tool":"system.memory.summary","arguments":{}}"#,
                1_000,
            )
            .expect("native read should complete");
        let completed = match outcome {
            BeginOutcome::Completed(completed) => completed,
            other => panic!("unexpected outcome: {other:?}"),
        };
        assert!(completed.verification.succeeded);
        assert!(matches!(completed.output, ToolOutput::MemorySummary(_)));
        assert!(engine.executor.calls.is_empty());
        assert_eq!(engine.memory_summary.calls, 1);
        assert!(engine.audit().verify_chain());
        assert!(
            engine
                .audit()
                .records()
                .iter()
                .any(|record| matches!(record.event, AuditEvent::MemorySummaryReadFinished { .. }))
        );
    }

    #[test]
    fn memory_summary_failure_is_audited_without_executor_fallback() {
        let policy = PolicyEngine::new(vec![PolicyRule {
            capability: Capability::SystemReadMemorySummary,
            decision: PolicyDecision::Allow,
        }]);
        let provider = ScriptedMemorySummary {
            result: Some(Err(MemorySummaryError::Missing)),
            calls: 0,
        };
        let mut engine = BlossomEngine::with_memory_summary(
            policy,
            ApprovalStore::new(100),
            ScriptedExecutor::successful(),
            provider,
        );
        assert!(matches!(
            engine.begin(
                r#"{"request_id":"req-memory","tool":"system.memory.summary","arguments":{}}"#,
                1_000,
            ),
            Err(EngineError::MemorySummary(MemorySummaryError::Missing))
        ));
        assert!(engine.executor.calls.is_empty());
        assert!(matches!(
            engine.audit().records().last().map(|record| &record.event),
            Some(AuditEvent::MemorySummaryReadFailed { .. })
        ));
    }

    #[test]
    fn storage_summary_allow_uses_native_provider_not_executor() {
        let policy = PolicyEngine::new(vec![PolicyRule {
            capability: Capability::SystemReadStorageSummary,
            decision: PolicyDecision::Allow,
        }]);
        let provider = ScriptedStorageSummary {
            result: Some(Ok(storage_summary())),
            calls: 0,
        };
        let mut engine = BlossomEngine::with_storage_summary(
            policy,
            ApprovalStore::new(100),
            ScriptedExecutor::successful(),
            provider,
        );
        let outcome = engine
            .begin(
                r#"{"request_id":"req-storage","tool":"system.storage.summary","arguments":{}}"#,
                1_000,
            )
            .expect("native read should complete");
        let completed = match outcome {
            BeginOutcome::Completed(completed) => completed,
            other => panic!("unexpected outcome: {other:?}"),
        };
        assert!(completed.verification.succeeded);
        assert!(matches!(completed.output, ToolOutput::StorageSummary(_)));
        assert!(engine.executor.calls.is_empty());
        assert_eq!(engine.storage_summary.calls, 1);
        assert!(engine.audit().verify_chain());
        assert!(
            engine.audit().records().iter().any(|record| matches!(
                record.event,
                AuditEvent::StorageSummaryReadFinished { .. }
            ))
        );
    }

    #[test]
    fn storage_summary_failure_is_audited_without_executor_fallback() {
        let policy = PolicyEngine::new(vec![PolicyRule {
            capability: Capability::SystemReadStorageSummary,
            decision: PolicyDecision::Allow,
        }]);
        let provider = ScriptedStorageSummary {
            result: Some(Err(StorageSummaryError::StatFailed)),
            calls: 0,
        };
        let mut engine = BlossomEngine::with_storage_summary(
            policy,
            ApprovalStore::new(100),
            ScriptedExecutor::successful(),
            provider,
        );
        assert!(matches!(
            engine.begin(
                r#"{"request_id":"req-storage","tool":"system.storage.summary","arguments":{}}"#,
                1_000,
            ),
            Err(EngineError::StorageSummary(StorageSummaryError::StatFailed))
        ));
        assert!(engine.executor.calls.is_empty());
        assert!(matches!(
            engine.audit().records().last().map(|record| &record.event),
            Some(AuditEvent::StorageSummaryReadFailed { .. })
        ));
    }

    #[test]
    fn process_self_allow_uses_native_provider_not_executor() {
        let policy = PolicyEngine::new(vec![PolicyRule {
            capability: Capability::ProcessReadSelf,
            decision: PolicyDecision::Allow,
        }]);
        let provider = ScriptedProcessSelf {
            result: Some(Ok(process_self())),
            calls: 0,
        };
        let mut engine = BlossomEngine::with_process_self(
            policy,
            ApprovalStore::new(100),
            ScriptedExecutor::successful(),
            provider,
        );
        let outcome = engine
            .begin(
                r#"{"request_id":"req-self","tool":"process.self","arguments":{}}"#,
                1_000,
            )
            .expect("native read should complete");
        let completed = match outcome {
            BeginOutcome::Completed(completed) => completed,
            other => panic!("unexpected outcome: {other:?}"),
        };
        assert!(completed.verification.succeeded);
        assert!(matches!(completed.output, ToolOutput::ProcessSelf(_)));
        assert!(engine.executor.calls.is_empty());
        assert_eq!(engine.process_self.calls, 1);
        assert!(engine.audit().verify_chain());
        assert!(
            engine
                .audit()
                .records()
                .iter()
                .any(|record| matches!(record.event, AuditEvent::ProcessSelfReadFinished { .. }))
        );
    }

    #[test]
    fn process_self_failure_is_audited_without_executor_fallback() {
        let policy = PolicyEngine::new(vec![PolicyRule {
            capability: Capability::ProcessReadSelf,
            decision: PolicyDecision::Allow,
        }]);
        let provider = ScriptedProcessSelf {
            result: Some(Err(ProcessSelfError::InvalidProcessId)),
            calls: 0,
        };
        let mut engine = BlossomEngine::with_process_self(
            policy,
            ApprovalStore::new(100),
            ScriptedExecutor::successful(),
            provider,
        );
        assert!(matches!(
            engine.begin(
                r#"{"request_id":"req-self","tool":"process.self","arguments":{}}"#,
                1_000,
            ),
            Err(EngineError::ProcessSelf(ProcessSelfError::InvalidProcessId))
        ));
        assert!(engine.executor.calls.is_empty());
        assert!(matches!(
            engine.audit().records().last().map(|record| &record.event),
            Some(AuditEvent::ProcessSelfReadFailed { .. })
        ));
    }

    #[test]
    fn records_executor_failure() {
        let executor = ScriptedExecutor {
            outcomes: VecDeque::from([Err(ExecutorError::Timeout)]),
            calls: Vec::new(),
        };
        let mut engine = engine(PolicyDecision::Allow, executor);
        assert!(matches!(
            engine.begin(REQUEST, 1_000),
            Err(EngineError::Executor(ExecutorError::Timeout))
        ));
        assert!(engine.audit().verify_chain());
    }

    #[test]
    fn rejects_malformed_request_before_execution() {
        let mut engine = engine(PolicyDecision::Allow, ScriptedExecutor::successful());
        assert!(matches!(
            engine.begin("not-json", 1_000),
            Err(EngineError::InvalidRequest(_))
        ));
        assert!(engine.executor.calls.is_empty());
        assert_eq!(engine.audit().records().len(), 1);
        assert!(matches!(
            engine.audit().records()[0].event,
            AuditEvent::RequestRejected { .. }
        ));
    }
}
