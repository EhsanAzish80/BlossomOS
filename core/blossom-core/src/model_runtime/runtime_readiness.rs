//! Fail-closed evidence binding for an installed local-model runtime.
//!
//! This module reads account databases and package artifacts through already
//! opened descriptors. It does not start a provider, create a socket, or admit
//! model input.

use super::GatewayPeerCredentials;
use super::provider_profile::{
    ProviderProfileError, ProviderProfileSpec, ValidatedProviderProfile,
    load_installed_provider_profile, load_installed_provider_profile_from_set,
};
use sha2::{Digest, Sha256};
use std::fs::{File, Metadata};
use std::io::Read;
use std::path::{Component, Path, PathBuf};

const PASSWD_PATH: &str = "/etc/passwd";
const GROUP_PATH: &str = "/etc/group";
const UNIT_DIRECTORY: &str = "/usr/lib/systemd/system";
const GATEWAY_USER: &str = "blossom-model-gateway";
const PROVIDER_USER: &str = "blossom-model-provider";
const ACCESS_GROUP: &str = "blossom-ai";
const NOLOGIN_SHELL: &str = "/usr/bin/nologin";
const MAX_ACCOUNT_DATABASE_BYTES: u64 = 1024 * 1024;
const MAX_RUNTIME_BYTES: u64 = 64 * 1024 * 1024 * 1024;
const MAX_MODEL_BYTES: u64 = 64 * 1024 * 1024 * 1024;
const MAX_UNIT_BYTES: u64 = 256 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeFileEvidence {
    path: PathBuf,
    sha256: String,
    bytes: u64,
    device: u64,
    inode: u64,
}

impl RuntimeFileEvidence {
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn sha256(&self) -> &str {
        &self.sha256
    }
    pub fn bytes(&self) -> u64 {
        self.bytes
    }
    pub fn device(&self) -> u64 {
        self.device
    }
    pub fn inode(&self) -> u64 {
        self.inode
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccountDatabaseEvidence {
    passwd: RuntimeFileEvidence,
    group: RuntimeFileEvidence,
}

impl AccountDatabaseEvidence {
    pub fn passwd(&self) -> &RuntimeFileEvidence {
        &self.passwd
    }
    pub fn group(&self) -> &RuntimeFileEvidence {
        &self.group
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedModelIdentities {
    gateway_uid: u32,
    gateway_gid: u32,
    provider_uid: u32,
    provider_gid: u32,
    access_gid: u32,
}

impl ResolvedModelIdentities {
    pub fn gateway_uid(&self) -> u32 {
        self.gateway_uid
    }
    pub fn gateway_gid(&self) -> u32 {
        self.gateway_gid
    }
    pub fn provider_uid(&self) -> u32 {
        self.provider_uid
    }
    pub fn provider_gid(&self) -> u32 {
        self.provider_gid
    }
    pub fn access_gid(&self) -> u32 {
        self.access_gid
    }
}

#[derive(Debug)]
pub struct RuntimeReadinessEvidence {
    profile: ValidatedProviderProfile,
    accounts: ResolvedModelIdentities,
    account_databases: AccountDatabaseEvidence,
    passwd_bytes: Vec<u8>,
    group_bytes: Vec<u8>,
    runtime_files: Vec<RuntimeFileEvidence>,
    binary_index: usize,
    model_files: Vec<RuntimeFileEvidence>,
    unit: RuntimeFileEvidence,
    // Retained so a future launcher can consume the exact validated files
    // instead of reopening attacker-replaceable paths.
    runtime_descriptors: Vec<File>,
    model_descriptors: Vec<File>,
    unit_descriptor: File,
}

impl RuntimeReadinessEvidence {
    pub fn profile(&self) -> &ValidatedProviderProfile {
        &self.profile
    }
    pub fn accounts(&self) -> &ResolvedModelIdentities {
        &self.accounts
    }
    pub fn account_databases(&self) -> &AccountDatabaseEvidence {
        &self.account_databases
    }
    pub fn binary(&self) -> &RuntimeFileEvidence {
        &self.runtime_files[self.binary_index]
    }
    pub fn runtime_files(&self) -> &[RuntimeFileEvidence] {
        &self.runtime_files
    }
    pub fn model_files(&self) -> &[RuntimeFileEvidence] {
        &self.model_files
    }
    pub fn unit(&self) -> &RuntimeFileEvidence {
        &self.unit
    }

    /// Authorize one connected client against the exact account-database bytes
    /// captured during this readiness decision. No pathname is reopened and no
    /// caller-supplied name or group is trusted.
    pub fn authorize_client(
        &self,
        peer: GatewayPeerCredentials,
    ) -> Result<AuthorizedGatewayClient, RuntimeReadinessError> {
        authorize_client_from_snapshot(&self.passwd_bytes, &self.group_bytes, &self.accounts, peer)
    }

    #[cfg(unix)]
    pub fn binary_descriptor(&self) -> std::os::fd::BorrowedFd<'_> {
        use std::os::fd::AsFd;
        self.runtime_descriptors[self.binary_index].as_fd()
    }
    #[cfg(unix)]
    pub fn runtime_descriptors(&self) -> impl Iterator<Item = std::os::fd::BorrowedFd<'_>> {
        use std::os::fd::AsFd;
        self.runtime_descriptors.iter().map(File::as_fd)
    }
    #[cfg(unix)]
    pub fn model_descriptors(&self) -> impl Iterator<Item = std::os::fd::BorrowedFd<'_>> {
        use std::os::fd::AsFd;
        self.model_descriptors.iter().map(File::as_fd)
    }
    #[cfg(unix)]
    pub fn unit_descriptor(&self) -> std::os::fd::BorrowedFd<'_> {
        use std::os::fd::AsFd;
        self.unit_descriptor.as_fd()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeReadinessError {
    Profile(ProviderProfileError),
    InvalidPath,
    OpenFailed,
    MetadataFailed,
    NotRegularFile,
    WrongOwner,
    UnsafePermissions,
    NotExecutable,
    TooLarge,
    ReadFailed,
    SourceChanged,
    DigestMismatch,
    SizeMismatch,
    InvalidAccountDatabase,
    MissingIdentity,
    DuplicateIdentity,
    UnsafeIdentity,
    IdentityMismatch,
    UnsafeModelDirectory,
    UnexpectedModelEntry,
    UnexpectedRuntimeEntry,
    UnauthorizedClient,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AuthorizedGatewayClient {
    uid: u32,
    pid: u32,
}

impl AuthorizedGatewayClient {
    pub fn uid(&self) -> u32 {
        self.uid
    }

    pub fn pid(&self) -> u32 {
        self.pid
    }
}

impl From<ProviderProfileError> for RuntimeReadinessError {
    fn from(value: ProviderProfileError) -> Self {
        Self::Profile(value)
    }
}

/// Validate the complete installed package boundary without starting it.
pub fn load_installed_runtime_readiness(
    manifest_path: &Path,
    expected: &ProviderProfileSpec,
) -> Result<RuntimeReadinessEvidence, RuntimeReadinessError> {
    let profile = load_installed_provider_profile(manifest_path, expected)?;
    let unit_path = Path::new(UNIT_DIRECTORY).join(profile.identity().provider_unit());
    load_runtime_readiness(
        profile,
        Path::new(PASSWD_PATH),
        Path::new(GROUP_PATH),
        &unit_path,
        0,
    )
}

/// Validate one root-owned active profile against a closed embedded set while
/// opening and reading the manifest only once.
pub fn load_installed_runtime_readiness_from_set(
    manifest_path: &Path,
    expected: &[ProviderProfileSpec],
) -> Result<RuntimeReadinessEvidence, RuntimeReadinessError> {
    let profile = load_installed_provider_profile_from_set(manifest_path, expected)?;
    let unit_path = Path::new(UNIT_DIRECTORY).join(profile.identity().provider_unit());
    load_runtime_readiness(
        profile,
        Path::new(PASSWD_PATH),
        Path::new(GROUP_PATH),
        &unit_path,
        0,
    )
}

pub(super) fn load_runtime_readiness(
    profile: ValidatedProviderProfile,
    passwd_path: &Path,
    group_path: &Path,
    unit_path: &Path,
    expected_owner: u32,
) -> Result<RuntimeReadinessEvidence, RuntimeReadinessError> {
    let (passwd_bytes, passwd, _) = read_root_owned(
        passwd_path,
        expected_owner,
        MAX_ACCOUNT_DATABASE_BYTES,
        false,
        true,
    )?;
    let (group_bytes, group, _) = read_root_owned(
        group_path,
        expected_owner,
        MAX_ACCOUNT_DATABASE_BYTES,
        false,
        true,
    )?;
    let expected = profile.identity();
    if expected.gateway_user() != GATEWAY_USER
        || expected.gateway_group() != GATEWAY_USER
        || expected.provider_user() != PROVIDER_USER
        || expected.provider_group() != PROVIDER_USER
        || expected.access_group() != ACCESS_GROUP
    {
        return Err(RuntimeReadinessError::IdentityMismatch);
    }
    let accounts = resolve_accounts(&passwd_bytes, &group_bytes)?;
    validate_artifact_inventory(
        profile.runtime_mount(),
        profile.runtime_files(),
        expected_owner,
        RuntimeReadinessError::UnexpectedRuntimeEntry,
    )?;
    let mut runtime_files = Vec::with_capacity(profile.runtime_files().len());
    let mut runtime_descriptors = Vec::with_capacity(profile.runtime_files().len());
    let mut runtime_total = 0_u64;
    let mut binary_index = None;
    for artifact in profile.runtime_files() {
        let remaining = MAX_RUNTIME_BYTES
            .checked_sub(runtime_total)
            .ok_or(RuntimeReadinessError::TooLarge)?;
        let is_binary = artifact.path() == profile.binary().path();
        let (evidence, descriptor) = read_digest_bound(
            artifact.path(),
            expected_owner,
            remaining,
            artifact.sha256(),
            is_binary,
        )?;
        if evidence.bytes() != artifact.bytes() {
            return Err(RuntimeReadinessError::SizeMismatch);
        }
        if is_binary {
            binary_index = Some(runtime_files.len());
        }
        runtime_total = runtime_total
            .checked_add(evidence.bytes())
            .ok_or(RuntimeReadinessError::TooLarge)?;
        runtime_files.push(evidence);
        runtime_descriptors.push(descriptor);
    }
    let binary_index = binary_index.ok_or(RuntimeReadinessError::UnexpectedRuntimeEntry)?;
    validate_model_inventory(&profile, expected_owner)?;
    let mut model_files = Vec::with_capacity(profile.model_files().len());
    let mut model_descriptors = Vec::with_capacity(profile.model_files().len());
    let mut model_total = 0_u64;
    for artifact in profile.model_files() {
        let remaining = MAX_MODEL_BYTES
            .checked_sub(model_total)
            .ok_or(RuntimeReadinessError::TooLarge)?;
        let (evidence, descriptor) = read_digest_bound(
            artifact.path(),
            expected_owner,
            remaining,
            artifact.sha256(),
            false,
        )?;
        if evidence.bytes() != artifact.bytes() {
            return Err(RuntimeReadinessError::SizeMismatch);
        }
        model_total = model_total
            .checked_add(evidence.bytes())
            .ok_or(RuntimeReadinessError::TooLarge)?;
        model_files.push(evidence);
        model_descriptors.push(descriptor);
    }
    let (unit, unit_descriptor) = read_digest_bound(
        unit_path,
        expected_owner,
        MAX_UNIT_BYTES,
        profile.unit_sha256(),
        false,
    )?;
    Ok(RuntimeReadinessEvidence {
        profile,
        accounts,
        account_databases: AccountDatabaseEvidence { passwd, group },
        passwd_bytes,
        group_bytes,
        runtime_files,
        binary_index,
        model_files,
        unit,
        runtime_descriptors,
        model_descriptors,
        unit_descriptor,
    })
}

fn authorize_client_from_snapshot(
    passwd: &[u8],
    group: &[u8],
    identities: &ResolvedModelIdentities,
    peer: GatewayPeerCredentials,
) -> Result<AuthorizedGatewayClient, RuntimeReadinessError> {
    if peer.pid == 0
        || peer.uid == 0
        || peer.uid == identities.gateway_uid
        || peer.uid == identities.provider_uid
    {
        return Err(RuntimeReadinessError::UnauthorizedClient);
    }
    let passwd =
        std::str::from_utf8(passwd).map_err(|_| RuntimeReadinessError::InvalidAccountDatabase)?;
    let group =
        std::str::from_utf8(group).map_err(|_| RuntimeReadinessError::InvalidAccountDatabase)?;
    if passwd.contains('\0') || group.contains('\0') {
        return Err(RuntimeReadinessError::InvalidAccountDatabase);
    }
    let client = unique_passwd_by_uid(passwd, peer.uid)?;
    let access = unique_group(group, ACCESS_GROUP)?;
    if access.gid != identities.access_gid {
        return Err(RuntimeReadinessError::IdentityMismatch);
    }
    let membership_count = access
        .members
        .iter()
        .filter(|member| member.as_str() == client.name)
        .count();
    if membership_count > 1 {
        return Err(RuntimeReadinessError::InvalidAccountDatabase);
    }
    if client.gid != identities.access_gid && membership_count != 1 {
        return Err(RuntimeReadinessError::UnauthorizedClient);
    }
    Ok(AuthorizedGatewayClient {
        uid: peer.uid,
        pid: peer.pid,
    })
}

pub(super) fn validate_model_inventory(
    profile: &ValidatedProviderProfile,
    expected_owner: u32,
) -> Result<(), RuntimeReadinessError> {
    if profile.profile() == super::GatewayProfile::LlamaCppCpuV1 {
        return Ok(());
    }
    validate_artifact_inventory(
        profile.model_mount(),
        profile.model_files(),
        expected_owner,
        RuntimeReadinessError::UnexpectedModelEntry,
    )
}

fn validate_artifact_inventory(
    mount: &Path,
    artifacts: &[super::provider_profile::ProviderArtifact],
    expected_owner: u32,
    unexpected: RuntimeReadinessError,
) -> Result<(), RuntimeReadinessError> {
    if mount.is_file() && artifacts.len() == 1 && artifacts[0].path() == mount {
        return Ok(());
    }
    let mut observed = Vec::new();
    collect_artifact_files(mount, expected_owner, &mut observed, unexpected)?;
    observed.sort();
    let expected = artifacts
        .iter()
        .map(|artifact| artifact.path().to_path_buf())
        .collect::<Vec<_>>();
    if observed != expected {
        return Err(unexpected);
    }
    Ok(())
}

fn collect_artifact_files(
    directory: &Path,
    expected_owner: u32,
    files: &mut Vec<PathBuf>,
    unexpected: RuntimeReadinessError,
) -> Result<(), RuntimeReadinessError> {
    if files.len() > 4_096 {
        return Err(RuntimeReadinessError::TooLarge);
    }
    let before =
        std::fs::symlink_metadata(directory).map_err(|_| RuntimeReadinessError::OpenFailed)?;
    validate_directory_metadata(&before, expected_owner)?;
    let entries = std::fs::read_dir(directory).map_err(|_| RuntimeReadinessError::ReadFailed)?;
    for entry in entries {
        let entry = entry.map_err(|_| RuntimeReadinessError::ReadFailed)?;
        let path = entry.path();
        let metadata =
            std::fs::symlink_metadata(&path).map_err(|_| RuntimeReadinessError::MetadataFailed)?;
        if metadata.file_type().is_symlink() {
            return Err(unexpected);
        }
        if metadata.is_dir() {
            collect_artifact_files(&path, expected_owner, files, unexpected)?;
        } else if metadata.is_file() {
            files.push(path);
            if files.len() > 4_096 {
                return Err(RuntimeReadinessError::TooLarge);
            }
        } else {
            return Err(unexpected);
        }
    }
    let after =
        std::fs::symlink_metadata(directory).map_err(|_| RuntimeReadinessError::MetadataFailed)?;
    if !same_state(&before, &after) {
        return Err(RuntimeReadinessError::SourceChanged);
    }
    Ok(())
}

#[cfg(unix)]
fn validate_directory_metadata(metadata: &Metadata, uid: u32) -> Result<(), RuntimeReadinessError> {
    use std::os::unix::fs::MetadataExt;
    if !metadata.is_dir() {
        return Err(RuntimeReadinessError::UnsafeModelDirectory);
    }
    if metadata.uid() != uid {
        return Err(RuntimeReadinessError::WrongOwner);
    }
    if metadata.mode() & 0o022 != 0 {
        return Err(RuntimeReadinessError::UnsafePermissions);
    }
    Ok(())
}

#[cfg(not(unix))]
fn validate_directory_metadata(_: &Metadata, _: u32) -> Result<(), RuntimeReadinessError> {
    Err(RuntimeReadinessError::MetadataFailed)
}

fn read_digest_bound(
    path: &Path,
    expected_owner: u32,
    maximum: u64,
    expected_digest: &str,
    executable: bool,
) -> Result<(RuntimeFileEvidence, File), RuntimeReadinessError> {
    let (_, evidence, descriptor) =
        read_root_owned(path, expected_owner, maximum, executable, false)?;
    if evidence.sha256 != expected_digest {
        return Err(RuntimeReadinessError::DigestMismatch);
    }
    Ok((evidence, descriptor))
}

fn read_root_owned(
    path: &Path,
    expected_owner: u32,
    maximum: u64,
    executable: bool,
    capture_bytes: bool,
) -> Result<(Vec<u8>, RuntimeFileEvidence, File), RuntimeReadinessError> {
    validate_path(path)?;
    let mut file = open_beneath_root(path)?;
    let before = file
        .metadata()
        .map_err(|_| RuntimeReadinessError::MetadataFailed)?;
    validate_metadata(&before, expected_owner, executable)?;
    if before.len() > maximum || before.len() > usize::MAX as u64 {
        return Err(RuntimeReadinessError::TooLarge);
    }
    let mut bytes = if capture_bytes {
        Vec::with_capacity(before.len() as usize)
    } else {
        Vec::new()
    };
    let mut digest = Sha256::new();
    let mut total = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|_| RuntimeReadinessError::ReadFailed)?;
        if count == 0 {
            break;
        }
        total = total
            .checked_add(count as u64)
            .ok_or(RuntimeReadinessError::TooLarge)?;
        if total > maximum {
            return Err(RuntimeReadinessError::TooLarge);
        }
        digest.update(&buffer[..count]);
        if capture_bytes {
            bytes.extend_from_slice(&buffer[..count]);
        }
    }
    let after = file
        .metadata()
        .map_err(|_| RuntimeReadinessError::MetadataFailed)?;
    if !same_state(&before, &after) || after.len() != total {
        return Err(RuntimeReadinessError::SourceChanged);
    }
    let sha256 = digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    Ok((
        bytes,
        RuntimeFileEvidence {
            path: path.to_path_buf(),
            sha256,
            bytes: after.len(),
            device: metadata_device(&after),
            inode: metadata_inode(&after),
        },
        file,
    ))
}

fn resolve_accounts(
    passwd: &[u8],
    group: &[u8],
) -> Result<ResolvedModelIdentities, RuntimeReadinessError> {
    let passwd =
        std::str::from_utf8(passwd).map_err(|_| RuntimeReadinessError::InvalidAccountDatabase)?;
    let group =
        std::str::from_utf8(group).map_err(|_| RuntimeReadinessError::InvalidAccountDatabase)?;
    if passwd.contains('\0') || group.contains('\0') {
        return Err(RuntimeReadinessError::InvalidAccountDatabase);
    }
    let gateway = unique_passwd(passwd, GATEWAY_USER)?;
    let provider = unique_passwd(passwd, PROVIDER_USER)?;
    let gateway_group = unique_group(group, GATEWAY_USER)?;
    let provider_group = unique_group(group, PROVIDER_USER)?;
    let access = unique_group(group, ACCESS_GROUP)?;
    if gateway.uid == 0
        || gateway.gid == 0
        || provider.uid == 0
        || provider.gid == 0
        || gateway.uid == provider.uid
        || gateway.gid == provider.gid
        || access.gid == 0
        || access.gid == gateway.gid
        || access.gid == provider.gid
        || gateway.home != "/"
        || provider.home != "/"
        || gateway.shell != NOLOGIN_SHELL
        || provider.shell != NOLOGIN_SHELL
        || gateway_group.gid != gateway.gid
        || provider_group.gid != provider.gid
        || !access.members.iter().any(|member| member == GATEWAY_USER)
        || access.members.iter().any(|member| member == PROVIDER_USER)
    {
        return Err(RuntimeReadinessError::UnsafeIdentity);
    }
    Ok(ResolvedModelIdentities {
        gateway_uid: gateway.uid,
        gateway_gid: gateway.gid,
        provider_uid: provider.uid,
        provider_gid: provider.gid,
        access_gid: access.gid,
    })
}

struct PasswdEntry<'a> {
    name: &'a str,
    uid: u32,
    gid: u32,
    home: &'a str,
    shell: &'a str,
}
struct GroupEntry {
    gid: u32,
    members: Vec<String>,
}

fn unique_passwd<'a>(
    source: &'a str,
    name: &str,
) -> Result<PasswdEntry<'a>, RuntimeReadinessError> {
    let mut found = None;
    for line in source.lines().filter(|line| !line.is_empty()) {
        let fields: Vec<_> = line.split(':').collect();
        if fields.len() != 7 {
            return Err(RuntimeReadinessError::InvalidAccountDatabase);
        }
        if fields[0] == name {
            if found.is_some() {
                return Err(RuntimeReadinessError::DuplicateIdentity);
            }
            found = Some(PasswdEntry {
                name: fields[0],
                uid: fields[2]
                    .parse()
                    .map_err(|_| RuntimeReadinessError::InvalidAccountDatabase)?,
                gid: fields[3]
                    .parse()
                    .map_err(|_| RuntimeReadinessError::InvalidAccountDatabase)?,
                home: fields[5],
                shell: fields[6],
            });
        }
    }
    found.ok_or(RuntimeReadinessError::MissingIdentity)
}

fn unique_passwd_by_uid(source: &str, uid: u32) -> Result<PasswdEntry<'_>, RuntimeReadinessError> {
    let mut found = None;
    for line in source.lines().filter(|line| !line.is_empty()) {
        let fields: Vec<_> = line.split(':').collect();
        if fields.len() != 7 || !valid_account_name(fields[0]) {
            return Err(RuntimeReadinessError::InvalidAccountDatabase);
        }
        let parsed_uid = fields[2]
            .parse::<u32>()
            .map_err(|_| RuntimeReadinessError::InvalidAccountDatabase)?;
        let parsed_gid = fields[3]
            .parse::<u32>()
            .map_err(|_| RuntimeReadinessError::InvalidAccountDatabase)?;
        if parsed_uid == uid {
            if found.is_some() {
                return Err(RuntimeReadinessError::DuplicateIdentity);
            }
            found = Some(PasswdEntry {
                name: fields[0],
                uid: parsed_uid,
                gid: parsed_gid,
                home: fields[5],
                shell: fields[6],
            });
        }
    }
    found.ok_or(RuntimeReadinessError::UnauthorizedClient)
}

fn unique_group(source: &str, name: &str) -> Result<GroupEntry, RuntimeReadinessError> {
    let mut found = None;
    for line in source.lines().filter(|line| !line.is_empty()) {
        let fields: Vec<_> = line.split(':').collect();
        if fields.len() != 4 || !valid_account_name(fields[0]) {
            return Err(RuntimeReadinessError::InvalidAccountDatabase);
        }
        if fields[0] == name {
            if found.is_some() {
                return Err(RuntimeReadinessError::DuplicateIdentity);
            }
            let members = if fields[3].is_empty() {
                vec![]
            } else {
                fields[3].split(',').map(str::to_owned).collect()
            };
            if members.iter().any(|member| !valid_account_name(member)) {
                return Err(RuntimeReadinessError::InvalidAccountDatabase);
            }
            found = Some(GroupEntry {
                gid: fields[2]
                    .parse()
                    .map_err(|_| RuntimeReadinessError::InvalidAccountDatabase)?,
                members,
            });
        }
    }
    found.ok_or(RuntimeReadinessError::MissingIdentity)
}

fn valid_account_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 256
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
}

fn validate_path(path: &Path) -> Result<(), RuntimeReadinessError> {
    let text = path.to_str().ok_or(RuntimeReadinessError::InvalidPath)?;
    if !path.is_absolute()
        || text.len() > 4096
        || text.contains('\0')
        || path
            .components()
            .any(|part| matches!(part, Component::ParentDir | Component::CurDir))
    {
        return Err(RuntimeReadinessError::InvalidPath);
    }
    Ok(())
}

#[cfg(unix)]
fn validate_metadata(
    metadata: &Metadata,
    uid: u32,
    executable: bool,
) -> Result<(), RuntimeReadinessError> {
    use std::os::unix::fs::MetadataExt;
    if !metadata.file_type().is_file() {
        return Err(RuntimeReadinessError::NotRegularFile);
    }
    if metadata.uid() != uid {
        return Err(RuntimeReadinessError::WrongOwner);
    }
    if metadata.mode() & 0o022 != 0 {
        return Err(RuntimeReadinessError::UnsafePermissions);
    }
    if executable && metadata.mode() & 0o111 == 0 {
        return Err(RuntimeReadinessError::NotExecutable);
    }
    Ok(())
}

#[cfg(not(unix))]
fn validate_metadata(_: &Metadata, _: u32, _: bool) -> Result<(), RuntimeReadinessError> {
    Err(RuntimeReadinessError::MetadataFailed)
}

#[cfg(unix)]
fn same_state(a: &Metadata, b: &Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    a.dev() == b.dev()
        && a.ino() == b.ino()
        && a.len() == b.len()
        && a.mtime() == b.mtime()
        && a.mtime_nsec() == b.mtime_nsec()
        && a.mode() == b.mode()
        && a.uid() == b.uid()
        && a.gid() == b.gid()
}
#[cfg(not(unix))]
fn same_state(a: &Metadata, b: &Metadata) -> bool {
    a.len() == b.len() && a.modified().ok() == b.modified().ok()
}

#[cfg(unix)]
fn metadata_device(m: &Metadata) -> u64 {
    use std::os::unix::fs::MetadataExt;
    m.dev()
}
#[cfg(not(unix))]
fn metadata_device(_: &Metadata) -> u64 {
    0
}
#[cfg(unix)]
fn metadata_inode(m: &Metadata) -> u64 {
    use std::os::unix::fs::MetadataExt;
    m.ino()
}
#[cfg(not(unix))]
fn metadata_inode(_: &Metadata) -> u64 {
    0
}

#[cfg(target_os = "linux")]
fn open_beneath_root(path: &Path) -> Result<File, RuntimeReadinessError> {
    super::open_absolute_file_no_symlinks(path).map_err(|_| RuntimeReadinessError::OpenFailed)
}

#[cfg(all(unix, not(target_os = "linux")))]
fn open_beneath_root(path: &Path) -> Result<File, RuntimeReadinessError> {
    use std::os::unix::fs::OpenOptionsExt;
    std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW)
        .open(path)
        .map_err(|_| RuntimeReadinessError::OpenFailed)
}

#[cfg(not(unix))]
fn open_beneath_root(_: &Path) -> Result<File, RuntimeReadinessError> {
    Err(RuntimeReadinessError::OpenFailed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(1);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let sequence = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "blossom-runtime-readiness-{}-{sequence}",
                std::process::id()
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn resolves_only_distinct_non_login_identities_and_access_membership() {
        let passwd = b"blossom-model-gateway:x:980:980::/:/usr/bin/nologin\nblossom-model-provider:x:981:981::/:/usr/bin/nologin\n";
        let group = b"blossom-model-gateway:x:980:\nblossom-model-provider:x:981:\nblossom-ai:x:982:blossom-model-gateway\n";
        let identities = resolve_accounts(passwd, group).unwrap();
        assert_eq!(identities.gateway_uid(), 980);
        assert_eq!(identities.provider_uid(), 981);
        assert_eq!(identities.access_gid(), 982);
    }

    #[test]
    fn authorizes_primary_or_supplementary_client_from_retained_snapshot() {
        let passwd = b"blossom-model-gateway:x:980:980::/:/usr/bin/nologin\nblossom-model-provider:x:981:981::/:/usr/bin/nologin\nalice:x:1000:1000::/home/alice:/bin/bash\nbob:x:1001:982::/home/bob:/bin/bash\n";
        let group = b"blossom-model-gateway:x:980:\nblossom-model-provider:x:981:\nblossom-ai:x:982:blossom-model-gateway,alice\n";
        let identities = resolve_accounts(passwd, group).unwrap();
        let supplementary = authorize_client_from_snapshot(
            passwd,
            group,
            &identities,
            GatewayPeerCredentials {
                pid: 10,
                uid: 1000,
                gid: 1000,
            },
        )
        .unwrap();
        assert_eq!((supplementary.uid(), supplementary.pid()), (1000, 10));
        let primary = authorize_client_from_snapshot(
            passwd,
            group,
            &identities,
            GatewayPeerCredentials {
                pid: 11,
                uid: 1001,
                gid: 1001,
            },
        )
        .unwrap();
        assert_eq!((primary.uid(), primary.pid()), (1001, 11));
    }

    #[test]
    fn client_eligibility_rejects_privileged_unknown_duplicate_and_unlisted_peers() {
        let passwd = b"blossom-model-gateway:x:980:980::/:/usr/bin/nologin\nblossom-model-provider:x:981:981::/:/usr/bin/nologin\nalice:x:1000:1000::/home/alice:/bin/bash\nmallory:x:1002:1002::/home/mallory:/bin/bash\n";
        let group = b"blossom-model-gateway:x:980:\nblossom-model-provider:x:981:\nblossom-ai:x:982:blossom-model-gateway,alice\n";
        let identities = resolve_accounts(passwd, group).unwrap();
        for peer in [
            GatewayPeerCredentials {
                pid: 1,
                uid: 0,
                gid: 0,
            },
            GatewayPeerCredentials {
                pid: 1,
                uid: 980,
                gid: 980,
            },
            GatewayPeerCredentials {
                pid: 1,
                uid: 981,
                gid: 981,
            },
            GatewayPeerCredentials {
                pid: 0,
                uid: 1000,
                gid: 1000,
            },
            GatewayPeerCredentials {
                pid: 1,
                uid: 1002,
                gid: 1002,
            },
            GatewayPeerCredentials {
                pid: 1,
                uid: 9999,
                gid: 9999,
            },
        ] {
            assert_eq!(
                authorize_client_from_snapshot(passwd, group, &identities, peer),
                Err(RuntimeReadinessError::UnauthorizedClient)
            );
        }

        let duplicate_uid = b"blossom-model-gateway:x:980:980::/:/usr/bin/nologin\nblossom-model-provider:x:981:981::/:/usr/bin/nologin\nalice:x:1000:1000::/home/alice:/bin/bash\nalias:x:1000:1000::/home/alias:/bin/bash\n";
        assert_eq!(
            authorize_client_from_snapshot(
                duplicate_uid,
                group,
                &identities,
                GatewayPeerCredentials {
                    pid: 2,
                    uid: 1000,
                    gid: 1000
                },
            ),
            Err(RuntimeReadinessError::DuplicateIdentity)
        );
        let duplicate_member = b"blossom-model-gateway:x:980:\nblossom-model-provider:x:981:\nblossom-ai:x:982:blossom-model-gateway,alice,alice\n";
        assert_eq!(
            authorize_client_from_snapshot(
                passwd,
                duplicate_member,
                &identities,
                GatewayPeerCredentials {
                    pid: 2,
                    uid: 1000,
                    gid: 1000
                },
            ),
            Err(RuntimeReadinessError::InvalidAccountDatabase)
        );
    }

    #[test]
    fn rejects_root_shared_login_duplicate_and_provider_access_identities() {
        let valid_group = b"blossom-model-gateway:x:980:\nblossom-model-provider:x:981:\nblossom-ai:x:982:blossom-model-gateway\n";
        let cases: &[&[u8]] = &[
            b"blossom-model-gateway:x:0:980::/:/usr/bin/nologin\nblossom-model-provider:x:981:981::/:/usr/bin/nologin\n",
            b"blossom-model-gateway:x:980:980::/:/bin/sh\nblossom-model-provider:x:981:981::/:/usr/bin/nologin\n",
            b"blossom-model-gateway:x:980:980::/:/usr/bin/nologin\nblossom-model-gateway:x:982:982::/:/usr/bin/nologin\nblossom-model-provider:x:981:981::/:/usr/bin/nologin\n",
        ];
        for passwd in cases {
            assert!(resolve_accounts(passwd, valid_group).is_err());
        }
        let passwd = b"blossom-model-gateway:x:980:980::/:/usr/bin/nologin\nblossom-model-provider:x:981:981::/:/usr/bin/nologin\n";
        let bad_group = b"blossom-model-gateway:x:980:\nblossom-model-provider:x:981:\nblossom-ai:x:982:blossom-model-gateway,blossom-model-provider\n";
        assert_eq!(
            resolve_accounts(passwd, bad_group),
            Err(RuntimeReadinessError::UnsafeIdentity)
        );
    }

    #[cfg(unix)]
    #[test]
    fn artifact_validation_rejects_digest_mode_size_and_symlink_expansion() {
        let directory = TestDirectory::new();
        let path = directory.0.join("artifact");
        fs::write(&path, b"fixed").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        let uid = fs::metadata(&path).unwrap().uid();
        let digest: String = Sha256::digest(b"fixed")
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        assert!(read_digest_bound(&path, uid, 5, &digest, true).is_ok());
        assert_eq!(
            read_digest_bound(&path, uid, 5, &"0".repeat(64), true).unwrap_err(),
            RuntimeReadinessError::DigestMismatch
        );
        assert_eq!(
            read_digest_bound(&path, uid, 4, &digest, true).unwrap_err(),
            RuntimeReadinessError::TooLarge
        );
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        assert_eq!(
            read_digest_bound(&path, uid, 5, &digest, true).unwrap_err(),
            RuntimeReadinessError::NotExecutable
        );
        fs::set_permissions(&path, fs::Permissions::from_mode(0o622)).unwrap();
        assert_eq!(
            read_digest_bound(&path, uid, 5, &digest, false).unwrap_err(),
            RuntimeReadinessError::UnsafePermissions
        );
        let link = directory.0.join("artifact-link");
        symlink(&path, &link).unwrap();
        assert_eq!(
            read_digest_bound(&link, uid, 5, &digest, false).unwrap_err(),
            RuntimeReadinessError::OpenFailed
        );
    }
}
