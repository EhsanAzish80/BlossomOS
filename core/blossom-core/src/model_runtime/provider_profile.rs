//! Closed provider-profile manifest validation for the future packaged gateway.
//!
//! A manifest is descriptive data, not authority. Production code can load one
//! only against a code-owned `ProviderProfileSpec`; callers cannot compile a
//! specification or choose provider isolation settings.

use super::{
    GATEWAY_PROTOCOL_VERSION, GatewayProfile, LLAMA_CPP_ENDPOINT, MODEL_PROTOCOL_VERSION,
    ModelProviderKind, OLLAMA_ENDPOINT,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{File, Metadata};
use std::io::Read;
use std::path::{Component, Path, PathBuf};

pub const MAX_PROVIDER_MANIFEST_BYTES: usize = 32 * 1024;
const PROVIDER_PROFILE_VERSION: u16 = 4;
const MAX_PATH_BYTES: usize = 4096;
const MAX_ARGUMENTS: usize = 64;
const MAX_ARGUMENT_BYTES: usize = 4096;
const MAX_ENVIRONMENT_NAMES: usize = 32;
const MAX_FILESYSTEM_PATHS: usize = 16;
const MAX_MODEL_FILES: usize = 4_096;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderArtifact {
    path: PathBuf,
    sha256: String,
    bytes: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderServiceIdentity {
    gateway_user: String,
    gateway_group: String,
    provider_user: String,
    provider_group: String,
    access_group: String,
    gateway_unit: String,
    provider_unit: String,
    namespace_unit: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderFilesystemPolicy {
    read_only_paths: Vec<PathBuf>,
    writable_paths: Vec<PathBuf>,
    devices: Vec<PathBuf>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderProfileResources {
    memory_max_bytes: u64,
    memory_swap_max_bytes: u64,
    cpu_quota_percent: u16,
    tasks_max: u32,
    open_files_max: u32,
    file_size_max_bytes: u64,
    output_max_bytes: u32,
    request_deadline_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderProfileManifest {
    profile_version: u16,
    profile: GatewayProfile,
    provider: ModelProviderKind,
    gateway_protocol_version: u16,
    model_protocol_version: u16,
    binary: ProviderArtifact,
    runtime_mount: PathBuf,
    runtime_files: Vec<ProviderArtifact>,
    runtime_set_sha256: String,
    model_mount: PathBuf,
    model_files: Vec<ProviderArtifact>,
    model_set_sha256: String,
    unit_sha256: String,
    executable_arguments: Vec<String>,
    environment_names: Vec<String>,
    endpoint: String,
    inference_path: String,
    filesystem: ProviderFilesystemPolicy,
    resources: ProviderProfileResources,
    identity: ProviderServiceIdentity,
}

/// An opaque, code-owned expected manifest and its canonical representation.
#[derive(Clone, Debug)]
pub struct ProviderProfileSpec {
    expected: ProviderProfileManifest,
    canonical_bytes: Vec<u8>,
    sha256: String,
}

impl ProviderProfileSpec {
    fn compile(expected: ProviderProfileManifest) -> Result<Self, ProviderProfileError> {
        validate_manifest(&expected)?;
        let canonical_bytes =
            serde_json::to_vec(&expected).map_err(|_| ProviderProfileError::InvalidManifest)?;
        if canonical_bytes.len() > MAX_PROVIDER_MANIFEST_BYTES {
            return Err(ProviderProfileError::ManifestTooLarge);
        }
        let sha256 = hex_digest(&canonical_bytes);
        Ok(Self {
            expected,
            canonical_bytes,
            sha256,
        })
    }

    fn from_embedded(bytes: &[u8]) -> Result<Self, ProviderProfileError> {
        if bytes.len() > MAX_PROVIDER_MANIFEST_BYTES {
            return Err(ProviderProfileError::ManifestTooLarge);
        }
        let expected: ProviderProfileManifest =
            serde_json::from_slice(bytes).map_err(|_| ProviderProfileError::InvalidManifest)?;
        let specification = Self::compile(expected)?;
        if specification.canonical_bytes != bytes {
            return Err(ProviderProfileError::InvalidManifest);
        }
        Ok(specification)
    }

    pub fn manifest(&self) -> &ProviderProfileManifest {
        &self.expected
    }

    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }

    pub fn sha256(&self) -> &str {
        &self.sha256
    }
}

/// Return the sole release-compiled production profile currently packaged.
///
/// This constructs authority only from repository-owned bytes embedded at
/// compile time. It does not read a caller-selected registry or start a
/// provider. Ollama remains unavailable until its deterministic store package
/// is independently pinned and reviewed.
pub fn production_provider_profile(
    profile: GatewayProfile,
) -> Result<Option<ProviderProfileSpec>, ProviderProfileError> {
    const LLAMA_CPP_X86_64: &[u8] = include_bytes!(
        "../../../../system/model-runtime/registry/llama-cpp-cpu-x86_64.profile.json"
    );
    match profile {
        GatewayProfile::LlamaCppCpuV1 if cfg!(target_arch = "x86_64") => {
            let bytes = LLAMA_CPP_X86_64
                .strip_suffix(b"\n")
                .unwrap_or(LLAMA_CPP_X86_64);
            ProviderProfileSpec::from_embedded(bytes).map(Some)
        }
        GatewayProfile::LlamaCppCpuV1 | GatewayProfile::OllamaCpuV1 => Ok(None),
    }
}

/// A deterministic, developer-authored package fixture. It is excluded from
/// release builds and cannot carry a real artifact or private input.
#[cfg(any(test, debug_assertions))]
#[derive(Clone, Debug)]
pub struct SyntheticProviderPackage {
    profile: GatewayProfile,
    spec: ProviderProfileSpec,
    rendered_unit: Vec<u8>,
}

#[cfg(any(test, debug_assertions))]
impl SyntheticProviderPackage {
    pub fn profile(&self) -> GatewayProfile {
        self.profile
    }

    pub fn spec(&self) -> &ProviderProfileSpec {
        &self.spec
    }

    pub fn rendered_unit(&self) -> &[u8] {
        &self.rendered_unit
    }
}

/// Render one of the two closed provider templates with fixed synthetic
/// artifacts and resource values. No caller-controlled path, argument,
/// environment, identity, digest, or resource enters the result.
#[cfg(any(test, debug_assertions))]
pub fn fixed_synthetic_provider_package(
    profile: GatewayProfile,
) -> Result<SyntheticProviderPackage, ProviderProfileError> {
    const OLLAMA_TEMPLATE: &str =
        include_str!("../../../../system/model-runtime/packaging/blossom-model-ollama.service.in");
    const LLAMA_CPP_TEMPLATE: &str = include_str!(
        "../../../../system/model-runtime/packaging/blossom-model-llama-cpp.service.in"
    );

    let fixture = match profile {
        GatewayProfile::OllamaCpuV1 => SyntheticPackageFields {
            template: OLLAMA_TEMPLATE,
            binary: "/usr/lib/blossom-os/providers/ollama/ollama",
            runtime_mount: "/usr/lib/blossom-os/providers/ollama",
            model: "/usr/lib/blossom-os/models/ollama",
            model_mount: "/usr/lib/blossom-os/models/ollama",
            model_files: vec![
                "/usr/lib/blossom-os/models/ollama/blobs/sha256-fixture",
                "/usr/lib/blossom-os/models/ollama/manifests/fixture-model",
            ],
            model_directory: Some("/usr/lib/blossom-os/models/ollama"),
            writable: "/var/lib/blossom/model-provider/ollama",
            arguments: vec![
                "/usr/lib/blossom-os/providers/ollama/ollama".into(),
                "serve".into(),
            ],
            environment_names: vec!["HOME".into(), "OLLAMA_HOST".into(), "OLLAMA_MODELS".into()],
            endpoint: OLLAMA_ENDPOINT,
            inference_path: "/api/chat",
            provider_unit: "blossom-model-ollama.service",
        },
        GatewayProfile::LlamaCppCpuV1 => SyntheticPackageFields {
            template: LLAMA_CPP_TEMPLATE,
            binary: "/usr/lib/blossom-os/providers/llama-cpp/llama-server",
            runtime_mount: "/usr/lib/blossom-os/providers/llama-cpp",
            model: "/usr/lib/blossom-os/models/llama-cpp/evidence.gguf",
            model_mount: "/usr/lib/blossom-os/models/llama-cpp/evidence.gguf",
            model_files: vec!["/usr/lib/blossom-os/models/llama-cpp/evidence.gguf"],
            model_directory: None,
            writable: "/var/lib/blossom/model-provider/llama-cpp",
            arguments: vec![
                "/usr/lib/blossom-os/providers/llama-cpp/llama-server".into(),
                "--model".into(),
                "/usr/lib/blossom-os/models/llama-cpp/evidence.gguf".into(),
                "--no-webui".into(),
            ],
            environment_names: vec!["HOME".into()],
            endpoint: LLAMA_CPP_ENDPOINT,
            inference_path: "/v1/chat/completions",
            provider_unit: "blossom-model-llama-cpp.service",
        },
    };
    let rendered = render_synthetic_unit(&fixture)?;
    let mut manifest = ProviderProfileManifest {
        profile_version: PROVIDER_PROFILE_VERSION,
        profile,
        provider: profile.provider(),
        gateway_protocol_version: GATEWAY_PROTOCOL_VERSION,
        model_protocol_version: MODEL_PROTOCOL_VERSION,
        binary: ProviderArtifact {
            path: fixture.binary.into(),
            sha256: "a".repeat(64),
            bytes: 1,
        },
        runtime_mount: fixture.runtime_mount.into(),
        runtime_files: vec![ProviderArtifact {
            path: fixture.binary.into(),
            sha256: "a".repeat(64),
            bytes: 1,
        }],
        runtime_set_sha256: String::new(),
        model_mount: fixture.model_mount.into(),
        model_files: fixture
            .model_files
            .iter()
            .map(|path| ProviderArtifact {
                path: (*path).into(),
                sha256: "b".repeat(64),
                bytes: 1,
            })
            .collect(),
        model_set_sha256: String::new(),
        unit_sha256: hex_digest(&rendered),
        executable_arguments: fixture.arguments,
        environment_names: fixture.environment_names,
        endpoint: fixture.endpoint.into(),
        inference_path: fixture.inference_path.into(),
        filesystem: ProviderFilesystemPolicy {
            read_only_paths: vec![fixture.runtime_mount.into(), fixture.model_mount.into()],
            writable_paths: vec![fixture.writable.into()],
            devices: vec![],
        },
        resources: ProviderProfileResources {
            memory_max_bytes: 4 * 1024 * 1024 * 1024,
            memory_swap_max_bytes: 0,
            cpu_quota_percent: 200,
            tasks_max: 64,
            open_files_max: 256,
            file_size_max_bytes: 1024 * 1024,
            output_max_bytes: super::MAX_OUTPUT_BYTES as u32,
            request_deadline_ms: super::MAX_DEADLINE_MS,
        },
        identity: ProviderServiceIdentity {
            gateway_user: "blossom-model-gateway".into(),
            gateway_group: "blossom-model-gateway".into(),
            provider_user: "blossom-model-provider".into(),
            provider_group: "blossom-model-provider".into(),
            access_group: "blossom-ai".into(),
            gateway_unit: "blossom-model-gateway.service".into(),
            provider_unit: fixture.provider_unit.into(),
            namespace_unit: "blossom-model-netns.service".into(),
        },
    };
    manifest.runtime_set_sha256 = artifact_set_digest(&manifest.runtime_files)?;
    manifest.model_set_sha256 = artifact_set_digest(&manifest.model_files)?;
    Ok(SyntheticProviderPackage {
        profile,
        spec: ProviderProfileSpec::compile(manifest)?,
        rendered_unit: rendered,
    })
}

#[cfg(any(test, debug_assertions))]
struct SyntheticPackageFields {
    template: &'static str,
    binary: &'static str,
    runtime_mount: &'static str,
    model: &'static str,
    model_mount: &'static str,
    model_files: Vec<&'static str>,
    model_directory: Option<&'static str>,
    writable: &'static str,
    arguments: Vec<String>,
    environment_names: Vec<String>,
    endpoint: &'static str,
    inference_path: &'static str,
    provider_unit: &'static str,
}

#[cfg(any(test, debug_assertions))]
fn render_synthetic_unit(
    fixture: &SyntheticPackageFields,
) -> Result<Vec<u8>, ProviderProfileError> {
    let replacements = [
        ("@PROVIDER_BINARY@", fixture.binary),
        ("@PROVIDER_DIRECTORY@", fixture.runtime_mount),
        ("@MODEL_PATH@", fixture.model),
        ("@TASKS_MAX@", "64"),
        ("@MEMORY_MAX@", "4G"),
        ("@MEMORY_SWAP_MAX@", "0"),
        ("@CPU_QUOTA@", "200%"),
        ("@FILE_SIZE_MAX@", "1M"),
        ("@OPEN_FILES_MAX@", "256"),
    ];
    let mut rendered = fixture.template.to_owned();
    for (token, value) in replacements {
        if !rendered.contains(token) {
            return Err(ProviderProfileError::InvalidManifest);
        }
        rendered = rendered.replace(token, value);
    }
    if let Some(model_directory) = fixture.model_directory {
        if !rendered.contains("@MODEL_DIRECTORY@") {
            return Err(ProviderProfileError::InvalidManifest);
        }
        rendered = rendered.replace("@MODEL_DIRECTORY@", model_directory);
    }
    if rendered.contains("@PROVIDER_")
        || rendered.contains("@MODEL_")
        || rendered.contains("@TASKS_")
        || rendered.contains("@MEMORY_")
        || rendered.contains("@CPU_")
        || rendered.contains("@FILE_")
        || rendered.contains("@OPEN_")
    {
        return Err(ProviderProfileError::InvalidManifest);
    }
    Ok(rendered.into_bytes())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidatedProviderProfile {
    manifest: ProviderProfileManifest,
    manifest_sha256: String,
    source_bytes: u64,
    source_device: u64,
    source_inode: u64,
}

impl ValidatedProviderProfile {
    pub fn manifest(&self) -> &ProviderProfileManifest {
        &self.manifest
    }

    pub fn manifest_sha256(&self) -> &str {
        &self.manifest_sha256
    }

    pub fn source_bytes(&self) -> u64 {
        self.source_bytes
    }

    pub fn source_device(&self) -> u64 {
        self.source_device
    }

    pub fn source_inode(&self) -> u64 {
        self.source_inode
    }

    pub(crate) fn binary(&self) -> &ProviderArtifact {
        &self.manifest.binary
    }

    pub(crate) fn model_files(&self) -> &[ProviderArtifact] {
        &self.manifest.model_files
    }

    pub(crate) fn runtime_files(&self) -> &[ProviderArtifact] {
        &self.manifest.runtime_files
    }

    pub(crate) fn runtime_mount(&self) -> &Path {
        &self.manifest.runtime_mount
    }

    pub(crate) fn model_mount(&self) -> &Path {
        &self.manifest.model_mount
    }

    pub(crate) fn profile(&self) -> GatewayProfile {
        self.manifest.profile
    }

    pub(crate) fn identity(&self) -> &ProviderServiceIdentity {
        &self.manifest.identity
    }

    pub(crate) fn unit_sha256(&self) -> &str {
        &self.manifest.unit_sha256
    }
}

impl ProviderArtifact {
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn sha256(&self) -> &str {
        &self.sha256
    }

    pub(crate) fn bytes(&self) -> u64 {
        self.bytes
    }
}

impl ProviderServiceIdentity {
    pub(crate) fn gateway_user(&self) -> &str {
        &self.gateway_user
    }
    pub(crate) fn gateway_group(&self) -> &str {
        &self.gateway_group
    }
    pub(crate) fn provider_user(&self) -> &str {
        &self.provider_user
    }
    pub(crate) fn provider_group(&self) -> &str {
        &self.provider_group
    }
    pub(crate) fn access_group(&self) -> &str {
        &self.access_group
    }
    pub(crate) fn provider_unit(&self) -> &str {
        &self.provider_unit
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProviderProfileError {
    InvalidPath,
    OpenFailed,
    MetadataFailed,
    NotRegularFile,
    WrongOwner,
    UnsafePermissions,
    ManifestTooLarge,
    ReadFailed,
    SourceChanged,
    InvalidManifest,
    UnexpectedManifest,
}

/// Load a packaged manifest that must be owned by root and match a code-owned
/// canonical specification byte for byte.
pub fn load_installed_provider_profile(
    path: &Path,
    expected: &ProviderProfileSpec,
) -> Result<ValidatedProviderProfile, ProviderProfileError> {
    load_provider_profile(path, expected, 0)
}

fn load_provider_profile(
    path: &Path,
    expected: &ProviderProfileSpec,
    expected_uid: u32,
) -> Result<ValidatedProviderProfile, ProviderProfileError> {
    validate_absolute_path(path)?;
    let mut file = open_manifest(path)?;
    let before = file
        .metadata()
        .map_err(|_| ProviderProfileError::MetadataFailed)?;
    validate_metadata(&before, expected_uid)?;
    if before.len() as usize > MAX_PROVIDER_MANIFEST_BYTES {
        return Err(ProviderProfileError::ManifestTooLarge);
    }

    let mut bytes = Vec::with_capacity(before.len() as usize);
    file.by_ref()
        .take((MAX_PROVIDER_MANIFEST_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| ProviderProfileError::ReadFailed)?;
    if bytes.len() > MAX_PROVIDER_MANIFEST_BYTES {
        return Err(ProviderProfileError::ManifestTooLarge);
    }
    let after = file
        .metadata()
        .map_err(|_| ProviderProfileError::MetadataFailed)?;
    if !same_file_state(&before, &after) || bytes.len() as u64 != after.len() {
        return Err(ProviderProfileError::SourceChanged);
    }

    let parsed: ProviderProfileManifest =
        serde_json::from_slice(&bytes).map_err(|_| ProviderProfileError::InvalidManifest)?;
    validate_manifest(&parsed)?;
    let digest = hex_digest(&bytes);
    if bytes != expected.canonical_bytes || digest != expected.sha256 || parsed != expected.expected
    {
        return Err(ProviderProfileError::UnexpectedManifest);
    }

    Ok(ValidatedProviderProfile {
        manifest: parsed,
        manifest_sha256: digest,
        source_bytes: bytes.len() as u64,
        source_device: metadata_device(&after),
        source_inode: metadata_inode(&after),
    })
}

fn validate_manifest(manifest: &ProviderProfileManifest) -> Result<(), ProviderProfileError> {
    if manifest.profile_version != PROVIDER_PROFILE_VERSION
        || manifest.gateway_protocol_version != GATEWAY_PROTOCOL_VERSION
        || manifest.model_protocol_version != MODEL_PROTOCOL_VERSION
        || manifest.provider != manifest.profile.provider()
    {
        return Err(ProviderProfileError::InvalidManifest);
    }

    let (endpoint, inference_path, provider_unit) = match manifest.profile {
        GatewayProfile::OllamaCpuV1 => {
            (OLLAMA_ENDPOINT, "/api/chat", "blossom-model-ollama.service")
        }
        GatewayProfile::LlamaCppCpuV1 => (
            LLAMA_CPP_ENDPOINT,
            "/v1/chat/completions",
            "blossom-model-llama-cpp.service",
        ),
    };
    if manifest.endpoint != endpoint
        || manifest.inference_path != inference_path
        || manifest.identity.provider_unit != provider_unit
        || manifest.identity.gateway_unit != "blossom-model-gateway.service"
        || manifest.identity.namespace_unit != "blossom-model-netns.service"
    {
        return Err(ProviderProfileError::InvalidManifest);
    }

    validate_artifact(&manifest.binary)?;
    validate_runtime_set(manifest)?;
    validate_model_set(manifest)?;
    validate_digest(&manifest.unit_sha256)?;
    validate_arguments(manifest)?;
    validate_environment(&manifest.environment_names)?;
    validate_filesystem(manifest)?;
    validate_identity(&manifest.identity)?;
    validate_resources(&manifest.resources)?;
    Ok(())
}

fn validate_artifact(artifact: &ProviderArtifact) -> Result<(), ProviderProfileError> {
    validate_absolute_path(&artifact.path)?;
    validate_digest(&artifact.sha256)?;
    if artifact.bytes == 0 || artifact.bytes > 64 * 1024 * 1024 * 1024 {
        return Err(ProviderProfileError::InvalidManifest);
    }
    Ok(())
}

fn validate_arguments(manifest: &ProviderProfileManifest) -> Result<(), ProviderProfileError> {
    let arguments = &manifest.executable_arguments;
    if arguments.is_empty() || arguments.len() > MAX_ARGUMENTS {
        return Err(ProviderProfileError::InvalidManifest);
    }
    for argument in arguments {
        if argument.is_empty()
            || argument.len() > MAX_ARGUMENT_BYTES
            || argument.contains('\0')
            || argument.contains("http://")
            || argument.contains("https://")
            || ["--host", "--port", "--listen", "--address"]
                .iter()
                .any(|option| argument == option || argument.starts_with(&format!("{option}=")))
        {
            return Err(ProviderProfileError::InvalidManifest);
        }
    }
    let provider_arguments_valid = match manifest.profile {
        GatewayProfile::OllamaCpuV1 => arguments.len() == 2 && arguments[1] == "serve",
        GatewayProfile::LlamaCppCpuV1 => arguments
            .iter()
            .any(|argument| Path::new(argument) == manifest.model_mount),
    };
    if Path::new(&arguments[0]) != manifest.binary.path || !provider_arguments_valid {
        return Err(ProviderProfileError::InvalidManifest);
    }
    Ok(())
}

fn validate_environment(names: &[String]) -> Result<(), ProviderProfileError> {
    if names.len() > MAX_ENVIRONMENT_NAMES {
        return Err(ProviderProfileError::InvalidManifest);
    }
    let mut previous: Option<&str> = None;
    for name in names {
        if !matches!(
            name.as_str(),
            "HOME" | "LANG" | "LC_ALL" | "OLLAMA_HOST" | "OLLAMA_MODELS" | "OMP_NUM_THREADS" | "TZ"
        ) || name.is_empty()
            || name.len() > 128
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
            || name
                .bytes()
                .next()
                .is_some_and(|byte| byte.is_ascii_digit())
            || previous.is_some_and(|value| value >= name.as_str())
        {
            return Err(ProviderProfileError::InvalidManifest);
        }
        previous = Some(name);
    }
    Ok(())
}

fn validate_filesystem(manifest: &ProviderProfileManifest) -> Result<(), ProviderProfileError> {
    let filesystem = &manifest.filesystem;
    if filesystem.read_only_paths.len() != 2
        || filesystem.read_only_paths[0] != manifest.runtime_mount
        || filesystem.read_only_paths[1] != manifest.model_mount
        || filesystem.writable_paths.len() > MAX_FILESYSTEM_PATHS
        || !filesystem.devices.is_empty()
    {
        return Err(ProviderProfileError::InvalidManifest);
    }
    let mut all_paths = filesystem.read_only_paths.clone();
    all_paths.extend(filesystem.writable_paths.clone());
    for path in &all_paths {
        validate_absolute_path(path)?;
    }
    let mut previous: Option<&Path> = None;
    for writable in &filesystem.writable_paths {
        if !(writable.starts_with("/var/lib/blossom/")
            || writable.starts_with("/var/cache/blossom/"))
            || previous.is_some_and(|path| path >= writable.as_path())
        {
            return Err(ProviderProfileError::InvalidManifest);
        }
        previous = Some(writable);
    }
    for writable in &filesystem.writable_paths {
        if filesystem
            .read_only_paths
            .iter()
            .any(|read_only| writable.starts_with(read_only) || read_only.starts_with(writable))
        {
            return Err(ProviderProfileError::InvalidManifest);
        }
    }
    Ok(())
}

fn validate_model_set(manifest: &ProviderProfileManifest) -> Result<(), ProviderProfileError> {
    validate_absolute_path(&manifest.model_mount)?;
    if manifest.model_files.is_empty() || manifest.model_files.len() > MAX_MODEL_FILES {
        return Err(ProviderProfileError::InvalidManifest);
    }
    let mut previous: Option<&Path> = None;
    for artifact in &manifest.model_files {
        validate_artifact(artifact)?;
        if previous.is_some_and(|path| path >= artifact.path.as_path()) {
            return Err(ProviderProfileError::InvalidManifest);
        }
        match manifest.profile {
            GatewayProfile::OllamaCpuV1 => {
                if artifact.path == manifest.model_mount
                    || !artifact.path.starts_with(&manifest.model_mount)
                {
                    return Err(ProviderProfileError::InvalidManifest);
                }
            }
            GatewayProfile::LlamaCppCpuV1 => {
                if manifest.model_files.len() != 1 || artifact.path != manifest.model_mount {
                    return Err(ProviderProfileError::InvalidManifest);
                }
            }
        }
        previous = Some(&artifact.path);
    }
    validate_digest(&manifest.model_set_sha256)?;
    if artifact_set_digest(&manifest.model_files)? != manifest.model_set_sha256 {
        return Err(ProviderProfileError::InvalidManifest);
    }
    Ok(())
}

fn validate_runtime_set(manifest: &ProviderProfileManifest) -> Result<(), ProviderProfileError> {
    validate_absolute_path(&manifest.runtime_mount)?;
    if manifest.runtime_files.is_empty() || manifest.runtime_files.len() > MAX_MODEL_FILES {
        return Err(ProviderProfileError::InvalidManifest);
    }
    let mut previous: Option<&Path> = None;
    let mut contains_binary = false;
    for artifact in &manifest.runtime_files {
        validate_artifact(artifact)?;
        if artifact.path == manifest.runtime_mount
            || !artifact.path.starts_with(&manifest.runtime_mount)
            || previous.is_some_and(|path| path >= artifact.path.as_path())
        {
            return Err(ProviderProfileError::InvalidManifest);
        }
        contains_binary |= artifact == &manifest.binary;
        previous = Some(&artifact.path);
    }
    validate_digest(&manifest.runtime_set_sha256)?;
    if !contains_binary
        || artifact_set_digest(&manifest.runtime_files)? != manifest.runtime_set_sha256
    {
        return Err(ProviderProfileError::InvalidManifest);
    }
    Ok(())
}

fn artifact_set_digest(files: &[ProviderArtifact]) -> Result<String, ProviderProfileError> {
    let canonical = serde_json::to_vec(files).map_err(|_| ProviderProfileError::InvalidManifest)?;
    Ok(hex_digest(&canonical))
}

fn validate_identity(identity: &ProviderServiceIdentity) -> Result<(), ProviderProfileError> {
    if identity.gateway_user != "blossom-model-gateway"
        || identity.gateway_group != "blossom-model-gateway"
        || identity.provider_user != "blossom-model-provider"
        || identity.provider_group != "blossom-model-provider"
        || identity.access_group != "blossom-ai"
    {
        return Err(ProviderProfileError::InvalidManifest);
    }
    Ok(())
}

fn validate_resources(resources: &ProviderProfileResources) -> Result<(), ProviderProfileError> {
    if !(256 * 1024 * 1024..=256 * 1024 * 1024 * 1024).contains(&resources.memory_max_bytes)
        || resources.memory_swap_max_bytes > resources.memory_max_bytes
        || !(1..=1600).contains(&resources.cpu_quota_percent)
        || !(1..=4096).contains(&resources.tasks_max)
        || !(32..=65_536).contains(&resources.open_files_max)
        || !(1..=16 * 1024 * 1024).contains(&resources.file_size_max_bytes)
        || resources.output_max_bytes == 0
        || resources.output_max_bytes as usize > super::MAX_OUTPUT_BYTES
        || resources.request_deadline_ms == 0
        || resources.request_deadline_ms > super::MAX_DEADLINE_MS
    {
        return Err(ProviderProfileError::InvalidManifest);
    }
    Ok(())
}

fn validate_absolute_path(path: &Path) -> Result<(), ProviderProfileError> {
    let text = path.to_str().ok_or(ProviderProfileError::InvalidPath)?;
    if !path.is_absolute()
        || text.len() > MAX_PATH_BYTES
        || text.contains('\0')
        || path
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::CurDir))
    {
        return Err(ProviderProfileError::InvalidPath);
    }
    Ok(())
}

fn validate_digest(value: &str) -> Result<(), ProviderProfileError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(ProviderProfileError::InvalidManifest);
    }
    Ok(())
}

fn hex_digest(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write;
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

#[cfg(unix)]
fn validate_metadata(metadata: &Metadata, expected_uid: u32) -> Result<(), ProviderProfileError> {
    use std::os::unix::fs::MetadataExt;
    if !metadata.file_type().is_file() {
        return Err(ProviderProfileError::NotRegularFile);
    }
    if metadata.uid() != expected_uid {
        return Err(ProviderProfileError::WrongOwner);
    }
    if metadata.mode() & 0o022 != 0 {
        return Err(ProviderProfileError::UnsafePermissions);
    }
    Ok(())
}

#[cfg(not(unix))]
fn validate_metadata(_: &Metadata, _: u32) -> Result<(), ProviderProfileError> {
    Err(ProviderProfileError::MetadataFailed)
}

#[cfg(unix)]
fn same_file_state(before: &Metadata, after: &Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    before.dev() == after.dev()
        && before.ino() == after.ino()
        && before.len() == after.len()
        && before.mtime() == after.mtime()
        && before.mtime_nsec() == after.mtime_nsec()
}

#[cfg(not(unix))]
fn same_file_state(before: &Metadata, after: &Metadata) -> bool {
    before.len() == after.len() && before.modified().ok() == after.modified().ok()
}

#[cfg(unix)]
fn metadata_device(metadata: &Metadata) -> u64 {
    use std::os::unix::fs::MetadataExt;
    metadata.dev()
}

#[cfg(not(unix))]
fn metadata_device(_: &Metadata) -> u64 {
    0
}

#[cfg(unix)]
fn metadata_inode(metadata: &Metadata) -> u64 {
    use std::os::unix::fs::MetadataExt;
    metadata.ino()
}

#[cfg(not(unix))]
fn metadata_inode(_: &Metadata) -> u64 {
    0
}

#[cfg(target_os = "linux")]
fn open_manifest(path: &Path) -> Result<File, ProviderProfileError> {
    use nix::fcntl::{OFlag, OpenHow, ResolveFlag, openat2};

    let relative = path
        .strip_prefix("/")
        .map_err(|_| ProviderProfileError::InvalidPath)?;
    let root = File::open("/").map_err(|_| ProviderProfileError::OpenFailed)?;
    let how = OpenHow::new()
        .flags(OFlag::O_RDONLY | OFlag::O_CLOEXEC | OFlag::O_NOFOLLOW)
        .resolve(
            ResolveFlag::RESOLVE_BENEATH
                | ResolveFlag::RESOLVE_NO_MAGICLINKS
                | ResolveFlag::RESOLVE_NO_SYMLINKS,
        );
    openat2(root, relative, how)
        .map(File::from)
        .map_err(|_| ProviderProfileError::OpenFailed)
}

#[cfg(all(unix, not(target_os = "linux")))]
fn open_manifest(path: &Path) -> Result<File, ProviderProfileError> {
    use std::os::unix::fs::OpenOptionsExt;
    std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW)
        .open(path)
        .map_err(|_| ProviderProfileError::OpenFailed)
}

#[cfg(not(unix))]
fn open_manifest(_: &Path) -> Result<File, ProviderProfileError> {
    Err(ProviderProfileError::OpenFailed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::{PermissionsExt, symlink};
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEST_DIRECTORY: AtomicU64 = AtomicU64::new(1);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let id = NEXT_TEST_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "blossom-provider-profile-{}-{id}",
                std::process::id()
            ));
            fs::create_dir(&path).expect("create test directory");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn fixture() -> ProviderProfileManifest {
        let mut manifest = ProviderProfileManifest {
            profile_version: PROVIDER_PROFILE_VERSION,
            profile: GatewayProfile::LlamaCppCpuV1,
            provider: ModelProviderKind::LlamaCpp,
            gateway_protocol_version: GATEWAY_PROTOCOL_VERSION,
            model_protocol_version: MODEL_PROTOCOL_VERSION,
            binary: ProviderArtifact {
                path: "/usr/bin/llama-server".into(),
                sha256: "a".repeat(64),
                bytes: 1,
            },
            runtime_mount: "/usr/bin".into(),
            runtime_files: vec![ProviderArtifact {
                path: "/usr/bin/llama-server".into(),
                sha256: "a".repeat(64),
                bytes: 1,
            }],
            runtime_set_sha256: String::new(),
            model_mount: "/usr/lib/blossom/models/evidence.gguf".into(),
            model_files: vec![ProviderArtifact {
                path: "/usr/lib/blossom/models/evidence.gguf".into(),
                sha256: "b".repeat(64),
                bytes: 1,
            }],
            model_set_sha256: String::new(),
            unit_sha256: "c".repeat(64),
            executable_arguments: vec![
                "/usr/bin/llama-server".into(),
                "--model".into(),
                "/usr/lib/blossom/models/evidence.gguf".into(),
                "--no-webui".into(),
            ],
            environment_names: vec!["LANG".into()],
            endpoint: LLAMA_CPP_ENDPOINT.into(),
            inference_path: "/v1/chat/completions".into(),
            filesystem: ProviderFilesystemPolicy {
                read_only_paths: vec![
                    "/usr/bin".into(),
                    "/usr/lib/blossom/models/evidence.gguf".into(),
                ],
                writable_paths: vec!["/var/cache/blossom/model-provider".into()],
                devices: vec![],
            },
            resources: ProviderProfileResources {
                memory_max_bytes: 4 * 1024 * 1024 * 1024,
                memory_swap_max_bytes: 0,
                cpu_quota_percent: 200,
                tasks_max: 64,
                open_files_max: 256,
                file_size_max_bytes: 1024 * 1024,
                output_max_bytes: 128 * 1024,
                request_deadline_ms: 120_000,
            },
            identity: ProviderServiceIdentity {
                gateway_user: "blossom-model-gateway".into(),
                gateway_group: "blossom-model-gateway".into(),
                provider_user: "blossom-model-provider".into(),
                provider_group: "blossom-model-provider".into(),
                access_group: "blossom-ai".into(),
                gateway_unit: "blossom-model-gateway.service".into(),
                provider_unit: "blossom-model-llama-cpp.service".into(),
                namespace_unit: "blossom-model-netns.service".into(),
            },
        };
        manifest.runtime_set_sha256 = artifact_set_digest(&manifest.runtime_files).unwrap();
        manifest.model_set_sha256 = artifact_set_digest(&manifest.model_files).unwrap();
        manifest
    }

    fn write_fixture(
        directory: &TestDirectory,
        bytes: &[u8],
    ) -> (PathBuf, ProviderProfileSpec, u32) {
        let spec = ProviderProfileSpec::compile(fixture()).unwrap();
        let path = directory.path().join("profile.json");
        fs::write(&path, bytes).unwrap();
        #[cfg(unix)]
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        #[cfg(unix)]
        let uid = {
            use std::os::unix::fs::MetadataExt;
            fs::metadata(&path).unwrap().uid()
        };
        #[cfg(not(unix))]
        let uid = 0;
        (path, spec, uid)
    }

    #[test]
    fn canonical_descriptor_read_binds_schema_digest_and_identity() {
        let directory = TestDirectory::new();
        let spec = ProviderProfileSpec::compile(fixture()).unwrap();
        let (path, _, uid) = write_fixture(&directory, spec.canonical_bytes());
        let validated = load_provider_profile(&path, &spec, uid).unwrap();
        assert_eq!(validated.manifest(), spec.manifest());
        assert_eq!(validated.manifest_sha256(), spec.sha256());
        assert_eq!(
            validated.source_bytes(),
            spec.canonical_bytes().len() as u64
        );
        assert_ne!(validated.source_inode(), 0);
    }

    #[test]
    fn rejects_noncanonical_or_modified_manifest() {
        let directory = TestDirectory::new();
        let spec = ProviderProfileSpec::compile(fixture()).unwrap();
        let mut bytes = spec.canonical_bytes().to_vec();
        bytes.push(b'\n');
        let (path, _, uid) = write_fixture(&directory, &bytes);
        assert_eq!(
            load_provider_profile(&path, &spec, uid),
            Err(ProviderProfileError::UnexpectedManifest)
        );
    }

    #[test]
    fn invalid_profile_expansions_are_rejected() {
        let mut cases = Vec::new();
        let mut remote = fixture();
        remote.endpoint = "192.0.2.1:8080".into();
        cases.push(remote);
        let mut argument = fixture();
        argument.executable_arguments.push("--port=9000".into());
        cases.push(argument);
        let mut device = fixture();
        device.filesystem.devices.push("/dev/dri/renderD128".into());
        cases.push(device);
        let mut writable_model = fixture();
        writable_model
            .filesystem
            .writable_paths
            .push("/usr/lib/blossom/models".into());
        cases.push(writable_model);
        let mut broad_write = fixture();
        broad_write.filesystem.writable_paths = vec!["/var".into()];
        cases.push(broad_write);
        let mut environment = fixture();
        environment.environment_names = vec!["LD_PRELOAD".into()];
        cases.push(environment);
        let mut wrong_unit = fixture();
        wrong_unit.identity.provider_unit = "provider.service".into();
        cases.push(wrong_unit);
        for manifest in cases {
            assert_eq!(
                ProviderProfileSpec::compile(manifest).unwrap_err(),
                ProviderProfileError::InvalidManifest
            );
        }
    }

    #[test]
    fn accepts_only_the_code_owned_provider_environment_names() {
        let mut manifest = fixture();
        manifest.environment_names = vec![
            "HOME".into(),
            "LANG".into(),
            "OLLAMA_HOST".into(),
            "OLLAMA_MODELS".into(),
            "OMP_NUM_THREADS".into(),
            "TZ".into(),
        ];
        ProviderProfileSpec::compile(manifest).unwrap();
    }

    #[test]
    fn model_artifact_set_rejects_reordering_duplicates_escape_and_digest_drift() {
        let package = fixed_synthetic_provider_package(GatewayProfile::OllamaCpuV1).unwrap();
        let baseline = package.spec.expected.clone();
        let mut cases = Vec::new();

        let mut reordered = baseline.clone();
        reordered.model_files.reverse();
        reordered.model_set_sha256 = artifact_set_digest(&reordered.model_files).unwrap();
        cases.push(reordered);

        let mut duplicate = baseline.clone();
        duplicate.model_files.push(duplicate.model_files[0].clone());
        duplicate.model_set_sha256 = artifact_set_digest(&duplicate.model_files).unwrap();
        cases.push(duplicate);

        let mut escaped = baseline.clone();
        escaped.model_files[0].path = "/usr/lib/blossom-os/models/outside".into();
        escaped.model_set_sha256 = artifact_set_digest(&escaped.model_files).unwrap();
        cases.push(escaped);

        let mut drifted = baseline;
        drifted.model_files[0].sha256 = "d".repeat(64);
        cases.push(drifted);

        for manifest in cases {
            assert_eq!(
                ProviderProfileSpec::compile(manifest).unwrap_err(),
                ProviderProfileError::InvalidManifest
            );
        }
    }

    #[test]
    fn runtime_artifact_set_rejects_reordering_duplicates_escape_and_unbound_binary() {
        let mut baseline = fixture();
        baseline.runtime_files.push(ProviderArtifact {
            path: "/usr/bin/libprovider.so".into(),
            sha256: "d".repeat(64),
            bytes: 1,
        });
        baseline
            .runtime_files
            .sort_by(|left, right| left.path.cmp(&right.path));
        baseline.runtime_set_sha256 = artifact_set_digest(&baseline.runtime_files).unwrap();
        ProviderProfileSpec::compile(baseline.clone()).unwrap();

        let mut cases = Vec::new();
        let mut reordered = baseline.clone();
        reordered.runtime_files.reverse();
        reordered.runtime_set_sha256 = artifact_set_digest(&reordered.runtime_files).unwrap();
        cases.push(reordered);

        let mut duplicate = baseline.clone();
        duplicate
            .runtime_files
            .push(duplicate.runtime_files[0].clone());
        duplicate.runtime_set_sha256 = artifact_set_digest(&duplicate.runtime_files).unwrap();
        cases.push(duplicate);

        let mut escaped = baseline.clone();
        escaped.runtime_files[0].path = "/opt/unbound-provider".into();
        escaped.runtime_set_sha256 = artifact_set_digest(&escaped.runtime_files).unwrap();
        cases.push(escaped);

        let mut unbound_binary = baseline;
        unbound_binary.binary.sha256 = "e".repeat(64);
        cases.push(unbound_binary);

        for manifest in cases {
            assert_eq!(
                ProviderProfileSpec::compile(manifest).unwrap_err(),
                ProviderProfileError::InvalidManifest
            );
        }
    }

    #[test]
    fn closed_synthetic_packages_bind_manifest_to_rendered_unit() {
        for profile in [GatewayProfile::OllamaCpuV1, GatewayProfile::LlamaCppCpuV1] {
            let first = fixed_synthetic_provider_package(profile)
                .unwrap_or_else(|error| panic!("{profile:?}: {error:?}"));
            let second = fixed_synthetic_provider_package(profile).unwrap();
            assert_eq!(first.profile(), profile);
            assert_eq!(first.rendered_unit(), second.rendered_unit());
            assert_eq!(
                first.spec().canonical_bytes(),
                second.spec().canonical_bytes()
            );

            let manifest: serde_json::Value =
                serde_json::from_slice(first.spec().canonical_bytes()).unwrap();
            assert_eq!(
                manifest["unit_sha256"].as_str().unwrap(),
                hex_digest(first.rendered_unit())
            );
            let arguments = manifest["executable_arguments"]
                .as_array()
                .unwrap()
                .iter()
                .map(|value| value.as_str().unwrap())
                .collect::<Vec<_>>()
                .join(" ");
            let rendered = std::str::from_utf8(first.rendered_unit()).unwrap();
            assert!(rendered.contains(&format!("ExecStart={arguments}\n")));
            let environment_names = rendered
                .lines()
                .filter_map(|line| line.strip_prefix("Environment="))
                .map(|assignment| assignment.split('=').next().unwrap())
                .collect::<Vec<_>>();
            let manifest_environment = manifest["environment_names"]
                .as_array()
                .unwrap()
                .iter()
                .map(|value| value.as_str().unwrap())
                .collect::<Vec<_>>();
            assert_eq!(environment_names, manifest_environment);
            assert!(rendered.contains("TasksMax=64\n"));
            assert!(rendered.contains("MemoryMax=4G\n"));
            assert!(rendered.contains("MemorySwapMax=0\n"));
            assert!(rendered.contains("CPUQuota=200%\n"));
            assert!(rendered.contains("LimitFSIZE=1M\n"));
            assert!(rendered.contains("LimitNOFILE=256\n"));
            assert!(rendered.contains(&format!(
                "BindReadOnlyPaths={} {}\n",
                manifest["runtime_mount"].as_str().unwrap(),
                manifest["model_mount"].as_str().unwrap()
            )));
            assert!(rendered.contains(&format!(
                "ReadWritePaths={}\n",
                manifest["filesystem"]["writable_paths"][0]
                    .as_str()
                    .unwrap()
            )));
            assert!(!rendered.contains("@PROVIDER_"));
            assert!(!rendered.contains("@MODEL_"));
            assert!(!rendered.contains("DeviceAllow="));
            assert!(!rendered.contains("%i"));
        }
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let spec = ProviderProfileSpec::compile(fixture()).unwrap();
        let mut value: serde_json::Value = serde_json::from_slice(spec.canonical_bytes()).unwrap();
        value["unknown"] = serde_json::Value::Bool(true);
        assert!(serde_json::from_value::<ProviderProfileManifest>(value).is_err());
    }

    #[test]
    fn embedded_production_registry_is_canonical_and_closed() {
        const EMBEDDED: &[u8] = include_bytes!(
            "../../../../system/model-runtime/registry/llama-cpp-cpu-x86_64.profile.json"
        );
        let bytes = EMBEDDED.strip_suffix(b"\n").unwrap_or(EMBEDDED);
        let profile = ProviderProfileSpec::from_embedded(bytes).unwrap();
        assert_eq!(profile.manifest().profile, GatewayProfile::LlamaCppCpuV1);
        assert_eq!(profile.canonical_bytes(), bytes);

        let mut changed = bytes.to_vec();
        let final_byte = changed.last_mut().unwrap();
        *final_byte = b' ';
        assert_eq!(
            ProviderProfileSpec::from_embedded(&changed).unwrap_err(),
            ProviderProfileError::InvalidManifest
        );
    }

    #[test]
    fn production_registry_has_no_fallback_profile() {
        assert!(
            production_provider_profile(GatewayProfile::OllamaCpuV1)
                .unwrap()
                .is_none()
        );
        #[cfg(target_arch = "x86_64")]
        assert!(
            production_provider_profile(GatewayProfile::LlamaCppCpuV1)
                .unwrap()
                .is_some()
        );
        #[cfg(not(target_arch = "x86_64"))]
        assert!(
            production_provider_profile(GatewayProfile::LlamaCppCpuV1)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn numeric_or_mutable_service_identity_profiles_are_rejected() {
        let spec = ProviderProfileSpec::compile(fixture()).unwrap();
        let mut old_schema: serde_json::Value =
            serde_json::from_slice(spec.canonical_bytes()).unwrap();
        old_schema["profile_version"] = 2.into();
        old_schema["identity"] = serde_json::json!({
            "gateway_uid": 980,
            "gateway_gid": 980,
            "provider_uid": 981,
            "provider_gid": 981,
            "gateway_unit": "blossom-model-gateway.service",
            "provider_unit": "blossom-model-llama-cpp.service",
            "namespace_unit": "blossom-model-netns.service"
        });
        assert!(serde_json::from_value::<ProviderProfileManifest>(old_schema).is_err());

        let mut wrong_name = fixture();
        wrong_name.identity.provider_user = "caller-selected-provider".into();
        assert_eq!(
            ProviderProfileSpec::compile(wrong_name).unwrap_err(),
            ProviderProfileError::InvalidManifest
        );
    }

    #[cfg(unix)]
    #[test]
    fn unsafe_mode_and_final_symlink_are_rejected() {
        let directory = TestDirectory::new();
        let spec = ProviderProfileSpec::compile(fixture()).unwrap();
        let (path, _, uid) = write_fixture(&directory, spec.canonical_bytes());
        fs::set_permissions(&path, fs::Permissions::from_mode(0o620)).unwrap();
        assert_eq!(
            load_provider_profile(&path, &spec, uid),
            Err(ProviderProfileError::UnsafePermissions)
        );
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        let link = directory.path().join("profile-link.json");
        symlink(&path, &link).unwrap();
        assert_eq!(
            load_provider_profile(&link, &spec, uid),
            Err(ProviderProfileError::OpenFailed)
        );
    }

    #[cfg(unix)]
    #[test]
    fn directory_and_oversized_input_are_rejected() {
        let directory = TestDirectory::new();
        let spec = ProviderProfileSpec::compile(fixture()).unwrap();
        assert!(matches!(
            load_provider_profile(directory.path(), &spec, 0),
            Err(ProviderProfileError::OpenFailed | ProviderProfileError::NotRegularFile)
        ));
        let oversized = vec![b'x'; MAX_PROVIDER_MANIFEST_BYTES + 1];
        let (path, _, uid) = write_fixture(&directory, &oversized);
        assert_eq!(
            load_provider_profile(&path, &spec, uid),
            Err(ProviderProfileError::ManifestTooLarge)
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn symlinked_parent_component_is_rejected_on_linux() {
        let directory = TestDirectory::new();
        let real = directory.path().join("real");
        fs::create_dir(&real).unwrap();
        let spec = ProviderProfileSpec::compile(fixture()).unwrap();
        let path = real.join("profile.json");
        fs::write(&path, spec.canonical_bytes()).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        let link = directory.path().join("linked");
        symlink(&real, &link).unwrap();
        use std::os::unix::fs::MetadataExt;
        let uid = fs::metadata(&path).unwrap().uid();
        assert_eq!(
            load_provider_profile(&link.join("profile.json"), &spec, uid),
            Err(ProviderProfileError::OpenFailed)
        );
    }

    #[cfg(unix)]
    #[test]
    fn wrong_owner_is_rejected_without_requiring_chown() {
        let directory = TestDirectory::new();
        let spec = ProviderProfileSpec::compile(fixture()).unwrap();
        let (path, _, uid) = write_fixture(&directory, spec.canonical_bytes());
        assert_eq!(
            load_provider_profile(&path, &spec, uid.saturating_add(1)),
            Err(ProviderProfileError::WrongOwner)
        );
    }

    #[cfg(unix)]
    #[test]
    fn readiness_binds_accounts_manifest_artifacts_and_rendered_unit() {
        use super::super::runtime_readiness::load_runtime_readiness;
        use std::os::unix::fs::MetadataExt;

        let directory = TestDirectory::new();
        let runtime_path = directory.path().join("runtime");
        fs::create_dir(&runtime_path).unwrap();
        let binary_path = runtime_path.join("provider");
        let model_path = directory.path().join("model.gguf");
        let unit_path = directory.path().join("provider.service");
        let passwd_path = directory.path().join("passwd");
        let group_path = directory.path().join("group");
        let binary = b"synthetic-provider";
        let model = b"synthetic-model";
        let unit = b"[Service]\nExecStart=/synthetic\n";
        fs::write(&binary_path, binary).unwrap();
        fs::write(&model_path, model).unwrap();
        fs::write(&unit_path, unit).unwrap();
        fs::write(
            &passwd_path,
            b"blossom-model-gateway:x:980:980::/:/usr/bin/nologin\nblossom-model-provider:x:981:981::/:/usr/bin/nologin\n",
        )
        .unwrap();
        fs::write(
            &group_path,
            b"blossom-model-gateway:x:980:\nblossom-model-provider:x:981:\nblossom-ai:x:982:blossom-model-gateway\n",
        )
        .unwrap();
        fs::set_permissions(&binary_path, fs::Permissions::from_mode(0o700)).unwrap();
        for path in [&model_path, &unit_path, &passwd_path, &group_path] {
            fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
        }

        let mut manifest = fixture();
        manifest.binary.path = binary_path.clone();
        manifest.binary.sha256 = hex_digest(binary);
        manifest.binary.bytes = binary.len() as u64;
        manifest.runtime_mount = runtime_path.clone();
        manifest.runtime_files = vec![manifest.binary.clone()];
        manifest.runtime_set_sha256 = artifact_set_digest(&manifest.runtime_files).unwrap();
        manifest.model_mount = model_path.clone();
        manifest.model_files = vec![ProviderArtifact {
            path: model_path.clone(),
            sha256: hex_digest(model),
            bytes: model.len() as u64,
        }];
        manifest.model_set_sha256 = artifact_set_digest(&manifest.model_files).unwrap();
        manifest.unit_sha256 = hex_digest(unit);
        manifest.executable_arguments[0] = binary_path.to_string_lossy().into_owned();
        manifest.executable_arguments[2] = model_path.to_string_lossy().into_owned();
        manifest.filesystem.read_only_paths = vec![runtime_path.clone(), model_path];
        let spec = ProviderProfileSpec::compile(manifest).unwrap();
        let (manifest_path, _, uid) = write_fixture(&directory, spec.canonical_bytes());
        let profile = load_provider_profile(&manifest_path, &spec, uid).unwrap();
        let readiness =
            load_runtime_readiness(profile, &passwd_path, &group_path, &unit_path, uid).unwrap();
        assert_eq!(readiness.accounts().access_gid(), 982);
        assert_eq!(readiness.binary().sha256(), hex_digest(binary));
        assert_eq!(readiness.model_files()[0].sha256(), hex_digest(model));
        assert_eq!(readiness.unit().sha256(), hex_digest(unit));
        assert_eq!(
            readiness.binary().device(),
            fs::metadata(readiness.binary().path()).unwrap().dev()
        );

        let unknown = runtime_path.join("unmeasured.so");
        fs::write(&unknown, b"unmeasured").unwrap();
        fs::set_permissions(&unknown, fs::Permissions::from_mode(0o600)).unwrap();
        let profile = load_provider_profile(&manifest_path, &spec, uid).unwrap();
        assert!(matches!(
            load_runtime_readiness(profile, &passwd_path, &group_path, &unit_path, uid),
            Err(super::super::runtime_readiness::RuntimeReadinessError::UnexpectedRuntimeEntry)
        ));
        fs::remove_file(&unknown).unwrap();

        let linked = runtime_path.join("linked.so");
        symlink(&binary_path, &linked).unwrap();
        let profile = load_provider_profile(&manifest_path, &spec, uid).unwrap();
        assert!(matches!(
            load_runtime_readiness(profile, &passwd_path, &group_path, &unit_path, uid),
            Err(super::super::runtime_readiness::RuntimeReadinessError::UnexpectedRuntimeEntry)
        ));
    }

    #[cfg(unix)]
    #[test]
    fn ollama_inventory_rejects_unknown_and_symlinked_store_entries() {
        use super::super::runtime_readiness::{RuntimeReadinessError, validate_model_inventory};
        use std::os::unix::fs::MetadataExt;

        let directory = TestDirectory::new();
        let root = directory.path().join("models");
        let blobs = root.join("blobs");
        let manifests = root.join("manifests");
        fs::create_dir_all(&blobs).unwrap();
        fs::create_dir_all(&manifests).unwrap();
        let blob = blobs.join("sha256-fixture");
        let model_manifest = manifests.join("fixture-model");
        fs::write(&blob, b"blob").unwrap();
        fs::write(&model_manifest, b"manifest").unwrap();
        for path in [&root, &blobs, &manifests] {
            fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
        }
        for path in [&blob, &model_manifest] {
            fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
        }
        let uid = fs::metadata(&root).unwrap().uid();

        let package = fixed_synthetic_provider_package(GatewayProfile::OllamaCpuV1).unwrap();
        let mut manifest = package.spec.expected.clone();
        manifest.model_mount = root.clone();
        manifest.model_files = vec![
            ProviderArtifact {
                path: blob.clone(),
                sha256: hex_digest(b"blob"),
                bytes: 4,
            },
            ProviderArtifact {
                path: model_manifest.clone(),
                sha256: hex_digest(b"manifest"),
                bytes: 8,
            },
        ];
        manifest.model_set_sha256 = artifact_set_digest(&manifest.model_files).unwrap();
        manifest.filesystem.read_only_paths = vec![manifest.runtime_mount.clone(), root];
        let spec = ProviderProfileSpec::compile(manifest).unwrap();
        let (profile_path, _, _) = write_fixture(&directory, spec.canonical_bytes());
        let profile = load_provider_profile(&profile_path, &spec, uid).unwrap();
        assert_eq!(validate_model_inventory(&profile, uid), Ok(()));

        let unknown = blobs.join("unknown");
        fs::write(&unknown, b"unknown").unwrap();
        fs::set_permissions(&unknown, fs::Permissions::from_mode(0o600)).unwrap();
        assert_eq!(
            validate_model_inventory(&profile, uid),
            Err(RuntimeReadinessError::UnexpectedModelEntry)
        );
        fs::remove_file(&unknown).unwrap();

        let link = blobs.join("linked");
        symlink(&blob, &link).unwrap();
        assert_eq!(
            validate_model_inventory(&profile, uid),
            Err(RuntimeReadinessError::UnexpectedModelEntry)
        );
    }
}
