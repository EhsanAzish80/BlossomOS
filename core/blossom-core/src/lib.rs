#![forbid(unsafe_code)]

pub mod approval;
pub mod audit;
pub mod engine;
pub mod executor;
pub mod memory_summary;
pub mod os_identity;
pub mod policy;
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
    Verification, verify_execution, verify_memory_summary, verify_os_identity,
    verify_storage_summary, verify_uptime,
};
