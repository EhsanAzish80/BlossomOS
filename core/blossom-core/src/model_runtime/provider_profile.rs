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
const PROVIDER_PROFILE_VERSION: u16 = 1;
const MAX_PATH_BYTES: usize = 4096;
const MAX_ARGUMENTS: usize = 64;
const MAX_ARGUMENT_BYTES: usize = 4096;
const MAX_ENVIRONMENT_NAMES: usize = 32;
const MAX_FILESYSTEM_PATHS: usize = 16;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderArtifact {
    path: PathBuf,
    sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderServiceIdentity {
    gateway_uid: u32,
    gateway_gid: u32,
    provider_uid: u32,
    provider_gid: u32,
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
    model: ProviderArtifact,
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
    #[cfg(test)]
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
    validate_artifact(&manifest.model)?;
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
    validate_digest(&artifact.sha256)
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
    if Path::new(&arguments[0]) != manifest.binary.path
        || !arguments
            .iter()
            .any(|argument| Path::new(argument) == manifest.model.path)
    {
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
        if !matches!(name.as_str(), "LANG" | "LC_ALL" | "OMP_NUM_THREADS" | "TZ")
            || name.is_empty()
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
        || filesystem.read_only_paths[0] != manifest.binary.path
        || filesystem.read_only_paths[1] != manifest.model.path
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

fn validate_identity(identity: &ProviderServiceIdentity) -> Result<(), ProviderProfileError> {
    if identity.gateway_uid == 0
        || identity.gateway_gid == 0
        || identity.provider_uid == 0
        || identity.provider_gid == 0
        || identity.gateway_uid == identity.provider_uid
        || identity.gateway_gid == identity.provider_gid
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
        ProviderProfileManifest {
            profile_version: 1,
            profile: GatewayProfile::LlamaCppCpuV1,
            provider: ModelProviderKind::LlamaCpp,
            gateway_protocol_version: GATEWAY_PROTOCOL_VERSION,
            model_protocol_version: MODEL_PROTOCOL_VERSION,
            binary: ProviderArtifact {
                path: "/usr/bin/llama-server".into(),
                sha256: "a".repeat(64),
            },
            model: ProviderArtifact {
                path: "/usr/lib/blossom/models/evidence.gguf".into(),
                sha256: "b".repeat(64),
            },
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
                    "/usr/bin/llama-server".into(),
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
                output_max_bytes: 128 * 1024,
                request_deadline_ms: 120_000,
            },
            identity: ProviderServiceIdentity {
                gateway_uid: 980,
                gateway_gid: 980,
                provider_uid: 981,
                provider_gid: 981,
                gateway_unit: "blossom-model-gateway.service".into(),
                provider_unit: "blossom-model-llama-cpp.service".into(),
                namespace_unit: "blossom-model-netns.service".into(),
            },
        }
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
    fn unknown_fields_are_rejected() {
        let spec = ProviderProfileSpec::compile(fixture()).unwrap();
        let mut value: serde_json::Value = serde_json::from_slice(spec.canonical_bytes()).unwrap();
        value["unknown"] = serde_json::Value::Bool(true);
        assert!(serde_json::from_value::<ProviderProfileManifest>(value).is_err());
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
}
