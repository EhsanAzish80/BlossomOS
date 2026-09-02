#![forbid(unsafe_code)]

pub mod approval;
pub mod audit;
pub mod engine;
pub mod executor;
pub mod os_identity;
pub mod policy;
pub mod request;
pub mod verification;

pub use approval::{ApprovalError, ApprovalStore, ApprovalToken};
pub use audit::{AuditEvent, AuditLog, AuditRecord};
pub use engine::{
    BeginOutcome, BlossomEngine, CompletionOutcome, EngineError, ToolOutput, command_for,
};
pub use executor::{CommandSpec, ExecutionResult, Executor, ExecutorError};
pub use os_identity::{
    OsIdentity, OsIdentityError, OsIdentityProvider, OsReleaseReader, OsReleaseSource,
    UnavailableOsIdentityProvider, parse_os_release,
};
pub use policy::{Capability, PolicyDecision, PolicyEngine, PolicyRule};
pub use request::{RequestError, RequestId, ToolRequest};
pub use verification::{Verification, verify_execution, verify_os_identity};
