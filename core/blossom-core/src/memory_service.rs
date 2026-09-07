use crate::durable_memory::{
    DurableMemoryRecord, DurableNoteDraft, EncryptedMemoryStore, MEMORY_SCHEMA_VERSION,
    MemoryConsumer, MemoryInputProvenance, MemoryOperation, MemoryPurpose, MemoryRetention,
    MemoryScope, MemoryStoreError, validate_durable_note_draft, validate_memory_record_id,
};
use crate::{Capability, PolicyDecision, PolicyEngine, RequestId};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fmt::{self, Write};

pub const MEMORY_APPROVAL_TTL_MS: u64 = 30_000;
pub const MAX_RECALL_RECORDS: usize = 8;
pub const MAX_RECALL_BYTES: usize = 4 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MemoryApprovalToken(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryDecision {
    ApproveOnce,
    Deny,
    Cancel,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum MemoryMutationRequest {
    Create {
        request_id: RequestId,
        record_id: String,
        draft: DurableNoteDraft,
    },
    Edit {
        request_id: RequestId,
        record_id: String,
        expected_last_user_edit_at_unix_ms: u64,
        provenance: MemoryInputProvenance,
        value: String,
    },
    Delete {
        request_id: RequestId,
        record_id: String,
        expected_last_user_edit_at_unix_ms: u64,
    },
}

impl MemoryMutationRequest {
    pub fn request_id(&self) -> &RequestId {
        match self {
            Self::Create { request_id, .. }
            | Self::Edit { request_id, .. }
            | Self::Delete { request_id, .. } => request_id,
        }
    }

    pub fn operation(&self) -> MemoryOperation {
        match self {
            Self::Create { .. } => MemoryOperation::Create,
            Self::Edit { .. } => MemoryOperation::Edit,
            Self::Delete { .. } => MemoryOperation::Delete,
        }
    }

    fn record_id(&self) -> &str {
        match self {
            Self::Create { record_id, .. }
            | Self::Edit { record_id, .. }
            | Self::Delete { record_id, .. } => record_id,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MemoryApprovalPreview {
    pub schema_version: u16,
    pub request_id: String,
    pub operation: MemoryOperation,
    pub record_id: String,
    pub value: Option<String>,
    pub purpose: &'static str,
    pub scope: &'static str,
    pub retention: &'static str,
    pub consumers: [&'static str; 1],
    pub approval: &'static str,
    pub expires_at_ms: u64,
    pub preview_sha256: String,
}

impl MemoryApprovalPreview {
    fn from_request(request: &MemoryMutationRequest, expires_at_ms: u64) -> Self {
        let value = match request {
            MemoryMutationRequest::Create { draft, .. } => Some(draft.value.clone()),
            MemoryMutationRequest::Edit { value, .. } => Some(value.clone()),
            MemoryMutationRequest::Delete { .. } => None,
        };
        let mut preview = Self {
            schema_version: MEMORY_SCHEMA_VERSION,
            request_id: request.request_id().as_str().into(),
            operation: request.operation(),
            record_id: request.record_id().into(),
            value,
            purpose: "user_preference",
            scope: "user",
            retention: "until_deleted",
            consumers: ["user_controls"],
            approval: "once_only",
            expires_at_ms,
            preview_sha256: String::new(),
        };
        preview.preview_sha256 = preview.digest();
        preview
    }

    pub fn verify_digest(&self) -> bool {
        self.preview_sha256 == self.digest()
    }

    fn digest(&self) -> String {
        #[derive(Serialize)]
        struct Material<'a> {
            schema_version: u16,
            request_id: &'a str,
            operation: MemoryOperation,
            record_id: &'a str,
            value: &'a Option<String>,
            purpose: &'a str,
            scope: &'a str,
            retention: &'a str,
            consumers: [&'a str; 1],
            approval: &'a str,
            expires_at_ms: u64,
        }
        let encoded = serde_json::to_vec(&Material {
            schema_version: self.schema_version,
            request_id: &self.request_id,
            operation: self.operation,
            record_id: &self.record_id,
            value: &self.value,
            purpose: self.purpose,
            scope: self.scope,
            retention: self.retention,
            consumers: self.consumers,
            approval: self.approval,
            expires_at_ms: self.expires_at_ms,
        })
        .expect("fixed memory preview serializes");
        hex_digest(&encoded)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryAuditOutcome {
    ApprovalIssued,
    Denied,
    Cancelled,
    Expired,
    Rejected,
    Verified,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MemoryAuditEvent {
    pub sequence: u64,
    pub request_id: String,
    pub operation: MemoryOperation,
    pub record_id_sha256: String,
    pub outcome: MemoryAuditOutcome,
    pub previous_sha256: String,
    pub event_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MemoryMutationResult {
    pub request_id: String,
    pub operation: MemoryOperation,
    pub record_id: String,
    pub verified: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MemoryRecallProjection {
    pub schema_version: u16,
    pub records: Vec<DurableMemoryRecord>,
    pub truncated: bool,
    pub authority: MemoryRecallAuthority,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryRecallAuthority {
    DataOnlyNotPermission,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryServiceError {
    Disabled,
    PolicyDenied,
    ApprovalRequired,
    UnknownApproval,
    ApprovalExpired,
    ApprovalReplay,
    ApprovalBindingMismatch,
    UserDenied,
    UserCancelled,
    InvalidRequest,
    RecordNotFound,
    RecordConflict,
    StoreFailed,
    VerificationFailed,
    RecallTooLarge,
}

impl fmt::Display for MemoryServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:?}", self)
    }
}

impl std::error::Error for MemoryServiceError {}

#[derive(Clone, Debug)]
struct PendingMemoryApproval {
    request: MemoryMutationRequest,
    expires_at_ms: u64,
}

pub struct DurableMemoryService {
    store: EncryptedMemoryStore,
    policy: PolicyEngine,
    enabled: bool,
    next_token: u64,
    pending: HashMap<MemoryApprovalToken, PendingMemoryApproval>,
    consumed: HashSet<MemoryApprovalToken>,
    audit: Vec<MemoryAuditEvent>,
}

impl DurableMemoryService {
    pub fn new_inactive(store: EncryptedMemoryStore, policy: PolicyEngine) -> Self {
        Self {
            store,
            policy,
            enabled: false,
            next_token: 1,
            pending: HashMap::new(),
            consumed: HashSet::new(),
            audit: Vec::new(),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn enable(&mut self) -> Result<(), MemoryServiceError> {
        self.require_allowed(Capability::MemoryDurableEnable)?;
        self.enabled = true;
        Ok(())
    }

    pub fn disable(&mut self) -> Result<(), MemoryServiceError> {
        self.require_allowed(Capability::MemoryDurableDisable)?;
        self.enabled = false;
        self.pending.clear();
        Ok(())
    }

    pub fn prepare_create(
        &self,
        request_id: RequestId,
        value: String,
    ) -> Result<MemoryMutationRequest, MemoryServiceError> {
        self.require_enabled()?;
        let draft = DurableNoteDraft::explicit_user_note(value);
        validate_durable_note_draft(&draft).map_err(|_| MemoryServiceError::InvalidRequest)?;
        let mut random = [0u8; 16];
        getrandom::fill(&mut random).map_err(|_| MemoryServiceError::InvalidRequest)?;
        Ok(MemoryMutationRequest::Create {
            request_id,
            record_id: format!("mem-{}", encode_hex(&random)),
            draft,
        })
    }

    pub fn begin_mutation(
        &mut self,
        request: MemoryMutationRequest,
        now_ms: u64,
    ) -> Result<(MemoryApprovalToken, MemoryApprovalPreview), MemoryServiceError> {
        self.require_enabled()?;
        validate_mutation(&request)?;
        if self
            .policy
            .evaluate_capability(request.operation().capability())
            != PolicyDecision::Ask
        {
            self.audit(&request, MemoryAuditOutcome::Denied);
            return Err(MemoryServiceError::PolicyDenied);
        }
        let token = MemoryApprovalToken(self.next_token);
        self.next_token = self.next_token.checked_add(1).unwrap_or(1);
        let expires_at_ms = now_ms.saturating_add(MEMORY_APPROVAL_TTL_MS);
        let preview = MemoryApprovalPreview::from_request(&request, expires_at_ms);
        self.pending.insert(
            token,
            PendingMemoryApproval {
                request: request.clone(),
                expires_at_ms,
            },
        );
        self.audit(&request, MemoryAuditOutcome::ApprovalIssued);
        Ok((token, preview))
    }

    pub fn complete_mutation(
        &mut self,
        token: MemoryApprovalToken,
        request: &MemoryMutationRequest,
        preview_sha256: &str,
        decision: MemoryDecision,
        now_ms: u64,
    ) -> Result<MemoryMutationResult, MemoryServiceError> {
        self.require_enabled()?;
        if self.consumed.contains(&token) {
            return Err(MemoryServiceError::ApprovalReplay);
        }
        let pending = self
            .pending
            .get(&token)
            .ok_or(MemoryServiceError::UnknownApproval)?;
        if now_ms > pending.expires_at_ms {
            let expired = self.pending.remove(&token).expect("checked pending");
            self.consumed.insert(token);
            self.audit(&expired.request, MemoryAuditOutcome::Expired);
            return Err(MemoryServiceError::ApprovalExpired);
        }
        let expected_preview =
            MemoryApprovalPreview::from_request(&pending.request, pending.expires_at_ms);
        if &pending.request != request || expected_preview.preview_sha256 != preview_sha256 {
            return Err(MemoryServiceError::ApprovalBindingMismatch);
        }
        let pending = self.pending.remove(&token).expect("checked pending");
        self.consumed.insert(token);
        match decision {
            MemoryDecision::Deny => {
                self.audit(&pending.request, MemoryAuditOutcome::Denied);
                return Err(MemoryServiceError::UserDenied);
            }
            MemoryDecision::Cancel => {
                self.audit(&pending.request, MemoryAuditOutcome::Cancelled);
                return Err(MemoryServiceError::UserCancelled);
            }
            MemoryDecision::ApproveOnce => {}
        }

        let mut records = self.load_or_empty()?;
        apply_mutation(&mut records, &pending.request, now_ms)?;
        self.store
            .replace_all(&records)
            .map_err(|_| MemoryServiceError::StoreFailed)?;
        let verified = self
            .store
            .load()
            .map_err(|_| MemoryServiceError::StoreFailed)?;
        if verified != records {
            self.audit(&pending.request, MemoryAuditOutcome::Failed);
            return Err(MemoryServiceError::VerificationFailed);
        }
        self.audit(&pending.request, MemoryAuditOutcome::Verified);
        Ok(MemoryMutationResult {
            request_id: pending.request.request_id().as_str().into(),
            operation: pending.request.operation(),
            record_id: pending.request.record_id().into(),
            verified: true,
        })
    }

    pub fn inspect(&self) -> Result<Vec<DurableMemoryRecord>, MemoryServiceError> {
        self.require_enabled()?;
        self.require_allowed(Capability::MemoryDurableInspect)?;
        self.load_or_empty()
    }

    pub fn export_json(&self) -> Result<Vec<u8>, MemoryServiceError> {
        self.require_enabled()?;
        self.require_allowed(Capability::MemoryDurableExport)?;
        serde_json::to_vec(&self.load_or_empty()?).map_err(|_| MemoryServiceError::StoreFailed)
    }

    pub fn confirm_until_deleted_retention(
        &self,
        record_id: &str,
    ) -> Result<MemoryRetention, MemoryServiceError> {
        self.require_enabled()?;
        self.require_allowed(Capability::MemoryDurableSetRetention)?;
        validate_memory_record_id(record_id).map_err(|_| MemoryServiceError::InvalidRequest)?;
        let record = self
            .load_or_empty()?
            .into_iter()
            .find(|record| record.record_id == record_id)
            .ok_or(MemoryServiceError::RecordNotFound)?;
        Ok(record.retention)
    }

    pub fn recall(&self) -> Result<MemoryRecallProjection, MemoryServiceError> {
        self.require_enabled()?;
        self.require_allowed(Capability::MemoryDurableRecall)?;
        let all = self.load_or_empty()?;
        let truncated = all.len() > MAX_RECALL_RECORDS;
        let mut records = Vec::new();
        let mut bytes = 0usize;
        for record in all.into_iter().rev().take(MAX_RECALL_RECORDS).rev() {
            bytes = bytes
                .checked_add(record.value.len())
                .ok_or(MemoryServiceError::RecallTooLarge)?;
            if bytes > MAX_RECALL_BYTES {
                return Err(MemoryServiceError::RecallTooLarge);
            }
            records.push(record);
        }
        Ok(MemoryRecallProjection {
            schema_version: MEMORY_SCHEMA_VERSION,
            records,
            truncated,
            authority: MemoryRecallAuthority::DataOnlyNotPermission,
        })
    }

    pub fn audit_events(&self) -> &[MemoryAuditEvent] {
        &self.audit
    }

    pub fn verify_audit_chain(&self) -> bool {
        let mut previous = String::new();
        for (index, event) in self.audit.iter().enumerate() {
            if event.sequence != index as u64 + 1 || event.previous_sha256 != previous {
                return false;
            }
            if event.event_sha256 != audit_digest(event) {
                return false;
            }
            previous.clone_from(&event.event_sha256);
        }
        true
    }

    fn require_enabled(&self) -> Result<(), MemoryServiceError> {
        if self.enabled {
            Ok(())
        } else {
            Err(MemoryServiceError::Disabled)
        }
    }

    fn require_allowed(&self, capability: Capability) -> Result<(), MemoryServiceError> {
        if self.policy.evaluate_capability(capability) == PolicyDecision::Allow {
            Ok(())
        } else {
            Err(MemoryServiceError::PolicyDenied)
        }
    }

    fn load_or_empty(&self) -> Result<Vec<DurableMemoryRecord>, MemoryServiceError> {
        match self.store.load() {
            Ok(records) => Ok(records),
            Err(MemoryStoreError::StoreMissing) => Ok(Vec::new()),
            Err(_) => Err(MemoryServiceError::StoreFailed),
        }
    }

    fn audit(&mut self, request: &MemoryMutationRequest, outcome: MemoryAuditOutcome) {
        let mut event = MemoryAuditEvent {
            sequence: self.audit.len() as u64 + 1,
            request_id: request.request_id().as_str().into(),
            operation: request.operation(),
            record_id_sha256: hex_digest(request.record_id().as_bytes()),
            outcome,
            previous_sha256: self
                .audit
                .last()
                .map_or_else(String::new, |event| event.event_sha256.clone()),
            event_sha256: String::new(),
        };
        event.event_sha256 = audit_digest(&event);
        self.audit.push(event);
    }
}

fn audit_digest(event: &MemoryAuditEvent) -> String {
    #[derive(Serialize)]
    struct AuditMaterial<'a> {
        sequence: u64,
        request_id: &'a str,
        operation: MemoryOperation,
        record_id_sha256: &'a str,
        outcome: MemoryAuditOutcome,
        previous_sha256: &'a str,
    }
    let encoded = serde_json::to_vec(&AuditMaterial {
        sequence: event.sequence,
        request_id: &event.request_id,
        operation: event.operation,
        record_id_sha256: &event.record_id_sha256,
        outcome: event.outcome,
        previous_sha256: &event.previous_sha256,
    })
    .expect("fixed memory audit event serializes");
    hex_digest(&encoded)
}

fn validate_mutation(request: &MemoryMutationRequest) -> Result<(), MemoryServiceError> {
    validate_memory_record_id(request.record_id())
        .map_err(|_| MemoryServiceError::InvalidRequest)?;
    match request {
        MemoryMutationRequest::Create { draft, .. } => {
            validate_durable_note_draft(draft).map_err(|_| MemoryServiceError::InvalidRequest)
        }
        MemoryMutationRequest::Edit {
            provenance, value, ..
        } => {
            if *provenance != MemoryInputProvenance::UserAuthored
                || value.is_empty()
                || value.len() > crate::MAX_DURABLE_NOTE_BYTES
            {
                Err(MemoryServiceError::InvalidRequest)
            } else {
                Ok(())
            }
        }
        MemoryMutationRequest::Delete { .. } => Ok(()),
    }
}

fn apply_mutation(
    records: &mut Vec<DurableMemoryRecord>,
    request: &MemoryMutationRequest,
    now_ms: u64,
) -> Result<(), MemoryServiceError> {
    match request {
        MemoryMutationRequest::Create {
            record_id, draft, ..
        } => {
            if records.iter().any(|record| record.record_id == *record_id) {
                return Err(MemoryServiceError::RecordConflict);
            }
            records.push(DurableMemoryRecord {
                record_id: record_id.clone(),
                schema_version: MEMORY_SCHEMA_VERSION,
                purpose: MemoryPurpose::UserPreference,
                scope: MemoryScope::User,
                retention: MemoryRetention::UntilDeleted,
                consumers: vec![MemoryConsumer::UserControls],
                created_at_unix_ms: now_ms,
                last_user_edit_at_unix_ms: now_ms,
                value: draft.value.clone(),
            });
        }
        MemoryMutationRequest::Edit {
            record_id,
            expected_last_user_edit_at_unix_ms,
            value,
            ..
        } => {
            let record = records
                .iter_mut()
                .find(|record| record.record_id == *record_id)
                .ok_or(MemoryServiceError::RecordNotFound)?;
            if record.last_user_edit_at_unix_ms != *expected_last_user_edit_at_unix_ms {
                return Err(MemoryServiceError::RecordConflict);
            }
            record.value = value.clone();
            record.last_user_edit_at_unix_ms = now_ms;
        }
        MemoryMutationRequest::Delete {
            record_id,
            expected_last_user_edit_at_unix_ms,
            ..
        } => {
            let index = records
                .iter()
                .position(|record| record.record_id == *record_id)
                .ok_or(MemoryServiceError::RecordNotFound)?;
            if records[index].last_user_edit_at_unix_ms != *expected_last_user_edit_at_unix_ms {
                return Err(MemoryServiceError::RecordConflict);
            }
            records.remove(index);
        }
    }
    Ok(())
}

fn hex_digest(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    encode_hex(&digest)
}

fn encode_hex(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut encoded, "{byte:02x}").expect("writing to a String cannot fail");
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PolicyRule, durable_memory::MemoryRetention};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn request_id(value: &str) -> RequestId {
        RequestId::parse(value.into()).expect("valid request id")
    }

    fn policy() -> PolicyEngine {
        let ask = [
            Capability::MemoryDurableCreate,
            Capability::MemoryDurableEdit,
            Capability::MemoryDurableDelete,
        ]
        .into_iter()
        .map(|capability| PolicyRule {
            capability,
            decision: PolicyDecision::Ask,
        });
        let allow = [
            Capability::MemoryDurableEnable,
            Capability::MemoryDurableDisable,
            Capability::MemoryDurableInspect,
            Capability::MemoryDurableExport,
            Capability::MemoryDurableSetRetention,
            Capability::MemoryDurableRecall,
        ]
        .into_iter()
        .map(|capability| PolicyRule {
            capability,
            decision: PolicyDecision::Allow,
        });
        PolicyEngine::new(ask.chain(allow).collect())
    }

    fn service(label: &str) -> (PathBuf, DurableMemoryService) {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        let root = std::env::temp_dir().join(format!(
            "blossom-memory-service-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&root).expect("root created");
        let store =
            EncryptedMemoryStore::initialize_inactive(&root.join("data"), &root.join("keys"))
                .expect("store created");
        (root, DurableMemoryService::new_inactive(store, policy()))
    }

    fn approve_create(
        service: &mut DurableMemoryService,
        id: &str,
        value: &str,
        now_ms: u64,
    ) -> MemoryMutationResult {
        let request = service
            .prepare_create(request_id(id), value.into())
            .expect("create prepared");
        let (token, preview) = service
            .begin_mutation(request.clone(), now_ms)
            .expect("approval issued");
        assert!(preview.verify_digest());
        service
            .complete_mutation(
                token,
                &request,
                &preview.preview_sha256,
                MemoryDecision::ApproveOnce,
                now_ms + 1,
            )
            .expect("create verified")
    }

    #[test]
    fn disabled_default_and_policy_cannot_silently_enable_or_write() {
        let (root, mut service) = service("disabled");
        assert!(!service.is_enabled());
        assert_eq!(
            service.prepare_create(request_id("disabled"), "value".into()),
            Err(MemoryServiceError::Disabled)
        );

        let store = EncryptedMemoryStore::initialize_inactive(
            &root.join("other-data"),
            &root.join("other-keys"),
        )
        .expect("second store");
        let mut denied = DurableMemoryService::new_inactive(store, PolicyEngine::default());
        assert_eq!(denied.enable(), Err(MemoryServiceError::PolicyDenied));
        service.enable().expect("explicit enable allowed");
        assert!(service.is_enabled());
        std::fs::remove_dir_all(root).expect("test removed");
    }

    #[test]
    fn create_preview_binds_value_metadata_and_is_once_only() {
        let (root, mut service) = service("binding");
        service.enable().expect("enabled");
        let request = service
            .prepare_create(request_id("create-1"), "exact private marker".into())
            .expect("prepared");
        let (token, preview) = service
            .begin_mutation(request.clone(), 1_000)
            .expect("begun");
        assert!(preview.verify_digest());
        let mut changed = request.clone();
        let MemoryMutationRequest::Create { draft, .. } = &mut changed else {
            unreachable!()
        };
        draft.value.push_str(" changed");
        assert_eq!(
            service.complete_mutation(
                token,
                &changed,
                &preview.preview_sha256,
                MemoryDecision::ApproveOnce,
                1_001
            ),
            Err(MemoryServiceError::ApprovalBindingMismatch)
        );
        assert!(service.inspect().expect("inspected").is_empty());
        let result = service
            .complete_mutation(
                token,
                &request,
                &preview.preview_sha256,
                MemoryDecision::ApproveOnce,
                1_002,
            )
            .expect("completed");
        assert!(result.verified);
        assert_eq!(
            service.inspect().expect("inspected")[0].value,
            "exact private marker"
        );
        assert_eq!(
            service.complete_mutation(
                token,
                &request,
                &preview.preview_sha256,
                MemoryDecision::ApproveOnce,
                1_003
            ),
            Err(MemoryServiceError::ApprovalReplay)
        );
        let audit = serde_json::to_string(service.audit_events()).expect("audit serialized");
        assert!(!audit.contains("exact private marker"));
        assert!(audit.contains("verified"));
        assert!(service.verify_audit_chain());
        std::fs::remove_dir_all(root).expect("test removed");
    }

    #[test]
    fn denial_cancellation_expiry_and_service_loss_leave_no_record() {
        let (root, mut service) = service("negative");
        service.enable().expect("enabled");
        for (suffix, decision) in [
            ("deny", MemoryDecision::Deny),
            ("cancel", MemoryDecision::Cancel),
        ] {
            let request = service
                .prepare_create(request_id(suffix), suffix.into())
                .expect("prepared");
            let (token, preview) = service
                .begin_mutation(request.clone(), 1_000)
                .expect("begun");
            assert!(
                service
                    .complete_mutation(token, &request, &preview.preview_sha256, decision, 1_001)
                    .is_err()
            );
        }
        let request = service
            .prepare_create(request_id("expired"), "expired".into())
            .expect("prepared");
        let (token, preview) = service
            .begin_mutation(request.clone(), 1_000)
            .expect("begun");
        assert_eq!(
            service.complete_mutation(
                token,
                &request,
                &preview.preview_sha256,
                MemoryDecision::ApproveOnce,
                31_001
            ),
            Err(MemoryServiceError::ApprovalExpired)
        );

        let lost = service
            .prepare_create(request_id("lost"), "lost".into())
            .expect("prepared");
        let (lost_token, lost_preview) =
            service.begin_mutation(lost.clone(), 50_000).expect("begun");
        drop(service);
        let store =
            EncryptedMemoryStore::initialize_inactive(&root.join("data"), &root.join("keys"))
                .expect("store reopened");
        let mut replacement = DurableMemoryService::new_inactive(store, policy());
        replacement.enable().expect("replacement enabled");
        assert_eq!(
            replacement.complete_mutation(
                lost_token,
                &lost,
                &lost_preview.preview_sha256,
                MemoryDecision::ApproveOnce,
                50_001
            ),
            Err(MemoryServiceError::UnknownApproval)
        );
        assert!(replacement.inspect().expect("inspected").is_empty());
        std::fs::remove_dir_all(root).expect("test removed");
    }

    #[test]
    fn edit_delete_export_retention_disable_and_recall_are_fixed() {
        let (root, mut service) = service("lifecycle");
        service.enable().expect("enabled");
        let created = approve_create(&mut service, "create", "first", 100);
        let original = service.inspect().expect("inspected")[0].clone();

        let edit = MemoryMutationRequest::Edit {
            request_id: request_id("edit"),
            record_id: created.record_id.clone(),
            expected_last_user_edit_at_unix_ms: original.last_user_edit_at_unix_ms,
            provenance: MemoryInputProvenance::UserAuthored,
            value: "second".into(),
        };
        let (token, preview) = service
            .begin_mutation(edit.clone(), 200)
            .expect("edit begun");
        service
            .complete_mutation(
                token,
                &edit,
                &preview.preview_sha256,
                MemoryDecision::ApproveOnce,
                201,
            )
            .expect("edit completed");
        let edited = service.inspect().expect("inspected")[0].clone();
        assert_eq!(edited.value, "second");
        assert_eq!(
            service.confirm_until_deleted_retention(&created.record_id),
            Ok(MemoryRetention::UntilDeleted)
        );
        assert!(
            String::from_utf8(service.export_json().expect("exported"))
                .expect("utf8")
                .contains("second")
        );
        let recall = service.recall().expect("recalled");
        assert_eq!(recall.records, vec![edited.clone()]);
        assert_eq!(
            recall.authority,
            MemoryRecallAuthority::DataOnlyNotPermission
        );

        let delete = MemoryMutationRequest::Delete {
            request_id: request_id("delete"),
            record_id: created.record_id,
            expected_last_user_edit_at_unix_ms: edited.last_user_edit_at_unix_ms,
        };
        let (token, preview) = service
            .begin_mutation(delete.clone(), 300)
            .expect("delete begun");
        service
            .complete_mutation(
                token,
                &delete,
                &preview.preview_sha256,
                MemoryDecision::ApproveOnce,
                301,
            )
            .expect("delete completed");
        assert!(service.inspect().expect("inspected").is_empty());
        service.disable().expect("disabled");
        assert_eq!(service.inspect(), Err(MemoryServiceError::Disabled));
        std::fs::remove_dir_all(root).expect("test removed");
    }

    #[test]
    fn policy_allow_cannot_bypass_mandatory_mutation_approval() {
        let (root, _) = service("allow-bypass");
        let store = EncryptedMemoryStore::initialize_inactive(
            &root.join("allow-data"),
            &root.join("allow-keys"),
        )
        .expect("store created");
        let mut service = DurableMemoryService::new_inactive(
            store,
            PolicyEngine::new(vec![
                PolicyRule {
                    capability: Capability::MemoryDurableEnable,
                    decision: PolicyDecision::Allow,
                },
                PolicyRule {
                    capability: Capability::MemoryDurableCreate,
                    decision: PolicyDecision::Allow,
                },
            ]),
        );
        service.enable().expect("enabled");
        let request = service
            .prepare_create(request_id("no-bypass"), "value".into())
            .expect("prepared");
        assert_eq!(
            service.begin_mutation(request, 1_000),
            Err(MemoryServiceError::PolicyDenied)
        );
        std::fs::remove_dir_all(root).expect("test removed");
    }

    #[test]
    fn content_free_audit_chain_detects_mutation() {
        let (root, mut service) = service("audit-chain");
        service.enable().expect("enabled");
        approve_create(&mut service, "audit-create", "never in audit", 1_000);
        assert!(service.verify_audit_chain());
        assert!(
            !serde_json::to_string(service.audit_events())
                .expect("audit serialized")
                .contains("never in audit")
        );
        service.audit[0].outcome = MemoryAuditOutcome::Failed;
        assert!(!service.verify_audit_chain());
        std::fs::remove_dir_all(root).expect("test removed");
    }
}
