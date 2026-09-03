//! Fail-closed evidence binding for an installed local-model runtime.
//!
//! This module reads account databases and package artifacts through already
//! opened descriptors. It does not start a provider, create a socket, or admit
//! model input.

use super::provider_profile::{
    ProviderProfileError, ProviderProfileSpec, ValidatedProviderProfile,
    load_installed_provider_profile,
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
const MAX_BINARY_BYTES: u64 = 1024 * 1024 * 1024;
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
    binary: RuntimeFileEvidence,
    model: RuntimeFileEvidence,
    unit: RuntimeFileEvidence,
    // Retained so a future launcher can consume the exact validated files
    // instead of reopening attacker-replaceable paths.
    binary_descriptor: File,
    model_descriptor: File,
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
        &self.binary
    }
    pub fn model(&self) -> &RuntimeFileEvidence {
        &self.model
    }
    pub fn unit(&self) -> &RuntimeFileEvidence {
        &self.unit
    }

    #[cfg(unix)]
    pub fn binary_descriptor(&self) -> std::os::fd::BorrowedFd<'_> {
        use std::os::fd::AsFd;
        self.binary_descriptor.as_fd()
    }
    #[cfg(unix)]
    pub fn model_descriptor(&self) -> std::os::fd::BorrowedFd<'_> {
        use std::os::fd::AsFd;
        self.model_descriptor.as_fd()
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
    InvalidAccountDatabase,
    MissingIdentity,
    DuplicateIdentity,
    UnsafeIdentity,
    IdentityMismatch,
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
    let accounts = resolve_accounts(&passwd_bytes, &group_bytes)?;
    let expected = profile.identity();
    if accounts.gateway_uid != expected.gateway_uid()
        || accounts.gateway_gid != expected.gateway_gid()
        || accounts.provider_uid != expected.provider_uid()
        || accounts.provider_gid != expected.provider_gid()
    {
        return Err(RuntimeReadinessError::IdentityMismatch);
    }
    let (binary, binary_descriptor) = read_digest_bound(
        profile.binary().path(),
        expected_owner,
        MAX_BINARY_BYTES,
        profile.binary().sha256(),
        true,
    )?;
    let (model, model_descriptor) = read_digest_bound(
        profile.model().path(),
        expected_owner,
        MAX_MODEL_BYTES,
        profile.model().sha256(),
        false,
    )?;
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
        binary,
        model,
        unit,
        binary_descriptor,
        model_descriptor,
        unit_descriptor,
    })
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

fn unique_group(source: &str, name: &str) -> Result<GroupEntry, RuntimeReadinessError> {
    let mut found = None;
    for line in source.lines().filter(|line| !line.is_empty()) {
        let fields: Vec<_> = line.split(':').collect();
        if fields.len() != 4 {
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
    use nix::fcntl::{OFlag, OpenHow, ResolveFlag, openat2};
    let relative = path
        .strip_prefix("/")
        .map_err(|_| RuntimeReadinessError::InvalidPath)?;
    let root = File::open("/").map_err(|_| RuntimeReadinessError::OpenFailed)?;
    let how = OpenHow::new()
        .flags(OFlag::O_RDONLY | OFlag::O_CLOEXEC | OFlag::O_NOFOLLOW)
        .resolve(
            ResolveFlag::RESOLVE_BENEATH
                | ResolveFlag::RESOLVE_NO_MAGICLINKS
                | ResolveFlag::RESOLVE_NO_SYMLINKS,
        );
    openat2(root, relative, how)
        .map(File::from)
        .map_err(|_| RuntimeReadinessError::OpenFailed)
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
