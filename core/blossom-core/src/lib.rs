#![forbid(unsafe_code)]

pub mod approval;
pub mod audit;
pub mod battery_summary;
pub mod context;
pub mod engine;
pub mod executor;
pub mod file_read;
pub mod memory_summary;
pub mod model_runtime;
pub mod network_connectivity;
pub mod orchestration;
pub mod os_identity;
pub mod policy;
pub mod privileged;
pub mod process_list;
pub mod process_self;
pub mod request;
pub mod service_status;
pub mod shell_activity;
pub mod shell_ipc;
pub mod shell_service;
pub mod shell_session;
pub mod storage_summary;
pub mod uptime;
pub mod verification;
pub mod workspace_create;

pub use approval::{ApprovalError, ApprovalStore, ApprovalToken};
pub use audit::{AuditEvent, AuditLog, AuditRecord, BatteryAuditStatus};
pub use battery_summary::{
    BATTERY_READ_TIMEOUT_MS, BatteryObservation, BatteryObservationError, BatteryReadError,
    BatteryState, BatterySummary, BatterySummaryProvider, DBUS_PROPERTIES_INTERFACE,
    SYSTEM_BUS_ADDRESS as BATTERY_SYSTEM_BUS_ADDRESS, UPOWER_DESTINATION, UPOWER_DEVICE_INTERFACE,
    UPOWER_DISPLAY_DEVICE_PATH, UnavailableBatterySummaryProvider, UpowerBatterySummaryProvider,
    validate_battery_observation,
};
pub use context::{
    BATTERY_MAX_AGE_MS, BATTERY_MIN_POLL_INTERVAL_MS, CONTEXT_PROTOCOL_VERSION, ContextObservation,
    ContextSource, ContextValue, MAX_CONTEXT_RESPONSE_BYTES, NETWORK_CONNECTIVITY_MAX_AGE_MS,
    NETWORK_CONNECTIVITY_MIN_POLL_INTERVAL_MS,
};
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
#[cfg(all(unix, debug_assertions))]
pub use model_runtime::serve_synthetic_gateway_via_adapter_once;
pub use model_runtime::{
    AuthorizedGatewayClient, ConversationMessage, ConversationRole, GATEWAY_PROTOCOL_VERSION,
    GatewayEventValidator, GatewayFrame, GatewayFrameDecoder, GatewayMessageKind,
    GatewayPeerCredentials, GatewayProfile, GatewayProtocolError, InferenceAuditOutcome,
    InferenceAuditProjection, InferenceCancellation, InferenceOutputMode, InferenceRequest,
    InferenceRequestId, LLAMA_CPP_ENDPOINT, LlamaCppAdapter, LlamaCppAdapterError,
    MAX_GATEWAY_FRAME_BYTES, MAX_PROVIDER_MANIFEST_BYTES, MODEL_PROTOCOL_VERSION,
    ModelContractError, ModelIntentDefinition, ModelIntentKind, ModelProfile, ModelProviderKind,
    ModelStreamState, NormalizedCompletion, NormalizedStreamEvent, NormalizedStreamKind,
    OLLAMA_ENDPOINT, OllamaAdapter, OllamaAdapterError, ProposedToolIntent, ProviderArtifact,
    ProviderFailureCategory, ProviderFilesystemPolicy, ProviderProfileError,
    ProviderProfileManifest, ProviderProfileResources, ProviderProfileSpec,
    ProviderServiceIdentity, ProviderStreamInput, TurnIntentCatalogue, ValidatedProviderProfile,
    decode_gateway_cancel, decode_gateway_event, decode_gateway_hello,
    decode_gateway_private_request, decode_gateway_synthetic_request, encode_gateway_cancel,
    encode_gateway_event, encode_gateway_hello, encode_gateway_private_request,
    encode_gateway_synthetic_request, load_installed_provider_profile,
    load_installed_provider_profile_from_set, load_installed_runtime_readiness,
    load_installed_runtime_readiness_from_set, production_provider_profile, validate_gateway_peer,
    validate_provider_completion,
};
#[cfg(unix)]
pub use model_runtime::{
    GatewayFixtureError, SyntheticGatewayClient, fixed_synthetic_gateway_request,
    serve_synthetic_gateway_once,
};
#[cfg(debug_assertions)]
pub use model_runtime::{SyntheticProviderPackage, fixed_synthetic_provider_package};
pub use network_connectivity::{
    DBUS_PROPERTIES_INTERFACE as NETWORK_DBUS_PROPERTIES_INTERFACE,
    NETWORK_CONNECTIVITY_READ_TIMEOUT_MS, NETWORK_MANAGER_DESTINATION, NETWORK_MANAGER_INTERFACE,
    NETWORK_MANAGER_PATH, NetworkConnectivity, NetworkConnectivityObservation,
    NetworkConnectivityObservationError, NetworkConnectivityProvider, NetworkConnectivityReadError,
    NetworkManagerConnectivityProvider, SYSTEM_BUS_ADDRESS as NETWORK_SYSTEM_BUS_ADDRESS,
    UnavailableNetworkConnectivityProvider, validate_network_connectivity_observation,
};
pub use orchestration::{
    MAX_PLAN_STEPS, OrchestrationError, OrchestrationEvent, PlanError, PlanId, PlanOrchestrator,
    PlanOutcome, ProposedPlanStep, RecoveryDisposition, RetryDisposition, RollbackDisposition,
    StateError, StepId, StepLifecycle, StepPhase, StepTerminalOutcome, SummaryError,
    TruthfulPlanReport, TruthfulPlanSummary, TruthfulStepReport, TypedRequestEngine, ValidatedPlan,
    ValidatedPlanStep,
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
pub use shell_activity::{ShellActivityError, project_shell_activity};
pub use shell_ipc::{
    MAX_ACTIVITY_BATCH, MAX_SHELL_MESSAGE_BYTES, SHELL_BUS_NAME, SHELL_INTERFACE,
    SHELL_OBJECT_PATH, SHELL_PROTOCOL_VERSION, ShellActivityCategory, ShellActivityKind,
    ShellActivityProjection, ShellApprovalPreview, ShellBatteryProjection, ShellBatteryStatus,
    ShellClientRequest, ShellDecision, ShellProtocolError, decode_shell_client_request,
};
pub use shell_service::{
    SHELL_APPROVAL_TTL_MS, ShellDiagnosticService, ShellServiceError, ShellServiceOutcome,
};
pub use shell_session::{
    MAX_SHELL_PEER_NAME_BYTES, ShellCancellationReason, ShellCancelledApproval, ShellPeerId,
    ShellResolvedApproval, ShellSessionApprovals, ShellSessionError,
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
    Verification, verify_battery_summary, verify_execution, verify_file_content,
    verify_memory_summary, verify_network_connectivity, verify_os_identity, verify_process_list,
    verify_process_self, verify_service_status, verify_storage_summary, verify_uptime,
    verify_workspace_file_created,
};
pub use workspace_create::{
    AtomicWorkspaceFileCreator, DirectoryIdentity, UnavailableWorkspaceCreateProvider,
    WORKSPACE_FILE_MODE, WorkspaceCreateError, WorkspaceCreateProvider, WorkspaceCreateSelection,
    WorkspaceCreateState, WorkspaceFileCreated, validate_relative_destination,
    validate_workspace_selection,
};
