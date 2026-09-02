use crate::request::ToolRequest;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
pub struct ApprovalToken(u64);

impl ApprovalToken {
    pub fn value(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Debug)]
struct ApprovalRecord {
    request: ToolRequest,
    expires_at_ms: u64,
}

#[derive(Clone, Debug)]
pub struct ApprovalStore {
    next_token: u64,
    ttl_ms: u64,
    pending: HashMap<ApprovalToken, ApprovalRecord>,
    consumed: HashSet<ApprovalToken>,
}

impl ApprovalStore {
    pub fn new(ttl_ms: u64) -> Self {
        Self {
            next_token: 1,
            ttl_ms,
            pending: HashMap::new(),
            consumed: HashSet::new(),
        }
    }

    pub fn issue(&mut self, request: ToolRequest, now_ms: u64) -> ApprovalToken {
        let token = ApprovalToken(self.next_token);
        self.next_token = self.next_token.checked_add(1).unwrap_or(1);
        self.pending.insert(
            token,
            ApprovalRecord {
                request,
                expires_at_ms: now_ms.saturating_add(self.ttl_ms),
            },
        );
        token
    }

    pub fn consume(
        &mut self,
        token: ApprovalToken,
        request: &ToolRequest,
        now_ms: u64,
    ) -> Result<(), ApprovalError> {
        if self.consumed.contains(&token) {
            return Err(ApprovalError::Replay);
        }
        let record = self.pending.get(&token).ok_or(ApprovalError::Unknown)?;
        if now_ms > record.expires_at_ms {
            self.pending.remove(&token);
            return Err(ApprovalError::Expired);
        }
        if record.request != *request {
            return Err(ApprovalError::BindingMismatch);
        }
        self.pending.remove(&token);
        self.consumed.insert(token);
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum ApprovalError {
    Unknown,
    Expired,
    Replay,
    BindingMismatch,
}

impl fmt::Display for ApprovalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Unknown => "unknown approval token",
            Self::Expired => "approval token expired",
            Self::Replay => "approval token was already consumed",
            Self::BindingMismatch => "approval token does not match the request",
        })
    }
}

impl std::error::Error for ApprovalError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::{RequestId, ToolRequest};

    fn request(id: &str) -> ToolRequest {
        ToolRequest::SystemUname {
            request_id: RequestId::parse(id.into()).expect("valid test id"),
        }
    }

    #[test]
    fn binds_approval_and_rejects_replay() {
        let mut store = ApprovalStore::new(100);
        let original = request("req-1");
        let token = store.issue(original.clone(), 1_000);
        assert_eq!(
            store.consume(token, &request("req-2"), 1_001),
            Err(ApprovalError::BindingMismatch)
        );
        assert_eq!(store.consume(token, &original, 1_001), Ok(()));
        assert_eq!(
            store.consume(token, &original, 1_002),
            Err(ApprovalError::Replay)
        );
    }

    #[test]
    fn rejects_expired_approval() {
        let mut store = ApprovalStore::new(100);
        let request = request("req-1");
        let token = store.issue(request.clone(), 1_000);
        assert_eq!(
            store.consume(token, &request, 1_101),
            Err(ApprovalError::Expired)
        );
    }
}
