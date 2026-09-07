#![forbid(unsafe_code)]

use blossom_core::{
    Capability, DurableMemoryService, EncryptedMemoryStore, MemoryDecision, MemoryInputProvenance,
    MemoryMutationRequest, MemoryRecallAuthority, PolicyDecision, PolicyEngine, PolicyRule,
    RequestId,
};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const MARKER: &str = "phase8-installed-private-marker";

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

fn request_id(value: &str) -> RequestId {
    RequestId::parse(value.into()).expect("fixed evidence request ID")
}

fn root() -> PathBuf {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "blossom-phase8-memory-{}-{now}",
        std::process::id()
    ))
}

fn approve(service: &mut DurableMemoryService, request: &MemoryMutationRequest, now_ms: u64) {
    let (token, preview) = service
        .begin_mutation(request.clone(), now_ms)
        .expect("exact approval issued");
    assert!(preview.verify_digest());
    service
        .complete_mutation(
            token,
            request,
            &preview.preview_sha256,
            MemoryDecision::ApproveOnce,
            now_ms + 1,
        )
        .expect("approved mutation verified");
}

fn main() {
    let expected_arch = std::env::args().nth(1);
    if let Some(expected_arch) = expected_arch {
        assert_eq!(std::env::consts::ARCH, expected_arch);
    }
    let root = root();
    let data = root.join("data");
    let keys = root.join("keys");
    fs::create_dir(&root).expect("evidence root");
    let store = EncryptedMemoryStore::initialize_inactive(&data, &keys).expect("inactive store");
    let mut service = DurableMemoryService::new_inactive(store, policy());
    assert!(!service.is_enabled());
    service.enable().expect("explicit enable");

    let create = service
        .prepare_create(request_id("phase8-create"), MARKER.into())
        .expect("create prepared");
    approve(&mut service, &create, 1_000);
    let created = service.inspect().expect("inspect")[0].clone();
    assert!(
        !fs::read(data.join("records.v1"))
            .expect("encrypted store")
            .windows(MARKER.len())
            .any(|window| window == MARKER.as_bytes())
    );
    assert_eq!(
        fs::metadata(&data)
            .expect("data metadata")
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
    assert_eq!(
        fs::metadata(&keys)
            .expect("key metadata")
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
    assert_eq!(
        fs::metadata(data.join("records.v1"))
            .expect("store metadata")
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    assert_eq!(
        fs::metadata(keys.join("memory.v1.key"))
            .expect("key metadata")
            .permissions()
            .mode()
            & 0o777,
        0o600
    );

    drop(service);
    let reopened = EncryptedMemoryStore::initialize_inactive(&data, &keys).expect("store reopened");
    let mut service = DurableMemoryService::new_inactive(reopened, policy());
    service.enable().expect("explicit re-enable");
    assert_eq!(
        service.inspect().expect("restart inspect"),
        vec![created.clone()]
    );

    let edit = MemoryMutationRequest::Edit {
        request_id: request_id("phase8-edit"),
        record_id: created.record_id.clone(),
        expected_last_user_edit_at_unix_ms: created.last_user_edit_at_unix_ms,
        provenance: MemoryInputProvenance::UserAuthored,
        value: "phase8-installed-edited-marker".into(),
    };
    approve(&mut service, &edit, 2_000);
    let edited = service.inspect().expect("edited inspect")[0].clone();
    assert_eq!(
        service.recall().expect("bounded recall").authority,
        MemoryRecallAuthority::DataOnlyNotPermission
    );
    assert!(
        String::from_utf8(service.export_json().expect("export"))
            .expect("UTF-8 export")
            .contains("phase8-installed-edited-marker")
    );

    let delete = MemoryMutationRequest::Delete {
        request_id: request_id("phase8-delete"),
        record_id: edited.record_id,
        expected_last_user_edit_at_unix_ms: edited.last_user_edit_at_unix_ms,
    };
    approve(&mut service, &delete, 3_000);
    assert!(service.inspect().expect("post-delete inspect").is_empty());
    assert!(service.verify_audit_chain());
    let audit = serde_json::to_string(service.audit_events()).expect("audit serialization");
    assert!(!audit.contains("phase8-installed"));
    service.disable().expect("disabled");
    assert!(!service.is_enabled());

    fs::remove_dir_all(&root).expect("evidence cleanup");
    println!("memory-evidence=verified schema=1 encrypted=yes restart=yes deleted=yes");
}
