#![forbid(unsafe_code)]

pub mod approval;
pub mod audit;
pub mod engine;
pub mod executor;
pub mod file_read;
pub mod memory_summary;
pub mod model_runtime;
pub mod os_identity;
pub mod policy;
pub mod privileged;
pub mod process_list;
pub mod process_self;
pub mod request;
pub mod service_status;
pub mod storage_summary;
pub mod uptime;
pub mod verification;
pub mod workspace_create;

pub use approval::{ApprovalError, ApprovalStore, ApprovalToken};
pub use audit::{AuditEvent, AuditLog, AuditRecord};
pub use engine::{
    BeginOutcome, BlossomEngine, CompletionOutcome, EngineError, ToolOutput, command_for,
};
pub use executor::{CommandSpec, ExecutionResult, Executor, ExecutorError};
pub use file_read::{
    FileContent, FileContentProvider, FileIdentity, FileReadError, FileSelection,
    MAX_FILE_CONTENT_BYTES, MAX_SELECTED_PATH_BYTES, Openat2FileReader,
    UnavailableFileContentProvider, validate_selected_path,
};
pub use memory_summary::{
    MAX_PROC_MEMINFO_BYTES, MAX_PROC_MEMINFO_LINES, MemorySummary, MemorySummaryError,
    MemorySummaryProvider, PROC_MEMINFO_PATH, ProcMeminfoReader, UnavailableMemorySummaryProvider,
    parse_proc_meminfo,
};
pub use model_runtime::{
    ConversationMessage, ConversationRole, InferenceAuditOutcome, InferenceAuditProjection,
    InferenceCancellation, InferenceOutputMode, InferenceRequest, InferenceRequestId,
    LLAMA_CPP_ENDPOINT, LlamaCppAdapter, LlamaCppAdapterError, MODEL_PROTOCOL_VERSION,
    ModelContractError, ModelIntentDefinition, ModelIntentKind, ModelProfile, ModelProviderKind,
    ModelStreamState, NormalizedCompletion, NormalizedStreamEvent, NormalizedStreamKind,
    OLLAMA_ENDPOINT, OllamaAdapter, OllamaAdapterError, ProposedToolIntent,
    ProviderFailureCategory, ProviderStreamInput, TurnIntentCatalogue,
    validate_provider_completion,
};
pub use os_identity::{
    OsIdentity, OsIdentityError, OsIdentityProvider, OsReleaseReader, OsReleaseSource,
    UnavailableOsIdentityProvider, parse_os_release,
};
pub use policy::{Capability, PolicyDecision, PolicyEngine, PolicyRule};
pub use process_list::{
    MAX_PROCESS_DIRECTORY_ENTRIES, MAX_PROCESS_NAME_BYTES, MAX_PROCESS_RESULTS,
    MAX_PROCESS_STATUS_BYTES, MAX_PROCESS_STATUS_LINES, PROC_ROOT, ProcProcessListReader,
    ProcessList, ProcessListEntry, ProcessListError, ProcessListProvider, ProcessListSource,
    ProcessState, UnavailableProcessListProvider, parse_process_status,
};
pub use process_self::{
    NativeProcessSelfReader, ProcessSelf, ProcessSelfError, ProcessSelfProvider, ProcessSelfSource,
    UnavailableProcessSelfProvider,
};
pub use request::{RequestError, RequestId, ToolRequest};
pub use service_status::{
    MAX_SERVICE_STATE_BYTES, MAX_SERVICE_UNIT_BYTES, SYSTEM_BUS_ADDRESS, SYSTEMD_DESTINATION,
    SYSTEMD_MANAGER_INTERFACE, SYSTEMD_MANAGER_PATH, SYSTEMD_UNIT_INTERFACE, ServiceSelection,
    ServiceStatus, ServiceStatusError, ServiceStatusProvider, SystemdServiceStatusProvider,
    UnavailableServiceStatusProvider, validate_service_status, validate_service_unit,
};
pub use storage_summary::{
    ROOT_FILESYSTEM_PATH, RootStorageReader, StorageSummary, StorageSummaryError,
    StorageSummaryProvider, StorageSummarySource, UnavailableStorageSummaryProvider,
};
pub use uptime::{
    MAX_PROC_UPTIME_BYTES, PROC_UPTIME_PATH, ProcUptimeReader, SystemUptime,
    UnavailableUptimeProvider, UptimeError, UptimeProvider, parse_proc_uptime,
};
pub use verification::{
    Verification, verify_execution, verify_file_content, verify_memory_summary, verify_os_identity,
    verify_process_list, verify_process_self, verify_service_status, verify_storage_summary,
    verify_uptime, verify_workspace_file_created,
};
pub use workspace_create::{
    AtomicWorkspaceFileCreator, DirectoryIdentity, UnavailableWorkspaceCreateProvider,
    WORKSPACE_FILE_MODE, WorkspaceCreateError, WorkspaceCreateProvider, WorkspaceCreateSelection,
    WorkspaceCreateState, WorkspaceFileCreated, validate_relative_destination,
    validate_workspace_selection,
};
