use crate::{RequestId, ShellApprovalPreview, ShellDecision};
use std::collections::HashMap;
use std::fmt;

pub const MAX_SHELL_PEER_NAME_BYTES: usize = 255;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ShellPeerId(String);

impl ShellPeerId {
    /// Accepts only a D-Bus unique connection name, never a caller-selected
    /// well-known name. The transport must obtain this value from message
    /// metadata rather than an IPC payload.
    pub fn from_bus_unique_name(value: &str) -> Result<Self, ShellSessionError> {
        let valid = value.starts_with(':')
            && value.len() >= 4
            && value.len() <= MAX_SHELL_PEER_NAME_BYTES
            && value[1..].split('.').count() >= 2
            && value[1..]
                .split('.')
                .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()));
        if valid {
            Ok(Self(value.into()))
        } else {
            Err(ShellSessionError::InvalidPeerIdentity)
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

pub struct ShellSessionApprovals<S> {
    pending: HashMap<ShellPeerId, PendingApproval<S>>,
}

impl<S> Default for ShellSessionApprovals<S> {
    fn default() -> Self {
        Self {
            pending: HashMap::new(),
        }
    }
}

struct PendingApproval<S> {
    request_id: RequestId,
    preview_sha256: String,
    expires_at_ms: u64,
    secret: S,
}

impl<S> ShellSessionApprovals<S> {
    pub fn register_system_uname(
        &mut self,
        peer: ShellPeerId,
        request_id: RequestId,
        expires_at_ms: u64,
        secret: S,
    ) -> Result<ShellApprovalPreview, ShellSessionError> {
        if self.pending.contains_key(&peer) {
            return Err(ShellSessionError::ApprovalAlreadyPending);
        }
        let preview = ShellApprovalPreview::system_uname(&request_id, expires_at_ms);
        self.pending.insert(
            peer,
            PendingApproval {
                request_id,
                preview_sha256: preview.preview_sha256.clone(),
                expires_at_ms,
                secret,
            },
        );
        Ok(preview)
    }

    pub fn resolve(
        &mut self,
        peer: &ShellPeerId,
        request_id: &RequestId,
        preview_sha256: &str,
        decision: ShellDecision,
        now_ms: u64,
    ) -> Result<ShellResolvedApproval<S>, ShellSessionError> {
        let pending = self
            .pending
            .get(peer)
            .ok_or(ShellSessionError::NoPendingApproval)?;
        if now_ms > pending.expires_at_ms {
            return Err(ShellSessionError::ApprovalExpired);
        }
        if &pending.request_id != request_id || pending.preview_sha256 != preview_sha256 {
            return Err(ShellSessionError::BindingMismatch);
        }
        let pending = self
            .pending
            .remove(peer)
            .expect("pending approval was checked immediately before removal");
        Ok(ShellResolvedApproval {
            request_id: pending.request_id,
            decision,
            secret: pending.secret,
        })
    }

    pub fn cancel(
        &mut self,
        peer: &ShellPeerId,
        request_id: &RequestId,
        preview_sha256: &str,
        now_ms: u64,
    ) -> Result<ShellCancelledApproval<S>, ShellSessionError> {
        let pending = self
            .pending
            .get(peer)
            .ok_or(ShellSessionError::NoPendingApproval)?;
        if now_ms > pending.expires_at_ms {
            return Err(ShellSessionError::ApprovalExpired);
        }
        if &pending.request_id != request_id || pending.preview_sha256 != preview_sha256 {
            return Err(ShellSessionError::BindingMismatch);
        }
        let pending = self
            .pending
            .remove(peer)
            .expect("pending approval was checked immediately before removal");
        Ok(ShellCancelledApproval {
            request_id: pending.request_id,
            secret: pending.secret,
            reason: ShellCancellationReason::UserCancelled,
        })
    }

    pub fn disconnect(&mut self, peer: &ShellPeerId) -> Option<ShellCancelledApproval<S>> {
        self.pending
            .remove(peer)
            .map(|pending| ShellCancelledApproval {
                request_id: pending.request_id,
                secret: pending.secret,
                reason: ShellCancellationReason::PeerDisconnected,
            })
    }

    pub fn expire(&mut self, peer: &ShellPeerId, now_ms: u64) -> Option<ShellCancelledApproval<S>> {
        let expired = self
            .pending
            .get(peer)
            .is_some_and(|pending| now_ms > pending.expires_at_ms);
        if !expired {
            return None;
        }
        self.pending
            .remove(peer)
            .map(|pending| ShellCancelledApproval {
                request_id: pending.request_id,
                secret: pending.secret,
                reason: ShellCancellationReason::Expired,
            })
    }

    pub fn has_pending(&self, peer: &ShellPeerId) -> bool {
        self.pending.contains_key(peer)
    }
}

pub struct ShellResolvedApproval<S> {
    pub request_id: RequestId,
    pub decision: ShellDecision,
    secret: S,
}

impl<S> ShellResolvedApproval<S> {
    pub fn into_secret(self) -> S {
        self.secret
    }
}

pub struct ShellCancelledApproval<S> {
    pub request_id: RequestId,
    pub reason: ShellCancellationReason,
    secret: S,
}

impl<S> ShellCancelledApproval<S> {
    pub fn into_secret(self) -> S {
        self.secret
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShellCancellationReason {
    UserCancelled,
    PeerDisconnected,
    Expired,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShellSessionError {
    InvalidPeerIdentity,
    ApprovalAlreadyPending,
    NoPendingApproval,
    ApprovalExpired,
    BindingMismatch,
}

impl fmt::Display for ShellSessionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidPeerIdentity => "session peer identity is invalid",
            Self::ApprovalAlreadyPending => "an approval is already pending for this peer",
            Self::NoPendingApproval => "no approval is pending for this peer",
            Self::ApprovalExpired => "the pending approval expired",
            Self::BindingMismatch => "the decision does not match the pending approval",
        })
    }
}

impl std::error::Error for ShellSessionError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn peer(value: &str) -> ShellPeerId {
        ShellPeerId::from_bus_unique_name(value).expect("valid unique bus name")
    }

    fn request(value: &str) -> RequestId {
        RequestId::parse(value.into()).expect("valid request id")
    }

    #[test]
    fn accepts_only_unique_numeric_bus_names() {
        assert_eq!(peer(":1.42").as_str(), ":1.42");
        for invalid in ["org.blossomos.Shell1", ":1", ":x.1", ":1.", "1.2"] {
            assert_eq!(
                ShellPeerId::from_bus_unique_name(invalid),
                Err(ShellSessionError::InvalidPeerIdentity)
            );
        }
    }

    #[test]
    fn one_peer_cannot_resolve_another_peers_challenge() {
        let mut sessions = ShellSessionApprovals::default();
        let owner = peer(":1.10");
        let attacker = peer(":1.11");
        let id = request("req-1");
        let preview = sessions
            .register_system_uname(owner.clone(), id.clone(), 1_100, 73_u64)
            .expect("register");
        assert!(matches!(
            sessions.resolve(
                &attacker,
                &id,
                &preview.preview_sha256,
                ShellDecision::ApproveOnce,
                1_001
            ),
            Err(ShellSessionError::NoPendingApproval)
        ));
        assert!(sessions.has_pending(&owner));
    }

    #[test]
    fn mutation_does_not_consume_the_real_challenge() {
        let mut sessions = ShellSessionApprovals::default();
        let owner = peer(":1.10");
        let id = request("req-1");
        let preview = sessions
            .register_system_uname(owner.clone(), id.clone(), 1_100, 73_u64)
            .expect("register");
        assert!(matches!(
            sessions.resolve(
                &owner,
                &request("req-2"),
                &preview.preview_sha256,
                ShellDecision::ApproveOnce,
                1_001
            ),
            Err(ShellSessionError::BindingMismatch)
        ));
        assert!(sessions.has_pending(&owner));
    }

    #[test]
    fn resolution_is_once_only_and_returns_the_private_secret() {
        let mut sessions = ShellSessionApprovals::default();
        let owner = peer(":1.10");
        let id = request("req-1");
        let preview = sessions
            .register_system_uname(owner.clone(), id.clone(), 1_100, 73_u64)
            .expect("register");
        let resolved = sessions
            .resolve(
                &owner,
                &id,
                &preview.preview_sha256,
                ShellDecision::ApproveOnce,
                1_001,
            )
            .expect("resolve");
        assert_eq!(resolved.into_secret(), 73);
        assert!(matches!(
            sessions.resolve(
                &owner,
                &id,
                &preview.preview_sha256,
                ShellDecision::ApproveOnce,
                1_002
            ),
            Err(ShellSessionError::NoPendingApproval)
        ));
    }

    #[test]
    fn replacement_expiry_cancel_and_disconnect_fail_closed() {
        let mut sessions = ShellSessionApprovals::default();
        let owner = peer(":1.10");
        let id = request("req-1");
        let preview = sessions
            .register_system_uname(owner.clone(), id.clone(), 1_100, 73_u64)
            .expect("register");
        assert!(matches!(
            sessions.register_system_uname(owner.clone(), request("req-2"), 1_100, 74),
            Err(ShellSessionError::ApprovalAlreadyPending)
        ));
        let cancelled = sessions
            .cancel(&owner, &id, &preview.preview_sha256, 1_001)
            .expect("cancel");
        assert_eq!(cancelled.reason, ShellCancellationReason::UserCancelled);
        assert_eq!(cancelled.into_secret(), 73);

        sessions
            .register_system_uname(owner.clone(), request("req-2"), 1_100, 74)
            .expect("register second");
        assert_eq!(
            sessions.expire(&owner, 1_101).map(|item| item.reason),
            Some(ShellCancellationReason::Expired)
        );

        sessions
            .register_system_uname(owner.clone(), request("req-3"), 1_200, 75)
            .expect("register third");
        assert_eq!(
            sessions.disconnect(&owner).map(|item| item.reason),
            Some(ShellCancellationReason::PeerDisconnected)
        );
        assert!(!sessions.has_pending(&owner));
    }
}
