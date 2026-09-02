use crate::request::ToolRequest;
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
pub enum Capability {
    SystemReadKernelIdentity,
    SystemReadOsIdentity,
    SystemReadUptime,
    SystemReadMemorySummary,
    SystemReadStorageSummary,
    ProcessReadSelf,
    ProcessReadList,
    FilesReadContent,
    FilesWriteCreate,
    ServicesReadStatus,
}

impl Capability {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SystemReadKernelIdentity => "system.read:kernel.identity",
            Self::SystemReadOsIdentity => "system.read:os.identity",
            Self::SystemReadUptime => "system.read:uptime",
            Self::SystemReadMemorySummary => "system.read:memory.summary",
            Self::SystemReadStorageSummary => "system.read:storage.summary",
            Self::ProcessReadSelf => "process.read:self",
            Self::ProcessReadList => "process.read:list",
            Self::FilesReadContent => "files.read:content",
            Self::FilesWriteCreate => "files.write:create",
            Self::ServicesReadStatus => "services.read:status",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum PolicyDecision {
    Allow,
    Deny,
    Ask,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PolicyRule {
    pub capability: Capability,
    pub decision: PolicyDecision,
}

#[derive(Clone, Debug, Default)]
pub struct PolicyEngine {
    rules: Vec<PolicyRule>,
}

impl PolicyEngine {
    pub fn new(rules: Vec<PolicyRule>) -> Self {
        Self { rules }
    }

    pub fn required_capability(request: &ToolRequest) -> Capability {
        match request {
            ToolRequest::SystemUname { .. } => Capability::SystemReadKernelIdentity,
            ToolRequest::SystemOsIdentity { .. } => Capability::SystemReadOsIdentity,
            ToolRequest::SystemUptime { .. } => Capability::SystemReadUptime,
            ToolRequest::SystemMemorySummary { .. } => Capability::SystemReadMemorySummary,
            ToolRequest::SystemStorageSummary { .. } => Capability::SystemReadStorageSummary,
            ToolRequest::ProcessSelf { .. } => Capability::ProcessReadSelf,
            ToolRequest::ProcessList { .. } => Capability::ProcessReadList,
            ToolRequest::FilesReadContent { .. } => Capability::FilesReadContent,
            ToolRequest::FilesWriteCreate { .. } => Capability::FilesWriteCreate,
            ToolRequest::ServicesReadStatus { .. } => Capability::ServicesReadStatus,
        }
    }

    pub fn evaluate(&self, request: &ToolRequest) -> PolicyDecision {
        let capability = Self::required_capability(request);
        self.rules
            .iter()
            .rev()
            .find(|rule| rule.capability == capability)
            .map_or(PolicyDecision::Deny, |rule| rule.decision)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        file_read::{FileIdentity, FileSelection},
        request::{RequestId, ToolRequest},
        service_status::ServiceSelection,
        workspace_create::{
            DirectoryIdentity, WORKSPACE_FILE_MODE, WorkspaceCreateSelection, digest,
        },
    };

    fn request() -> ToolRequest {
        ToolRequest::SystemUname {
            request_id: RequestId::parse("req-1".into()).expect("valid test id"),
        }
    }

    #[test]
    fn denies_by_default() {
        assert_eq!(
            PolicyEngine::default().evaluate(&request()),
            PolicyDecision::Deny
        );
        let os_identity = ToolRequest::SystemOsIdentity {
            request_id: RequestId::parse("req-os".into()).expect("valid test id"),
        };
        assert_eq!(
            PolicyEngine::default().evaluate(&os_identity),
            PolicyDecision::Deny
        );
        let uptime = ToolRequest::SystemUptime {
            request_id: RequestId::parse("req-uptime".into()).expect("valid test id"),
        };
        assert_eq!(
            PolicyEngine::default().evaluate(&uptime),
            PolicyDecision::Deny
        );
        let memory = ToolRequest::SystemMemorySummary {
            request_id: RequestId::parse("req-memory".into()).expect("valid test id"),
        };
        assert_eq!(
            PolicyEngine::default().evaluate(&memory),
            PolicyDecision::Deny
        );
        let storage = ToolRequest::SystemStorageSummary {
            request_id: RequestId::parse("req-storage".into()).expect("valid test id"),
        };
        assert_eq!(
            PolicyEngine::default().evaluate(&storage),
            PolicyDecision::Deny
        );
        let process_self = ToolRequest::ProcessSelf {
            request_id: RequestId::parse("req-process-self".into()).expect("valid test id"),
        };
        assert_eq!(
            PolicyEngine::default().evaluate(&process_self),
            PolicyDecision::Deny
        );
    }

    #[test]
    fn returns_explicit_rule() {
        let policy = PolicyEngine::new(vec![PolicyRule {
            capability: Capability::SystemReadKernelIdentity,
            decision: PolicyDecision::Ask,
        }]);
        assert_eq!(policy.evaluate(&request()), PolicyDecision::Ask);
        assert_eq!(
            PolicyEngine::required_capability(&request()).as_str(),
            "system.read:kernel.identity"
        );
    }

    #[test]
    fn every_registered_request_has_the_exact_static_capability() {
        let id = || RequestId::parse("req-registry".into()).expect("valid test id");
        let requests = vec![
            (
                ToolRequest::SystemUname { request_id: id() },
                Capability::SystemReadKernelIdentity,
                "system.read:kernel.identity",
            ),
            (
                ToolRequest::SystemOsIdentity { request_id: id() },
                Capability::SystemReadOsIdentity,
                "system.read:os.identity",
            ),
            (
                ToolRequest::SystemUptime { request_id: id() },
                Capability::SystemReadUptime,
                "system.read:uptime",
            ),
            (
                ToolRequest::SystemMemorySummary { request_id: id() },
                Capability::SystemReadMemorySummary,
                "system.read:memory.summary",
            ),
            (
                ToolRequest::SystemStorageSummary { request_id: id() },
                Capability::SystemReadStorageSummary,
                "system.read:storage.summary",
            ),
            (
                ToolRequest::ProcessSelf { request_id: id() },
                Capability::ProcessReadSelf,
                "process.read:self",
            ),
            (
                ToolRequest::ProcessList { request_id: id() },
                Capability::ProcessReadList,
                "process.read:list",
            ),
            (
                ToolRequest::FilesReadContent {
                    request_id: id(),
                    selection: FileSelection {
                        absolute_path: "/tmp/file".into(),
                        identity: FileIdentity {
                            device: 1,
                            inode: 2,
                            size: 3,
                            modified_seconds: 4,
                            modified_nanoseconds: 5,
                            changed_seconds: 6,
                            changed_nanoseconds: 7,
                        },
                    },
                },
                Capability::FilesReadContent,
                "files.read:content",
            ),
            (
                ToolRequest::FilesWriteCreate {
                    request_id: id(),
                    selection: WorkspaceCreateSelection {
                        workspace_root: "/tmp/workspace".into(),
                        root_identity: DirectoryIdentity {
                            device: 1,
                            inode: 2,
                        },
                        parent_identity: DirectoryIdentity {
                            device: 1,
                            inode: 3,
                        },
                        relative_destination: "new.txt".into(),
                        content: "content".into(),
                        content_sha256: digest(b"content"),
                        mode: WORKSPACE_FILE_MODE,
                    },
                },
                Capability::FilesWriteCreate,
                "files.write:create",
            ),
            (
                ToolRequest::ServicesReadStatus {
                    request_id: id(),
                    selection: ServiceSelection {
                        unit: "dbus.service".into(),
                    },
                },
                Capability::ServicesReadStatus,
                "services.read:status",
            ),
        ];

        for (request, capability, name) in requests {
            assert_eq!(PolicyEngine::required_capability(&request), capability);
            assert_eq!(capability.as_str(), name);
            assert_eq!(
                PolicyEngine::default().evaluate(&request),
                PolicyDecision::Deny
            );
        }
    }
}
