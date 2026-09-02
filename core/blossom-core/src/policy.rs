use crate::request::ToolRequest;
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
pub enum Capability {
    SystemReadKernelIdentity,
    SystemReadOsIdentity,
    SystemReadUptime,
}

impl Capability {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SystemReadKernelIdentity => "system.read:kernel.identity",
            Self::SystemReadOsIdentity => "system.read:os.identity",
            Self::SystemReadUptime => "system.read:uptime",
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
    use crate::request::{RequestId, ToolRequest};

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
}
