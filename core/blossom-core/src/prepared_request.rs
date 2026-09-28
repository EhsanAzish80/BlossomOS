use crate::request::{RequestError, RequestId};
use crate::workspace_create::ModelWorkspaceCreateProposal;
use crate::{
    ApprovalError, ApprovalToken, AtomicWorkspaceFileCreator, FileContentProvider,
    Openat2FileReader, ServiceSelection, ToolRequest, WorkspaceCreateProvider,
    resolve_model_workspace_proposal, validate_service_unit,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

const MAX_WIRE_REQUEST_BYTES: usize = 512 * 1024;
const MAX_TOOL_NAME_BYTES: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestOrigin {
    UserCli,
    ModelProposed,
    InternalFixed,
}

pub struct SessionContext<'a> {
    pub workspace_root: &'a str,
    pub origin: RequestOrigin,
}

pub(crate) enum PreparedAuthority {
    None,
    FileRead(Openat2FileReader),
    WorkspaceCreate(AtomicWorkspaceFileCreator),
}

pub struct PreparedToolRequest {
    request: ToolRequest,
    origin: RequestOrigin,
    authority: PreparedAuthority,
    _reservation: CapacityReservation,
}

struct PendingPreparedApproval {
    prepared: PreparedToolRequest,
    preview_sha256: String,
    expires_at_ms: u64,
}

pub struct PreparedApprovalStore {
    next_token: u64,
    ttl_ms: u64,
    pending: HashMap<ApprovalToken, PendingPreparedApproval>,
    consumed: HashMap<ApprovalToken, u64>,
}

impl PreparedApprovalStore {
    pub fn new(ttl_ms: u64) -> Self {
        Self {
            next_token: 1,
            ttl_ms,
            pending: HashMap::new(),
            consumed: HashMap::new(),
        }
    }

    pub fn issue(
        &mut self,
        prepared: PreparedToolRequest,
        preview_sha256: String,
        now_ms: u64,
    ) -> ApprovalToken {
        self.sweep(now_ms);
        let token = ApprovalToken(self.next_token);
        self.next_token = self.next_token.checked_add(1).unwrap_or(1);
        self.pending.insert(
            token,
            PendingPreparedApproval {
                prepared,
                preview_sha256,
                expires_at_ms: now_ms.saturating_add(self.ttl_ms),
            },
        );
        token
    }

    pub fn consume(
        &mut self,
        token: ApprovalToken,
        preview_sha256: &str,
        now_ms: u64,
    ) -> Result<PreparedToolRequest, ApprovalError> {
        if self.consumed.contains_key(&token) {
            return Err(ApprovalError::Replay);
        }
        let pending = self.pending.get(&token).ok_or(ApprovalError::Unknown)?;
        if now_ms > pending.expires_at_ms {
            self.pending.remove(&token);
            return Err(ApprovalError::Expired);
        }
        if pending.preview_sha256 != preview_sha256 {
            return Err(ApprovalError::BindingMismatch);
        }
        let pending = self.pending.remove(&token).expect("checked pending entry");
        self.consumed.insert(token, pending.expires_at_ms);
        Ok(pending.prepared)
    }

    pub fn remove(
        &mut self,
        token: ApprovalToken,
        preview_sha256: &str,
        now_ms: u64,
    ) -> Result<(), ApprovalError> {
        self.consume(token, preview_sha256, now_ms).map(drop)
    }

    pub(crate) fn discard(
        &mut self,
        token: ApprovalToken,
        now_ms: u64,
    ) -> Result<PreparedToolRequest, ApprovalError> {
        if self.consumed.contains_key(&token) {
            return Err(ApprovalError::Replay);
        }
        let pending = self.pending.remove(&token).ok_or(ApprovalError::Unknown)?;
        if now_ms > pending.expires_at_ms {
            return Err(ApprovalError::Expired);
        }
        self.consumed.insert(token, pending.expires_at_ms);
        Ok(pending.prepared)
    }

    pub fn sweep(&mut self, now_ms: u64) {
        self.pending
            .retain(|_, pending| now_ms <= pending.expires_at_ms);
        self.consumed.retain(|_, expires| now_ms <= *expires);
    }

    pub fn pending_len(&self) -> usize {
        self.pending.len()
    }

    pub fn request(&self, token: ApprovalToken) -> Option<&ToolRequest> {
        self.pending
            .get(&token)
            .map(|pending| pending.prepared.request())
    }
}

impl PreparedToolRequest {
    pub fn request(&self) -> &ToolRequest {
        &self.request
    }
    pub fn origin(&self) -> RequestOrigin {
        self.origin
    }

    pub(crate) fn into_parts(self) -> (ToolRequest, RequestOrigin, PreparedAuthority) {
        (self.request, self.origin, self.authority)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResolveError {
    UnsupportedOrigin,
    InvalidSelection,
}

pub struct RequestResolver;

impl RequestResolver {
    pub fn resolve(
        wire: ToolRequestWire,
        session: SessionContext<'_>,
        reservation: CapacityReservation,
    ) -> Result<PreparedToolRequest, ResolveError> {
        let origin = session.origin;
        let (request, authority) = match wire {
            ToolRequestWire::FileRead {
                request_id,
                absolute_path,
            } => {
                if origin == RequestOrigin::ModelProposed {
                    return Err(ResolveError::UnsupportedOrigin);
                }
                let provider = Openat2FileReader::select(&absolute_path)
                    .map_err(|_| ResolveError::InvalidSelection)?;
                let selection = provider.selection().clone();
                (
                    ToolRequest::FilesReadContent {
                        request_id,
                        selection,
                    },
                    PreparedAuthority::FileRead(provider),
                )
            }
            ToolRequestWire::WorkspaceCreate {
                request_id,
                proposal,
            } => {
                let provider = resolve_model_workspace_proposal(session.workspace_root, &proposal)
                    .map_err(|_| ResolveError::InvalidSelection)?;
                let selection = provider.selection().clone();
                (
                    ToolRequest::FilesWriteCreate {
                        request_id,
                        selection,
                    },
                    PreparedAuthority::WorkspaceCreate(provider),
                )
            }
            ToolRequestWire::ServiceStatus { request_id, unit } => {
                if origin == RequestOrigin::ModelProposed || validate_service_unit(&unit).is_err() {
                    return Err(ResolveError::InvalidSelection);
                }
                (
                    ToolRequest::ServicesReadStatus {
                        request_id,
                        selection: ServiceSelection { unit },
                    },
                    PreparedAuthority::None,
                )
            }
            ToolRequestWire::Fixed { request_id, tool } => {
                let request = match tool.as_str() {
                    "system.uname" => ToolRequest::SystemUname { request_id },
                    "system.os.identity" => ToolRequest::SystemOsIdentity { request_id },
                    "system.uptime" => ToolRequest::SystemUptime { request_id },
                    "system.memory.summary" => ToolRequest::SystemMemorySummary { request_id },
                    "system.storage.summary" => ToolRequest::SystemStorageSummary { request_id },
                    "system.battery.summary" => ToolRequest::SystemBatterySummary { request_id },
                    "system.network.connectivity" => {
                        ToolRequest::SystemNetworkConnectivity { request_id }
                    }
                    "process.self" => ToolRequest::ProcessSelf { request_id },
                    "process.list" => ToolRequest::ProcessList { request_id },
                    _ => return Err(ResolveError::InvalidSelection),
                };
                (request, PreparedAuthority::None)
            }
        };
        Ok(PreparedToolRequest {
            request,
            origin,
            authority,
            _reservation: reservation,
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ToolRequestWire {
    Fixed {
        request_id: RequestId,
        tool: String,
    },
    FileRead {
        request_id: RequestId,
        absolute_path: String,
    },
    WorkspaceCreate {
        request_id: RequestId,
        proposal: ModelWorkspaceCreateProposal,
    },
    ServiceStatus {
        request_id: RequestId,
        unit: String,
    },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireEnvelope {
    request_id: String,
    tool: String,
    arguments: Value,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EmptyWire {}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FileReadWire {
    absolute_path: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkspaceCreateWire {
    name: String,
    content: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ServiceStatusWire {
    unit: String,
}

impl ToolRequestWire {
    pub fn parse_json(input: &str) -> Result<Self, RequestError> {
        if input.len() > MAX_WIRE_REQUEST_BYTES {
            return Err(RequestError::RequestTooLarge);
        }
        let envelope: WireEnvelope =
            serde_json::from_str(input).map_err(|error| RequestError::MalformedJson {
                message: error.to_string(),
            })?;
        if envelope.tool.is_empty()
            || envelope.tool.len() > MAX_TOOL_NAME_BYTES
            || !envelope
                .tool
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte == b'.')
        {
            return Err(RequestError::InvalidToolName);
        }
        let request_id = RequestId::parse(envelope.request_id)?;
        let invalid = |error: serde_json::Error| RequestError::InvalidArguments {
            message: error.to_string(),
        };
        match envelope.tool.as_str() {
            "files.read.content" => {
                let value: FileReadWire =
                    serde_json::from_value(envelope.arguments).map_err(invalid)?;
                Ok(Self::FileRead {
                    request_id,
                    absolute_path: value.absolute_path,
                })
            }
            "files.write.create" => {
                let value: WorkspaceCreateWire =
                    serde_json::from_value(envelope.arguments).map_err(invalid)?;
                Ok(Self::WorkspaceCreate {
                    request_id,
                    proposal: ModelWorkspaceCreateProposal {
                        name: value.name,
                        content: value.content,
                    },
                })
            }
            "services.read.status" => {
                let value: ServiceStatusWire =
                    serde_json::from_value(envelope.arguments).map_err(invalid)?;
                Ok(Self::ServiceStatus {
                    request_id,
                    unit: value.unit,
                })
            }
            tool @ ("system.uname"
            | "system.os.identity"
            | "system.uptime"
            | "system.memory.summary"
            | "system.storage.summary"
            | "system.battery.summary"
            | "system.network.connectivity"
            | "process.self"
            | "process.list") => {
                serde_json::from_value::<EmptyWire>(envelope.arguments).map_err(invalid)?;
                Ok(Self::Fixed {
                    request_id,
                    tool: tool.into(),
                })
            }
            _ => Err(RequestError::UnknownTool {
                tool: envelope.tool,
            }),
        }
    }
}

#[derive(Debug)]
pub struct ApprovalCapacity {
    state: Arc<CapacityState>,
}

#[derive(Debug)]
struct CapacityState {
    used: AtomicUsize,
    maximum: usize,
}

#[derive(Debug)]
pub struct CapacityReservation {
    state: Arc<CapacityState>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CapacityError {
    InvalidMaximum,
    Exhausted,
}

impl ApprovalCapacity {
    pub fn new(maximum: usize) -> Result<Self, CapacityError> {
        if maximum == 0 {
            return Err(CapacityError::InvalidMaximum);
        }
        Ok(Self {
            state: Arc::new(CapacityState {
                used: AtomicUsize::new(0),
                maximum,
            }),
        })
    }

    pub fn reserve(&self) -> Result<CapacityReservation, CapacityError> {
        self.state
            .used
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |used| {
                (used < self.state.maximum).then_some(used + 1)
            })
            .map_err(|_| CapacityError::Exhausted)?;
        Ok(CapacityReservation {
            state: Arc::clone(&self.state),
        })
    }

    pub fn used(&self) -> usize {
        self.state.used.load(Ordering::Acquire)
    }
}

impl Drop for CapacityReservation {
    fn drop(&mut self) {
        let previous = self.state.used.fetch_sub(1, Ordering::AcqRel);
        debug_assert!(previous > 0, "capacity reservation underflow");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixed_prepared(capacity: &ApprovalCapacity, request_id: &str) -> PreparedToolRequest {
        RequestResolver::resolve(
            ToolRequestWire::Fixed {
                request_id: RequestId::parse(request_id.into()).unwrap(),
                tool: "system.uname".into(),
            },
            SessionContext {
                workspace_root: "/tmp",
                origin: RequestOrigin::InternalFixed,
            },
            capacity.reserve().unwrap(),
        )
        .unwrap()
    }

    fn resolve(
        reservation: CapacityReservation,
        failure: Option<&'static str>,
    ) -> Result<CapacityReservation, &'static str> {
        failure.map_or(Ok(reservation), Err)
    }

    #[test]
    fn every_resolution_error_releases_capacity_by_drop() {
        let capacity = ApprovalCapacity::new(1).expect("capacity");
        for failure in ["bad path", "open error", "identity mismatch"] {
            let reservation = capacity.reserve().expect("reservation");
            assert_eq!(capacity.used(), 1);
            assert!(resolve(reservation, Some(failure)).is_err());
            assert_eq!(capacity.used(), 0);
        }
    }

    #[test]
    fn moving_reservation_retains_capacity_until_owner_drops() {
        let capacity = ApprovalCapacity::new(1).expect("capacity");
        let stored =
            resolve(capacity.reserve().expect("reservation"), None).expect("successful resolution");
        assert_eq!(capacity.used(), 1);
        assert!(matches!(capacity.reserve(), Err(CapacityError::Exhausted)));
        drop(stored);
        assert_eq!(capacity.used(), 0);
    }

    #[test]
    fn store_consumes_prepared_request_by_value_once() {
        let capacity = ApprovalCapacity::new(1).unwrap();
        let mut store = PreparedApprovalStore::new(30);
        let token = store.issue(fixed_prepared(&capacity, "once"), "preview".into(), 10);
        assert_eq!(capacity.used(), 1);
        assert_eq!(store.pending_len(), 1);

        assert!(matches!(
            store.consume(token, "changed", 11),
            Err(ApprovalError::BindingMismatch)
        ));
        assert_eq!(store.pending_len(), 1);
        assert_eq!(capacity.used(), 1);

        let prepared = store.consume(token, "preview", 11).unwrap();
        assert_eq!(store.pending_len(), 0);
        assert_eq!(capacity.used(), 1);
        assert!(matches!(
            store.consume(token, "preview", 11),
            Err(ApprovalError::Replay)
        ));
        drop(prepared);
        assert_eq!(capacity.used(), 0);
    }

    #[test]
    fn remove_and_expiry_drop_prepared_authority_and_capacity() {
        let capacity = ApprovalCapacity::new(1).unwrap();
        let mut store = PreparedApprovalStore::new(10);
        let denied = store.issue(fixed_prepared(&capacity, "deny"), "deny-preview".into(), 0);
        store.remove(denied, "deny-preview", 1).unwrap();
        assert_eq!(capacity.used(), 0);

        let expired = store.issue(fixed_prepared(&capacity, "expired"), "expiry".into(), 20);
        assert_eq!(capacity.used(), 1);
        assert!(matches!(
            store.consume(expired, "expiry", 31),
            Err(ApprovalError::Expired)
        ));
        assert_eq!(capacity.used(), 0);

        let swept = store.issue(fixed_prepared(&capacity, "swept"), "sweep".into(), 40);
        assert_eq!(capacity.used(), 1);
        store.sweep(51);
        assert_eq!(store.pending_len(), 0);
        assert_eq!(capacity.used(), 0);
        assert!(matches!(
            store.consume(swept, "sweep", 51),
            Err(ApprovalError::Unknown)
        ));
    }

    #[test]
    fn wire_rejects_every_caller_supplied_derived_field() {
        let cases = [
            (
                "files.read.content",
                serde_json::json!({"absolute_path":"/tmp/a","inode":2}),
            ),
            (
                "files.read.content",
                serde_json::json!({"absolute_path":"/tmp/a","device":1}),
            ),
            (
                "files.write.create",
                serde_json::json!({"name":"a","content":"x","content_sha256":"00"}),
            ),
            (
                "files.write.create",
                serde_json::json!({"name":"a","content":"x","workspace_root":"/tmp"}),
            ),
            (
                "files.write.create",
                serde_json::json!({"name":"a","content":"x","mode":384}),
            ),
        ];
        for (tool, arguments) in cases {
            let wire = serde_json::json!({"request_id":"wire-1","tool":tool,"arguments":arguments});
            assert!(matches!(
                ToolRequestWire::parse_json(&wire.to_string()),
                Err(RequestError::InvalidArguments { .. })
            ));
        }
    }

    #[test]
    fn wire_preserves_all_closed_parser_bounds() {
        for tool in [
            "system.uname",
            "system.os.identity",
            "system.uptime",
            "system.memory.summary",
            "system.storage.summary",
            "system.battery.summary",
            "system.network.connectivity",
            "process.self",
            "process.list",
        ] {
            let json = serde_json::json!({"request_id":"fixed-1","tool":tool,"arguments":{}});
            assert!(matches!(
                ToolRequestWire::parse_json(&json.to_string()),
                Ok(ToolRequestWire::Fixed { .. })
            ));
        }
        for tool in ["", "System.uname", "system_uname", &"a".repeat(65)] {
            let json = serde_json::json!({"request_id":"fixed-1","tool":tool,"arguments":{}});
            assert_eq!(
                ToolRequestWire::parse_json(&json.to_string()),
                Err(RequestError::InvalidToolName)
            );
        }
        let unknown =
            serde_json::json!({"request_id":"fixed-1","tool":"system.unknown","arguments":{}});
        assert!(matches!(
            ToolRequestWire::parse_json(&unknown.to_string()),
            Err(RequestError::UnknownTool { .. })
        ));
        let expanded = serde_json::json!({"request_id":"fixed-1","tool":"system.uname","arguments":{"extra":true}});
        assert!(matches!(
            ToolRequestWire::parse_json(&expanded.to_string()),
            Err(RequestError::InvalidArguments { .. })
        ));
        let envelope_expanded =
            r#"{"request_id":"fixed-1","tool":"system.uname","arguments":{},"extra":true}"#;
        assert!(matches!(
            ToolRequestWire::parse_json(envelope_expanded),
            Err(RequestError::MalformedJson { .. })
        ));
        assert_eq!(
            ToolRequestWire::parse_json(&"x".repeat(MAX_WIRE_REQUEST_BYTES + 1)),
            Err(RequestError::RequestTooLarge)
        );
    }

    #[test]
    fn thirty_third_reservation_fails_before_resolution_can_open_authority() {
        let capacity = ApprovalCapacity::new(32).unwrap();
        let reservations = (0..32)
            .map(|_| capacity.reserve().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(capacity.used(), 32);
        #[cfg(target_os = "linux")]
        let descriptors_before_rejection = std::fs::read_dir("/proc/self/fd").unwrap().count();
        assert!(matches!(capacity.reserve(), Err(CapacityError::Exhausted)));
        #[cfg(target_os = "linux")]
        assert_eq!(
            std::fs::read_dir("/proc/self/fd").unwrap().count(),
            descriptors_before_rejection
        );
        drop(reservations);
        assert_eq!(capacity.used(), 0);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn descriptor_counts_return_to_baseline_after_deny_cancel_expiry_and_disconnect() {
        use std::fs;

        fn fd_count() -> usize {
            fs::read_dir("/proc/self/fd").unwrap().count()
        }
        fn prepared(path: &str, capacity: &ApprovalCapacity, id: &str) -> PreparedToolRequest {
            RequestResolver::resolve(
                ToolRequestWire::FileRead {
                    request_id: RequestId::parse(id.into()).unwrap(),
                    absolute_path: path.into(),
                },
                SessionContext {
                    workspace_root: "/",
                    origin: RequestOrigin::UserCli,
                },
                capacity.reserve().unwrap(),
            )
            .unwrap()
        }

        let root = std::env::temp_dir().join(format!("blossom-prepared-fd-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir(&root).unwrap();
        let path = root.join("selected.txt");
        fs::write(&path, "selected").unwrap();
        let path = path.to_str().unwrap();
        let baseline = fd_count();
        let capacity = ApprovalCapacity::new(4).unwrap();
        let mut store = PreparedApprovalStore::new(10);

        let deny = store.issue(prepared(path, &capacity, "fd-deny"), "p".into(), 0);
        assert!(fd_count() > baseline);
        store.remove(deny, "p", 1).unwrap();
        assert_eq!(fd_count(), baseline);

        let cancel = store.issue(prepared(path, &capacity, "fd-cancel"), "p".into(), 20);
        assert!(fd_count() > baseline);
        store.remove(cancel, "p", 21).unwrap();
        assert_eq!(fd_count(), baseline);

        let expiry = store.issue(prepared(path, &capacity, "fd-expiry"), "p".into(), 40);
        assert!(fd_count() > baseline);
        assert!(matches!(
            store.consume(expiry, "p", 51),
            Err(ApprovalError::Expired)
        ));
        assert_eq!(fd_count(), baseline);

        let disconnect = store.issue(prepared(path, &capacity, "fd-disconnect"), "p".into(), 60);
        assert!(fd_count() > baseline);
        drop(store.discard(disconnect, 61).unwrap());
        assert_eq!(fd_count(), baseline);
        assert_eq!(capacity.used(), 0);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn real_resolver_failures_drop_the_capacity_reservation() {
        let capacity = ApprovalCapacity::new(1).expect("capacity");
        let file = ToolRequestWire::FileRead {
            request_id: RequestId::parse("bad-file".into()).unwrap(),
            absolute_path: "/definitely/not/a/blossom/file".into(),
        };
        assert!(
            RequestResolver::resolve(
                file,
                SessionContext {
                    workspace_root: "/tmp",
                    origin: RequestOrigin::UserCli
                },
                capacity.reserve().unwrap(),
            )
            .is_err()
        );
        assert_eq!(capacity.used(), 0);

        let workspace = ToolRequestWire::WorkspaceCreate {
            request_id: RequestId::parse("bad-workspace".into()).unwrap(),
            proposal: ModelWorkspaceCreateProposal {
                name: "../escape".into(),
                content: "x".into(),
            },
        };
        assert!(
            RequestResolver::resolve(
                workspace,
                SessionContext {
                    workspace_root: "/tmp",
                    origin: RequestOrigin::ModelProposed
                },
                capacity.reserve().unwrap(),
            )
            .is_err()
        );
        assert_eq!(capacity.used(), 0);
    }
}
