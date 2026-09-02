#![forbid(unsafe_code)]

pub mod approval;
pub mod audit;
pub mod engine;
pub mod executor;
pub mod file_read;
pub mod memory_summary;
pub mod os_identity;
pub mod policy;
pub mod process_list;
pub mod process_self;
pub mod request;
pub mod storage_summary;
pub mod uptime;
pub mod verification;

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
    verify_process_list, verify_process_self, verify_storage_summary, verify_uptime,
};
