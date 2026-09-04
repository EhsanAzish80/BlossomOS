#![forbid(unsafe_code)]

//! Fail-closed process boundary for the future local model gateway.
//!
//! Default release builds remain fail closed. Target-Linux packages may compile
//! the fixed, credential-gated production Unix listener only through an explicit
//! feature after installed evidence passes. Debug builds retain one synthetic
//! fixture mode for separate-process protocol evidence.

use std::fmt;

mod audit;

#[cfg(all(target_os = "linux", feature = "production-private-inference"))]
const PRODUCTION_AUDIT_PATH: &str = "/run/blossom-model-gateway/audit";

pub const PRODUCTION_SOCKET_PATH: &str = "/run/blossom-model-gateway/inference.sock";
#[cfg(all(target_os = "linux", feature = "production-private-inference"))]
const PRODUCTION_PROFILE_PATH: &str = "/etc/blossom-os/model-profiles/active.json";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GatewayProcessError {
    ProfileRegistryUnavailable,
    InvalidInvocation,
    InvalidFixtureConfiguration,
    FixtureUnavailable,
    PrivateConnectionUnavailable,
}

impl GatewayProcessError {
    pub fn exit_code(self) -> i32 {
        match self {
            Self::ProfileRegistryUnavailable => 78,
            Self::InvalidInvocation | Self::InvalidFixtureConfiguration => 64,
            Self::FixtureUnavailable => 69,
            Self::PrivateConnectionUnavailable => 70,
        }
    }
}

impl fmt::Display for GatewayProcessError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::ProfileRegistryUnavailable => "model gateway is not production-ready",
            Self::InvalidInvocation => "model gateway invocation is invalid",
            Self::InvalidFixtureConfiguration => "synthetic gateway configuration is invalid",
            Self::FixtureUnavailable => "synthetic gateway fixture is unavailable",
            Self::PrivateConnectionUnavailable => "private gateway connection failed closed",
        })
    }
}

#[cfg(unix)]
struct PrivateConnectionContext<'a> {
    profile: blossom_core::GatewayProfile,
    provider: blossom_core::ModelProviderKind,
    model: blossom_core::ModelProfile,
    boot_id_sha256: &'a str,
    instance_nonce: &'a str,
    instance_sha256: &'a str,
    client_uid_sha256: &'a str,
}

#[cfg(unix)]
#[cfg_attr(
    not(all(target_os = "linux", feature = "production-private-inference")),
    allow(
        dead_code,
        reason = "production listener is target-Linux and package-feature gated"
    )
)]
fn serve_authorized_private_connection<F>(
    mut stream: std::os::unix::net::UnixStream,
    context: PrivateConnectionContext<'_>,
    audit: &mut dyn audit::GatewayAudit,
    inference: F,
) -> Result<(), GatewayProcessError>
where
    F: FnOnce(
        &blossom_core::InferenceRequest,
        blossom_core::InferenceCancellation,
        &mut dyn FnMut(&blossom_core::NormalizedStreamEvent),
    ) -> Result<(), GatewayProcessError>,
{
    use audit::{GatewayAuditEvent, GatewayAuditOutcome, domain_digest};
    use blossom_core::{
        GatewayFrame, GatewayFrameDecoder, InferenceCancellation, decode_gateway_cancel,
        decode_gateway_private_request, encode_gateway_event, encode_gateway_hello,
    };
    use std::collections::VecDeque;
    use std::io::{Read, Write};
    use std::net::Shutdown;
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    use std::time::Duration;
    use std::time::Instant;

    struct Reader {
        decoder: GatewayFrameDecoder,
        pending: VecDeque<GatewayFrame>,
    }

    enum ReadFailure {
        TimedOut,
        Failed,
    }

    impl Reader {
        fn new() -> Self {
            Self {
                decoder: GatewayFrameDecoder::default(),
                pending: VecDeque::new(),
            }
        }

        fn read_one(
            &mut self,
            stream: &mut std::os::unix::net::UnixStream,
        ) -> Result<GatewayFrame, ReadFailure> {
            if let Some(frame) = self.pending.pop_front() {
                return Ok(frame);
            }
            let mut buffer = [0_u8; 8 * 1024];
            loop {
                let count = stream.read(&mut buffer).map_err(|error| {
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                    ) {
                        ReadFailure::TimedOut
                    } else {
                        ReadFailure::Failed
                    }
                })?;
                if count == 0 {
                    return Err(ReadFailure::Failed);
                }
                self.pending.extend(
                    self.decoder
                        .push(&buffer[..count])
                        .map_err(|_| ReadFailure::Failed)?,
                );
                if let Some(frame) = self.pending.pop_front() {
                    return Ok(frame);
                }
            }
        }

        fn is_idle(&self) -> bool {
            self.pending.is_empty() && self.decoder.is_idle()
        }
    }

    stream
        .set_read_timeout(Some(Duration::from_millis(250)))
        .map_err(|_| GatewayProcessError::PrivateConnectionUnavailable)?;
    stream
        .set_write_timeout(Some(Duration::from_secs(2)))
        .map_err(|_| GatewayProcessError::PrivateConnectionUnavailable)?;
    stream
        .write_all(
            &encode_gateway_hello(
                context.profile,
                context.boot_id_sha256,
                context.instance_nonce,
            )
            .map_err(|_| GatewayProcessError::PrivateConnectionUnavailable)?,
        )
        .map_err(|_| GatewayProcessError::PrivateConnectionUnavailable)?;
    stream
        .flush()
        .map_err(|_| GatewayProcessError::PrivateConnectionUnavailable)?;

    let mut reader = Reader::new();
    let frame = match reader.read_one(&mut stream) {
        Ok(frame) => frame,
        Err(_) => {
            audit
                .record(GatewayAuditEvent::ConnectionRejected {
                    instance_sha256: context.instance_sha256.into(),
                    client_uid_sha256: context.client_uid_sha256.into(),
                    outcome: GatewayAuditOutcome::ProtocolRejected,
                })
                .map_err(|_| GatewayProcessError::PrivateConnectionUnavailable)?;
            return Err(GatewayProcessError::PrivateConnectionUnavailable);
        }
    };
    let request = match decode_gateway_private_request(&frame, context.provider, context.model) {
        Ok(request) => request,
        Err(_) => {
            audit
                .record(GatewayAuditEvent::ConnectionRejected {
                    instance_sha256: context.instance_sha256.into(),
                    client_uid_sha256: context.client_uid_sha256.into(),
                    outcome: GatewayAuditOutcome::ProtocolRejected,
                })
                .map_err(|_| GatewayProcessError::PrivateConnectionUnavailable)?;
            return Err(GatewayProcessError::PrivateConnectionUnavailable);
        }
    };
    if !reader.is_idle() {
        audit
            .record(GatewayAuditEvent::ConnectionRejected {
                instance_sha256: context.instance_sha256.into(),
                client_uid_sha256: context.client_uid_sha256.into(),
                outcome: GatewayAuditOutcome::ProtocolRejected,
            })
            .map_err(|_| GatewayProcessError::PrivateConnectionUnavailable)?;
        return Err(GatewayProcessError::PrivateConnectionUnavailable);
    }
    let request_id_sha256 = domain_digest(
        "request_id",
        context.instance_nonce,
        request.request_id().as_str().as_bytes(),
    );
    audit
        .record(GatewayAuditEvent::RequestStarted {
            instance_sha256: context.instance_sha256.into(),
            client_uid_sha256: context.client_uid_sha256.into(),
            request_id_sha256: request_id_sha256.clone(),
            provider: match request.provider() {
                blossom_core::ModelProviderKind::Ollama => "ollama",
                blossom_core::ModelProviderKind::LlamaCpp => "llama_cpp",
            }
            .into(),
            model_sha256: domain_digest(
                "model",
                context.instance_nonce,
                request.model().as_str().as_bytes(),
            ),
            deadline_ms: request.deadline_ms(),
        })
        .map_err(|_| GatewayProcessError::PrivateConnectionUnavailable)?;
    let started_at = Instant::now();

    let cancellation = InferenceCancellation::new();
    let cancellation_reader = cancellation.clone();
    let request_id = request.request_id().clone();
    let finished = Arc::new(AtomicBool::new(false));
    let reader_finished = finished.clone();
    let mut read_stream = stream
        .try_clone()
        .map_err(|_| GatewayProcessError::PrivateConnectionUnavailable)?;
    let cancellation_thread = std::thread::spawn(move || {
        loop {
            match reader.read_one(&mut read_stream) {
                Ok(frame) => {
                    let valid = decode_gateway_cancel(&frame)
                        .is_ok_and(|cancelled| cancelled == request_id);
                    cancellation_reader.cancel();
                    return valid;
                }
                Err(ReadFailure::TimedOut) if reader_finished.load(Ordering::Acquire) => {
                    return true;
                }
                Err(ReadFailure::TimedOut) => continue,
                Err(ReadFailure::Failed) if reader_finished.load(Ordering::Acquire) => return true,
                Err(ReadFailure::Failed) => {
                    cancellation_reader.cancel();
                    return false;
                }
            }
        }
    });

    let mut write_failed = false;
    let mut pending_terminal: Option<(Vec<u8>, GatewayAuditOutcome)> = None;
    let mut output_bytes = 0_usize;
    let mut proposed_intents = 0_usize;
    let mut usage = None;
    let inference_result = {
        let mut emit = |event: &blossom_core::NormalizedStreamEvent| {
            if write_failed {
                cancellation.cancel();
                return;
            }
            match &event.event {
                blossom_core::NormalizedStreamKind::TextDelta { content } => {
                    output_bytes = output_bytes.saturating_add(content.len());
                }
                blossom_core::NormalizedStreamKind::ToolIntents { intents } => {
                    proposed_intents = intents.len();
                }
                blossom_core::NormalizedStreamKind::Usage {
                    prompt_tokens,
                    generated_tokens,
                } => usage = Some((*prompt_tokens, *generated_tokens)),
                _ => {}
            }
            let terminal = match &event.event {
                blossom_core::NormalizedStreamKind::Finished { completion } => {
                    Some(match completion {
                        blossom_core::NormalizedCompletion::Text { .. } => {
                            GatewayAuditOutcome::CompletedText
                        }
                        blossom_core::NormalizedCompletion::ToolIntents { .. } => {
                            GatewayAuditOutcome::CompletedProposals
                        }
                    })
                }
                blossom_core::NormalizedStreamKind::Cancelled => {
                    Some(GatewayAuditOutcome::Cancelled)
                }
                blossom_core::NormalizedStreamKind::Failed { .. } => {
                    Some(GatewayAuditOutcome::ProviderFailed)
                }
                _ => None,
            };
            let encoded = encode_gateway_event(event)
                .map_err(|_| ())
                .and_then(|frame| {
                    if let Some(outcome) = terminal {
                        if pending_terminal.is_some() {
                            return Err(());
                        }
                        pending_terminal = Some((frame, outcome));
                        Ok(())
                    } else {
                        stream.write_all(&frame).map_err(|_| ())
                    }
                });
            if encoded.is_err() {
                write_failed = true;
                cancellation.cancel();
            }
        };
        inference(&request, cancellation.clone(), &mut emit)
    };
    finished.store(true, Ordering::Release);
    let _ = stream.shutdown(Shutdown::Read);
    let cancellation_valid = cancellation_thread.join().unwrap_or(false);
    let terminal_outcome = if !cancellation_valid {
        GatewayAuditOutcome::ProtocolRejected
    } else if write_failed {
        GatewayAuditOutcome::DeliveryFailed
    } else if inference_result.is_err() {
        GatewayAuditOutcome::ProviderFailed
    } else if let Some((_, outcome)) = pending_terminal.as_ref() {
        *outcome
    } else {
        GatewayAuditOutcome::ProviderFailed
    };
    audit
        .record(GatewayAuditEvent::RequestTerminal {
            instance_sha256: context.instance_sha256.into(),
            request_id_sha256,
            outcome: terminal_outcome,
            elapsed_ms: u64::try_from(started_at.elapsed().as_millis()).unwrap_or(u64::MAX),
            output_bytes,
            proposed_intents,
            prompt_tokens: usage.map(|value| value.0),
            generated_tokens: usage.map(|value| value.1),
        })
        .map_err(|_| GatewayProcessError::PrivateConnectionUnavailable)?;
    if write_failed || !cancellation_valid {
        return Err(GatewayProcessError::PrivateConnectionUnavailable);
    }
    inference_result?;
    let (terminal, _) =
        pending_terminal.ok_or(GatewayProcessError::PrivateConnectionUnavailable)?;
    stream
        .write_all(&terminal)
        .map_err(|_| GatewayProcessError::PrivateConnectionUnavailable)?;
    stream
        .flush()
        .map_err(|_| GatewayProcessError::PrivateConnectionUnavailable)?;
    stream
        .shutdown(Shutdown::Write)
        .map_err(|_| GatewayProcessError::PrivateConnectionUnavailable)
}

impl std::error::Error for GatewayProcessError {}

#[cfg(all(target_os = "linux", feature = "production-private-inference"))]
struct ProductionSocket {
    listener: std::os::unix::net::UnixListener,
    path: std::path::PathBuf,
    device: u64,
    inode: u64,
}

#[cfg(all(target_os = "linux", feature = "production-private-inference"))]
impl Drop for ProductionSocket {
    fn drop(&mut self) {
        use std::os::unix::fs::{FileTypeExt, MetadataExt};

        let Ok(metadata) = std::fs::symlink_metadata(&self.path) else {
            return;
        };
        if metadata.file_type().is_socket()
            && metadata.dev() == self.device
            && metadata.ino() == self.inode
        {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

#[cfg(all(target_os = "linux", feature = "production-private-inference"))]
fn bind_production_socket(
    path: &std::path::Path,
    owner_uid: u32,
    access_gid: u32,
) -> Result<ProductionSocket, GatewayProcessError> {
    use nix::unistd::{Gid, chown};
    use std::fs;
    use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
    use std::os::unix::net::UnixListener;

    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        _ => return Err(GatewayProcessError::PrivateConnectionUnavailable),
    }
    let listener =
        UnixListener::bind(path).map_err(|_| GatewayProcessError::PrivateConnectionUnavailable)?;
    let initial = fs::symlink_metadata(path)
        .map_err(|_| GatewayProcessError::PrivateConnectionUnavailable)?;
    if !initial.file_type().is_socket() {
        return Err(GatewayProcessError::PrivateConnectionUnavailable);
    }
    let socket = ProductionSocket {
        listener,
        path: path.to_path_buf(),
        device: initial.dev(),
        inode: initial.ino(),
    };
    // Avoid requiring CAP_CHOWN when the socket already inherited the exact
    // code-owned access group. Production still changes the group when the
    // runtime directory's group differs, and the final descriptor-bound
    // metadata check remains authoritative in either case.
    if initial.gid() != access_gid {
        chown(path, None, Some(Gid::from_raw(access_gid)))
            .map_err(|_| GatewayProcessError::PrivateConnectionUnavailable)?;
    }
    fs::set_permissions(path, fs::Permissions::from_mode(0o660))
        .map_err(|_| GatewayProcessError::PrivateConnectionUnavailable)?;
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| GatewayProcessError::PrivateConnectionUnavailable)?;
    if !metadata.file_type().is_socket()
        || metadata.uid() != owner_uid
        || metadata.gid() != access_gid
        || metadata.mode() & 0o7777 != 0o660
    {
        return Err(GatewayProcessError::PrivateConnectionUnavailable);
    }
    if metadata.dev() != socket.device || metadata.ino() != socket.inode {
        return Err(GatewayProcessError::PrivateConnectionUnavailable);
    }
    Ok(socket)
}

#[cfg(all(target_os = "linux", feature = "production-private-inference"))]
fn process_identity() -> Result<(String, String), GatewayProcessError> {
    use sha2::{Digest, Sha256};
    use std::fs::File;
    use std::io::Read;
    use std::os::unix::fs::MetadataExt;

    let mut file = File::open("/proc/sys/kernel/random/boot_id")
        .map_err(|_| GatewayProcessError::ProfileRegistryUnavailable)?;
    let metadata_before = file
        .metadata()
        .map_err(|_| GatewayProcessError::ProfileRegistryUnavailable)?;
    if !metadata_before.is_file() {
        return Err(GatewayProcessError::ProfileRegistryUnavailable);
    }
    let mut boot_id = Vec::new();
    file.by_ref()
        .take(129)
        .read_to_end(&mut boot_id)
        .map_err(|_| GatewayProcessError::ProfileRegistryUnavailable)?;
    let metadata_after = file
        .metadata()
        .map_err(|_| GatewayProcessError::ProfileRegistryUnavailable)?;
    let text = std::str::from_utf8(&boot_id)
        .map_err(|_| GatewayProcessError::ProfileRegistryUnavailable)?
        .trim_end_matches('\n');
    if boot_id.len() > 128
        || text.len() != 36
        || !text.bytes().enumerate().all(|(index, byte)| {
            matches!(index, 8 | 13 | 18 | 23) && byte == b'-'
                || !matches!(index, 8 | 13 | 18 | 23) && byte.is_ascii_hexdigit()
        })
        || metadata_before.dev() != metadata_after.dev()
        || metadata_before.ino() != metadata_after.ino()
    {
        return Err(GatewayProcessError::ProfileRegistryUnavailable);
    }
    let boot_digest = Sha256::digest(&boot_id)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let mut nonce = [0_u8; 32];
    getrandom::fill(&mut nonce).map_err(|_| GatewayProcessError::ProfileRegistryUnavailable)?;
    let instance_nonce = nonce.iter().map(|byte| format!("{byte:02x}")).collect();
    Ok((boot_digest, instance_nonce))
}

/// Start the sole fixed production listener after all installed-runtime and
/// service-identity checks pass. Each connection is authorized from kernel
/// credentials before the hello or any request bytes are read.
pub fn run_production() -> Result<(), GatewayProcessError> {
    #[cfg(all(target_os = "linux", feature = "production-private-inference"))]
    {
        use audit::{
            FileGatewayAudit, GatewayAdmissionOutcome, GatewayAudit, GatewayAuditEvent,
            domain_digest,
        };
        use blossom_core::{
            GatewayPeerCredentials, GatewayProfile, LlamaCppAdapter, ModelProfile,
            ModelProviderKind, OllamaAdapter, load_installed_runtime_readiness_from_set,
            production_provider_profile,
        };
        use std::path::Path;

        let specifications = [
            production_provider_profile(GatewayProfile::LlamaCppCpuV1)
                .map_err(|_| GatewayProcessError::ProfileRegistryUnavailable)?
                .ok_or(GatewayProcessError::ProfileRegistryUnavailable)?,
            production_provider_profile(GatewayProfile::OllamaCpuV1)
                .map_err(|_| GatewayProcessError::ProfileRegistryUnavailable)?
                .ok_or(GatewayProcessError::ProfileRegistryUnavailable)?,
        ];
        let readiness = load_installed_runtime_readiness_from_set(
            Path::new(PRODUCTION_PROFILE_PATH),
            &specifications,
        )
        .map_err(|_| GatewayProcessError::ProfileRegistryUnavailable)?;
        let effective_uid = nix::unistd::geteuid().as_raw();
        let effective_gid = nix::unistd::getegid().as_raw();
        if effective_uid != readiness.accounts().gateway_uid()
            || effective_gid != readiness.accounts().gateway_gid()
        {
            return Err(GatewayProcessError::ProfileRegistryUnavailable);
        }
        let model = ModelProfile::parse(readiness.profile().manifest().logical_model().to_owned())
            .map_err(|_| GatewayProcessError::ProfileRegistryUnavailable)?;
        let active_profile = readiness.profile().manifest().profile();
        let active_provider = readiness.profile().manifest().provider();
        let (boot_id_sha256, instance_nonce) = process_identity()?;
        let instance_sha256 = domain_digest("gateway_instance", &instance_nonce, b"instance");
        let mut audit = FileGatewayAudit::create(
            Path::new(PRODUCTION_AUDIT_PATH),
            readiness.accounts().gateway_uid(),
        )
        .map_err(|_| GatewayProcessError::PrivateConnectionUnavailable)?;
        audit
            .record(GatewayAuditEvent::ProcessStarted {
                boot_id_sha256: boot_id_sha256.clone(),
                instance_sha256: instance_sha256.clone(),
                profile_sha256: readiness.profile().manifest_sha256().into(),
            })
            .map_err(|_| GatewayProcessError::PrivateConnectionUnavailable)?;
        let socket = bind_production_socket(
            Path::new(PRODUCTION_SOCKET_PATH),
            readiness.accounts().gateway_uid(),
            readiness.accounts().access_gid(),
        )?;
        loop {
            let (stream, _) = socket
                .listener
                .accept()
                .map_err(|_| GatewayProcessError::PrivateConnectionUnavailable)?;
            let peer = match GatewayPeerCredentials::from_stream(&stream) {
                Ok(peer) => peer,
                Err(_) => {
                    audit
                        .record(GatewayAuditEvent::ClientAdmission {
                            instance_sha256: instance_sha256.clone(),
                            client_uid_sha256: None,
                            outcome: GatewayAdmissionOutcome::CredentialsUnavailable,
                        })
                        .map_err(|_| GatewayProcessError::PrivateConnectionUnavailable)?;
                    continue;
                }
            };
            let client_uid_sha256 =
                domain_digest("client_uid", &instance_nonce, &peer.uid.to_be_bytes());
            if readiness.authorize_client(peer).is_err() {
                audit
                    .record(GatewayAuditEvent::ClientAdmission {
                        instance_sha256: instance_sha256.clone(),
                        client_uid_sha256: Some(client_uid_sha256),
                        outcome: GatewayAdmissionOutcome::Ineligible,
                    })
                    .map_err(|_| GatewayProcessError::PrivateConnectionUnavailable)?;
                continue;
            }
            audit
                .record(GatewayAuditEvent::ClientAdmission {
                    instance_sha256: instance_sha256.clone(),
                    client_uid_sha256: Some(client_uid_sha256.clone()),
                    outcome: GatewayAdmissionOutcome::Authorized,
                })
                .map_err(|_| GatewayProcessError::PrivateConnectionUnavailable)?;
            let llama_cpp = LlamaCppAdapter::default();
            let ollama = OllamaAdapter::default();
            let _ = serve_authorized_private_connection(
                stream,
                PrivateConnectionContext {
                    profile: active_profile,
                    provider: active_provider.clone(),
                    model: model.clone(),
                    boot_id_sha256: &boot_id_sha256,
                    instance_nonce: &instance_nonce,
                    instance_sha256: &instance_sha256,
                    client_uid_sha256: &client_uid_sha256,
                },
                &mut audit,
                |request, cancellation, emit| match request.provider() {
                    ModelProviderKind::LlamaCpp => llama_cpp
                        .stream(request, cancellation, emit)
                        .map_err(|_| GatewayProcessError::PrivateConnectionUnavailable),
                    ModelProviderKind::Ollama => ollama
                        .stream(request, cancellation, emit)
                        .map_err(|_| GatewayProcessError::PrivateConnectionUnavailable),
                },
            );
        }
    }
    #[cfg(not(all(target_os = "linux", feature = "production-private-inference")))]
    {
        Err(GatewayProcessError::ProfileRegistryUnavailable)
    }
}

#[cfg(all(debug_assertions, unix))]
pub fn run_synthetic_fixture_from_environment() -> Result<(), GatewayProcessError> {
    use blossom_core::{GatewayProfile, serve_synthetic_gateway_once};
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::net::UnixListener;
    use std::path::PathBuf;

    const SOCKET: &str = "BLOSSOM_SYNTHETIC_GATEWAY_SOCKET";
    const CLIENT_UID: &str = "BLOSSOM_SYNTHETIC_GATEWAY_CLIENT_UID";
    const CLIENT_GID: &str = "BLOSSOM_SYNTHETIC_GATEWAY_CLIENT_GID";
    const PROFILE: &str = "BLOSSOM_SYNTHETIC_GATEWAY_PROFILE";

    let socket = std::env::var_os(SOCKET)
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .ok_or(GatewayProcessError::InvalidFixtureConfiguration)?;
    let uid = parse_id(CLIENT_UID)?;
    let gid = parse_id(CLIENT_GID)?;
    let profile = match std::env::var(PROFILE).ok().as_deref() {
        Some("ollama_cpu_v1") => GatewayProfile::OllamaCpuV1,
        Some("llama_cpp_cpu_v1") => GatewayProfile::LlamaCppCpuV1,
        _ => return Err(GatewayProcessError::InvalidFixtureConfiguration),
    };
    if socket == std::path::Path::new(PRODUCTION_SOCKET_PATH) || socket.exists() {
        return Err(GatewayProcessError::InvalidFixtureConfiguration);
    }

    let listener =
        UnixListener::bind(&socket).map_err(|_| GatewayProcessError::FixtureUnavailable)?;
    fs::set_permissions(&socket, fs::Permissions::from_mode(0o600))
        .map_err(|_| GatewayProcessError::FixtureUnavailable)?;
    let result = serve_synthetic_gateway_once(
        &listener,
        uid,
        gid,
        profile,
        &"a".repeat(64),
        "gateway-process-fixture-1",
        "synthetic-gateway-ok",
    )
    .map_err(|_| GatewayProcessError::FixtureUnavailable);
    drop(listener);
    let _ = fs::remove_file(&socket);
    result
}

#[cfg(all(debug_assertions, unix))]
fn parse_id(name: &str) -> Result<u32, GatewayProcessError> {
    let value = std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .filter(|value| *value != 0)
        .ok_or(GatewayProcessError::InvalidFixtureConfiguration)?;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn production_entry_point_is_fail_closed() {
        assert_eq!(
            run_production(),
            Err(GatewayProcessError::ProfileRegistryUnavailable)
        );
        assert_eq!(
            GatewayProcessError::ProfileRegistryUnavailable.exit_code(),
            78
        );
    }

    #[test]
    fn errors_are_content_free() {
        assert!(
            !GatewayProcessError::FixtureUnavailable
                .to_string()
                .contains(PRODUCTION_SOCKET_PATH)
        );
    }

    #[cfg(all(target_os = "linux", feature = "production-private-inference"))]
    #[test]
    fn production_socket_is_exact_rejects_stale_path_and_cleans_up() {
        use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};

        let directory = std::env::temp_dir().join(format!(
            "blossom-production-socket-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("unnamed")
        ));
        std::fs::create_dir(&directory).unwrap();
        let path = directory.join("inference.sock");
        let uid = nix::unistd::geteuid().as_raw();
        let gid = nix::unistd::getegid().as_raw();
        let socket = bind_production_socket(&path, uid, gid).unwrap();
        let metadata = std::fs::symlink_metadata(&path).unwrap();
        assert!(metadata.file_type().is_socket());
        assert_eq!(metadata.uid(), uid);
        assert_eq!(metadata.gid(), gid);
        assert_eq!(metadata.permissions().mode() & 0o7777, 0o660);
        assert!(bind_production_socket(&path, uid, gid).is_err());
        drop(socket);
        assert!(!path.exists());
        std::fs::remove_dir(&directory).unwrap();
    }

    #[cfg(all(target_os = "linux", feature = "production-private-inference"))]
    #[test]
    fn process_identity_is_boot_bound_and_nonce_is_fresh() {
        let (first_boot, first_nonce) = process_identity().unwrap();
        let (second_boot, second_nonce) = process_identity().unwrap();
        assert_eq!(first_boot.len(), 64);
        assert_eq!(first_boot, second_boot);
        assert_eq!(first_nonce.len(), 64);
        assert_eq!(second_nonce.len(), 64);
        assert_ne!(first_nonce, second_nonce);
    }

    #[cfg(unix)]
    mod private_connection {
        use super::*;
        use blossom_core::{
            ConversationMessage, ConversationRole, GATEWAY_PROTOCOL_VERSION, GatewayFrame,
            GatewayFrameDecoder, GatewayMessageKind, GatewayProfile, InferenceOutputMode,
            InferenceRequestId, ModelProfile, ModelProviderKind, ModelStreamState,
            NormalizedStreamKind, ProviderStreamInput, TurnIntentCatalogue, decode_gateway_event,
            decode_gateway_hello, encode_gateway_cancel, encode_gateway_private_request,
        };
        use std::collections::VecDeque;
        use std::io::{Read, Write};
        use std::os::unix::net::UnixStream;
        use std::sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        };
        use std::time::{Duration, Instant};

        #[derive(Default)]
        struct TestAudit(Vec<audit::GatewayAuditEvent>);

        impl audit::GatewayAudit for TestAudit {
            fn record(
                &mut self,
                event: audit::GatewayAuditEvent,
            ) -> Result<(), audit::GatewayAuditError> {
                self.0.push(event);
                Ok(())
            }
        }

        struct FailingAudit {
            calls: usize,
            fail_at: usize,
        }

        impl audit::GatewayAudit for FailingAudit {
            fn record(
                &mut self,
                _: audit::GatewayAuditEvent,
            ) -> Result<(), audit::GatewayAuditError> {
                self.calls += 1;
                if self.calls == self.fail_at {
                    Err(audit::GatewayAuditError)
                } else {
                    Ok(())
                }
            }
        }

        struct ClientReader {
            decoder: GatewayFrameDecoder,
            pending: VecDeque<GatewayFrame>,
        }

        impl ClientReader {
            fn new() -> Self {
                Self {
                    decoder: GatewayFrameDecoder::default(),
                    pending: VecDeque::new(),
                }
            }

            fn read_one(&mut self, stream: &mut UnixStream) -> GatewayFrame {
                if let Some(frame) = self.pending.pop_front() {
                    return frame;
                }
                let mut bytes = [0_u8; 4096];
                loop {
                    let count = stream.read(&mut bytes).unwrap();
                    assert_ne!(count, 0);
                    self.pending
                        .extend(self.decoder.push(&bytes[..count]).unwrap());
                    if let Some(frame) = self.pending.pop_front() {
                        return frame;
                    }
                }
            }
        }

        fn private_frame(id: &InferenceRequestId) -> Vec<u8> {
            encode_gateway_private_request(
                id,
                &[
                    ConversationMessage::new(ConversationRole::User, "private fixture".into())
                        .unwrap(),
                ],
                &TurnIntentCatalogue::empty(),
                InferenceOutputMode::Text,
                2_000,
            )
            .unwrap()
        }

        #[test]
        fn one_private_request_streams_one_validated_terminal_result() {
            let (mut client, server) = UnixStream::pair().unwrap();
            client
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let server_thread = std::thread::spawn(move || {
                let mut audit = TestAudit::default();
                serve_authorized_private_connection(
                    server,
                    PrivateConnectionContext {
                        profile: GatewayProfile::LlamaCppCpuV1,
                        provider: ModelProviderKind::LlamaCpp,
                        model: ModelProfile::parse("fixture-model:1".into()).unwrap(),
                        boot_id_sha256: &"a".repeat(64),
                        instance_nonce: "private-connection-1",
                        instance_sha256: &"d".repeat(64),
                        client_uid_sha256: &"e".repeat(64),
                    },
                    &mut audit,
                    |request, cancellation, emit| {
                        let mut state = ModelStreamState::new(request, cancellation);
                        emit(&state.apply(0, ProviderStreamInput::Started).unwrap());
                        emit(
                            &state
                                .apply(1, ProviderStreamInput::TextDelta("ok".into()))
                                .unwrap(),
                        );
                        emit(&state.apply(2, ProviderStreamInput::Finished).unwrap());
                        Ok(())
                    },
                )
            });
            let mut reader = ClientReader::new();
            let hello = reader.read_one(&mut client);
            assert_eq!(hello.kind(), GatewayMessageKind::Hello);
            assert_eq!(
                decode_gateway_hello(&hello, GatewayProfile::LlamaCppCpuV1)
                    .unwrap()
                    .0,
                "a".repeat(64)
            );
            let request_id = InferenceRequestId::parse("private-connection-1".into()).unwrap();
            client.write_all(&private_frame(&request_id)).unwrap();
            let mut terminal = false;
            while !terminal {
                let event = decode_gateway_event(&reader.read_one(&mut client)).unwrap();
                terminal = matches!(event.event, NormalizedStreamKind::Finished { .. });
            }
            assert_eq!(server_thread.join().unwrap(), Ok(()));
        }

        #[test]
        fn matching_cancel_wins_before_completion() {
            let (mut client, server) = UnixStream::pair().unwrap();
            client
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let server_thread = std::thread::spawn(move || {
                let mut audit = TestAudit::default();
                serve_authorized_private_connection(
                    server,
                    PrivateConnectionContext {
                        profile: GatewayProfile::LlamaCppCpuV1,
                        provider: ModelProviderKind::LlamaCpp,
                        model: ModelProfile::parse("fixture-model:1".into()).unwrap(),
                        boot_id_sha256: &"b".repeat(64),
                        instance_nonce: "private-cancel-1",
                        instance_sha256: &"d".repeat(64),
                        client_uid_sha256: &"e".repeat(64),
                    },
                    &mut audit,
                    |request, cancellation, emit| {
                        let mut state = ModelStreamState::new(request, cancellation.clone());
                        emit(&state.apply(0, ProviderStreamInput::Started).unwrap());
                        let deadline = Instant::now() + Duration::from_secs(2);
                        while !cancellation.is_cancelled() {
                            assert!(Instant::now() < deadline);
                            std::thread::yield_now();
                        }
                        emit(&state.apply(1, ProviderStreamInput::Finished).unwrap());
                        Ok(())
                    },
                )
            });
            let mut reader = ClientReader::new();
            let _ = reader.read_one(&mut client);
            let request_id = InferenceRequestId::parse("private-cancel-1".into()).unwrap();
            client.write_all(&private_frame(&request_id)).unwrap();
            let started = decode_gateway_event(&reader.read_one(&mut client)).unwrap();
            assert!(matches!(started.event, NormalizedStreamKind::Started));
            client
                .write_all(&encode_gateway_cancel(&request_id).unwrap())
                .unwrap();
            let cancelled = decode_gateway_event(&reader.read_one(&mut client)).unwrap();
            assert!(matches!(cancelled.event, NormalizedStreamKind::Cancelled));
            assert_eq!(server_thread.join().unwrap(), Ok(()));
        }

        #[test]
        fn pipelined_second_frame_starts_no_inference() {
            let (mut client, server) = UnixStream::pair().unwrap();
            client
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let called = Arc::new(AtomicBool::new(false));
            let server_called = called.clone();
            let server_thread = std::thread::spawn(move || {
                let mut audit = TestAudit::default();
                serve_authorized_private_connection(
                    server,
                    PrivateConnectionContext {
                        profile: GatewayProfile::LlamaCppCpuV1,
                        provider: ModelProviderKind::LlamaCpp,
                        model: ModelProfile::parse("fixture-model:1".into()).unwrap(),
                        boot_id_sha256: &"c".repeat(64),
                        instance_nonce: "private-pipeline-1",
                        instance_sha256: &"d".repeat(64),
                        client_uid_sha256: &"e".repeat(64),
                    },
                    &mut audit,
                    move |_, _, _| {
                        server_called.store(true, Ordering::Release);
                        Ok(())
                    },
                )
            });
            let mut reader = ClientReader::new();
            let _ = reader.read_one(&mut client);
            let request_id = InferenceRequestId::parse("private-pipeline-1".into()).unwrap();
            let frame = private_frame(&request_id);
            let mut pipelined = frame.clone();
            pipelined.extend_from_slice(&frame);
            client.write_all(&pipelined).unwrap();
            assert_eq!(
                server_thread.join().unwrap(),
                Err(GatewayProcessError::PrivateConnectionUnavailable)
            );
            assert!(!called.load(Ordering::Acquire));
            assert_eq!(GATEWAY_PROTOCOL_VERSION, 1);
        }

        #[test]
        fn audit_failure_before_inference_starts_nothing() {
            let (mut client, server) = UnixStream::pair().unwrap();
            let called = Arc::new(AtomicBool::new(false));
            let server_called = called.clone();
            let server_thread = std::thread::spawn(move || {
                let mut audit = FailingAudit {
                    calls: 0,
                    fail_at: 1,
                };
                serve_authorized_private_connection(
                    server,
                    PrivateConnectionContext {
                        profile: GatewayProfile::LlamaCppCpuV1,
                        provider: ModelProviderKind::LlamaCpp,
                        model: ModelProfile::parse("fixture-model:1".into()).unwrap(),
                        boot_id_sha256: &"a".repeat(64),
                        instance_nonce: "private-audit-failure-1",
                        instance_sha256: &"d".repeat(64),
                        client_uid_sha256: &"e".repeat(64),
                    },
                    &mut audit,
                    move |_, _, _| {
                        server_called.store(true, Ordering::Release);
                        Ok(())
                    },
                )
            });
            let mut reader = ClientReader::new();
            let _ = reader.read_one(&mut client);
            let request_id = InferenceRequestId::parse("private-audit-failure-1".into()).unwrap();
            client.write_all(&private_frame(&request_id)).unwrap();
            assert_eq!(
                server_thread.join().unwrap(),
                Err(GatewayProcessError::PrivateConnectionUnavailable)
            );
            assert!(!called.load(Ordering::Acquire));
        }

        #[test]
        fn terminal_audit_failure_withholds_success_terminal() {
            let (mut client, server) = UnixStream::pair().unwrap();
            client
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let server_thread = std::thread::spawn(move || {
                let mut audit = FailingAudit {
                    calls: 0,
                    fail_at: 2,
                };
                serve_authorized_private_connection(
                    server,
                    PrivateConnectionContext {
                        profile: GatewayProfile::LlamaCppCpuV1,
                        provider: ModelProviderKind::LlamaCpp,
                        model: ModelProfile::parse("fixture-model:1".into()).unwrap(),
                        boot_id_sha256: &"a".repeat(64),
                        instance_nonce: "private-terminal-audit-1",
                        instance_sha256: &"d".repeat(64),
                        client_uid_sha256: &"e".repeat(64),
                    },
                    &mut audit,
                    |request, cancellation, emit| {
                        let mut state = ModelStreamState::new(request, cancellation);
                        emit(&state.apply(0, ProviderStreamInput::Started).unwrap());
                        emit(
                            &state
                                .apply(1, ProviderStreamInput::TextDelta("ok".into()))
                                .unwrap(),
                        );
                        emit(&state.apply(2, ProviderStreamInput::Finished).unwrap());
                        Ok(())
                    },
                )
            });
            let mut reader = ClientReader::new();
            let _ = reader.read_one(&mut client);
            let request_id = InferenceRequestId::parse("private-terminal-audit-1".into()).unwrap();
            client.write_all(&private_frame(&request_id)).unwrap();
            let started = decode_gateway_event(&reader.read_one(&mut client)).unwrap();
            assert!(matches!(started.event, NormalizedStreamKind::Started));
            let delta = decode_gateway_event(&reader.read_one(&mut client)).unwrap();
            assert!(matches!(
                delta.event,
                NormalizedStreamKind::TextDelta { .. }
            ));
            assert_eq!(
                server_thread.join().unwrap(),
                Err(GatewayProcessError::PrivateConnectionUnavailable)
            );
            let mut trailing = [0_u8; 1];
            assert_eq!(client.read(&mut trailing).unwrap(), 0);
        }
    }
}
