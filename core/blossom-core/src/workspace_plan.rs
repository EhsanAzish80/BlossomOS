//! Closed, non-authorizing workspace organization proposals.
//!
//! This module validates untrusted plan shape before any filesystem authority is
//! opened. A validated proposal is still data: a later resolver must derive
//! identities, retain directory authority and build the approval-bound prepared
//! plan described by ADR-0036.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
#[cfg(all(target_os = "linux", target_env = "gnu"))]
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

pub const WORKSPACE_PLAN_SCHEMA_VERSION: u16 = 1;
pub const MAX_WORKSPACE_PLAN_EFFECTS: usize = 128;
pub const MAX_WORKSPACE_PLAN_MESSAGE_BYTES: usize = 64 * 1024;
pub const MAX_WORKSPACE_RELATIVE_PATH_BYTES: usize = 1024;
pub const MAX_WORKSPACE_PATH_COMPONENT_BYTES: usize = 255;
pub const MAX_WORKSPACE_PATH_DEPTH: usize = 16;
pub const MAX_WORKSPACE_PLAN_DIRECTORY_AUTHORITIES: usize = 64;
pub const MAX_PENDING_WORKSPACE_PLANS: usize = 4;

#[derive(Clone, Debug)]
pub struct WorkspacePlanCapacity {
    inner: Arc<WorkspacePlanCapacityInner>,
}

#[derive(Debug)]
struct WorkspacePlanCapacityInner {
    in_use: AtomicUsize,
}

#[derive(Debug)]
pub struct WorkspacePlanCapacityReservation {
    inner: Arc<WorkspacePlanCapacityInner>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkspacePlanCapacityError {
    Exhausted,
}

impl Default for WorkspacePlanCapacity {
    fn default() -> Self {
        Self {
            inner: Arc::new(WorkspacePlanCapacityInner {
                in_use: AtomicUsize::new(0),
            }),
        }
    }
}

impl WorkspacePlanCapacity {
    /// Admission must happen before plan resolution opens filesystem authority.
    pub fn reserve(&self) -> Result<WorkspacePlanCapacityReservation, WorkspacePlanCapacityError> {
        self.inner
            .in_use
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                (current < MAX_PENDING_WORKSPACE_PLANS).then_some(current + 1)
            })
            .map_err(|_| WorkspacePlanCapacityError::Exhausted)?;
        Ok(WorkspacePlanCapacityReservation {
            inner: Arc::clone(&self.inner),
        })
    }

    pub fn in_use(&self) -> usize {
        self.inner.in_use.load(Ordering::Acquire)
    }
}

impl Drop for WorkspacePlanCapacityReservation {
    fn drop(&mut self) {
        let previous = self.inner.in_use.fetch_sub(1, Ordering::AcqRel);
        debug_assert!(previous > 0, "workspace plan capacity underflow");
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspacePlanOrigin {
    UserRequested,
    DeterministicFacts,
    ModelProposedScheme,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkspacePlanProposalWire {
    schema: u16,
    effects: Vec<WorkspacePlanEffectWire>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum WorkspacePlanEffectWire {
    CreateDirectory { path: String },
    MoveFile { source: String, destination: String },
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ValidatedWorkspacePlanEffect {
    CreateDirectory {
        path: WorkspaceRelativePath,
    },
    MoveFile {
        source: WorkspaceRelativePath,
        destination: WorkspaceRelativePath,
    },
}

impl ValidatedWorkspacePlanEffect {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::CreateDirectory { .. } => "create_directory",
            Self::MoveFile { .. } => "move_file",
        }
    }

    pub fn source(&self) -> Option<&WorkspaceRelativePath> {
        match self {
            Self::CreateDirectory { .. } => None,
            Self::MoveFile { source, .. } => Some(source),
        }
    }

    pub fn destination(&self) -> &WorkspaceRelativePath {
        match self {
            Self::CreateDirectory { path } => path,
            Self::MoveFile { destination, .. } => destination,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct WorkspaceRelativePath(String);

impl WorkspaceRelativePath {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn parse(value: String) -> Result<Self, WorkspacePlanError> {
        if value.is_empty() || value.len() > MAX_WORKSPACE_RELATIVE_PATH_BYTES {
            return Err(WorkspacePlanError::InvalidPath);
        }
        if value.starts_with('/') || value.ends_with('/') {
            return Err(WorkspacePlanError::InvalidPath);
        }
        let mut depth = 0usize;
        for component in value.split('/') {
            depth += 1;
            if component.is_empty()
                || component == "."
                || component == ".."
                || component.len() > MAX_WORKSPACE_PATH_COMPONENT_BYTES
                || component.chars().any(forbidden_path_character)
            {
                return Err(WorkspacePlanError::InvalidPath);
            }
        }
        if depth > MAX_WORKSPACE_PATH_DEPTH {
            return Err(WorkspacePlanError::InvalidPath);
        }
        Ok(Self(value))
    }

    fn parent(&self) -> Option<&str> {
        self.0.rsplit_once('/').map(|(parent, _)| parent)
    }
}

fn forbidden_path_character(value: char) -> bool {
    value.is_control()
        || matches!(
            value,
            '\u{00ad}'
                | '\u{061c}'
                | '\u{200b}'..='\u{200f}'
                | '\u{202a}'..='\u{202e}'
                | '\u{2060}'..='\u{206f}'
                | '\u{feff}'
        )
}

/// A validated proposal carries no descriptors or filesystem identity and is
/// therefore never executable or approvable on its own.
#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct ValidatedWorkspacePlanProposal {
    schema: u16,
    origin: WorkspacePlanOrigin,
    effects: Vec<ValidatedWorkspacePlanEffect>,
    proposal_sha256: String,
}

impl ValidatedWorkspacePlanProposal {
    pub fn schema(&self) -> u16 {
        self.schema
    }

    pub fn origin(&self) -> WorkspacePlanOrigin {
        self.origin
    }

    pub fn effects(&self) -> &[ValidatedWorkspacePlanEffect] {
        &self.effects
    }

    /// This digest binds only the non-authorizing proposal. The later prepared
    /// plan digest must additionally bind identities, authority and expiry.
    pub fn proposal_sha256(&self) -> &str {
        &self.proposal_sha256
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkspacePlanError {
    MessageTooLarge,
    Malformed,
    UnsupportedSchema,
    EmptyPlan,
    TooManyEffects,
    InvalidPath,
    DuplicateSource,
    DuplicateDestination,
    SourceDestinationOverlap,
    MissingEarlierDirectoryDependency,
    DigestEncoding,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
pub struct WorkspacePlanDirectoryIdentity {
    mount_id: u64,
    inode: u64,
}

impl WorkspacePlanDirectoryIdentity {
    pub fn mount_id(&self) -> u64 {
        self.mount_id
    }

    pub fn inode(&self) -> u64 {
        self.inode
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
pub struct WorkspacePlanFileIdentity {
    mount_id: u64,
    device_major: u32,
    device_minor: u32,
    inode: u64,
    size: u64,
    mtime_seconds: i64,
    mtime_nanoseconds: u32,
    ctime_seconds: i64,
    ctime_nanoseconds: u32,
    content_sha256: String,
}

impl WorkspacePlanFileIdentity {
    pub fn mount_id(&self) -> u64 {
        self.mount_id
    }

    pub fn inode(&self) -> u64 {
        self.inode
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn content_sha256(&self) -> &str {
        &self.content_sha256
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case", tag = "authority")]
pub enum WorkspacePlanParentAuthority {
    Existing { directory_index: usize },
    Planned { effect_index: usize },
}

#[derive(Debug, PartialEq, Eq)]
pub enum PreparedWorkspacePlanEffect {
    CreateDirectory {
        path: WorkspaceRelativePath,
        parent: WorkspacePlanParentAuthority,
        name: String,
    },
    MoveFile {
        source: WorkspaceRelativePath,
        source_parent_index: usize,
        source_name: String,
        destination: WorkspaceRelativePath,
        destination_parent: WorkspacePlanParentAuthority,
        destination_name: String,
        source_identity: WorkspacePlanFileIdentity,
    },
}

impl PreparedWorkspacePlanEffect {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::CreateDirectory { .. } => "create_directory",
            Self::MoveFile { .. } => "move_file",
        }
    }

    pub fn source(&self) -> Option<&WorkspaceRelativePath> {
        match self {
            Self::CreateDirectory { .. } => None,
            Self::MoveFile { source, .. } => Some(source),
        }
    }

    pub fn destination(&self) -> &WorkspaceRelativePath {
        match self {
            Self::CreateDirectory { path, .. } => path,
            Self::MoveFile { destination, .. } => destination,
        }
    }

    pub fn source_identity(&self) -> Option<&WorkspacePlanFileIdentity> {
        match self {
            Self::CreateDirectory { .. } => None,
            Self::MoveFile {
                source_identity, ..
            } => Some(source_identity),
        }
    }
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
#[derive(Debug)]
struct RetainedWorkspaceDirectory {
    relative_path: String,
    identity: WorkspacePlanDirectoryIdentity,
    descriptor: std::fs::File,
}

#[cfg(not(all(target_os = "linux", target_env = "gnu")))]
#[derive(Debug)]
struct RetainedWorkspaceDirectory;

/// Prepared authority is intentionally neither cloneable nor serializable.
/// Approval storage must own this value and execution must consume it by value.
#[derive(Debug)]
pub struct PreparedWorkspacePlan {
    workspace_root: String,
    workspace_identity: WorkspacePlanDirectoryIdentity,
    origin: WorkspacePlanOrigin,
    proposal_sha256: String,
    plan_sha256: String,
    created_at_ms: u64,
    expires_at_ms: u64,
    effects: Vec<PreparedWorkspacePlanEffect>,
    directories: Vec<RetainedWorkspaceDirectory>,
    _reservation: WorkspacePlanCapacityReservation,
}

impl PreparedWorkspacePlan {
    pub fn workspace_root(&self) -> &str {
        &self.workspace_root
    }

    pub fn workspace_identity(&self) -> &WorkspacePlanDirectoryIdentity {
        &self.workspace_identity
    }

    pub fn origin(&self) -> WorkspacePlanOrigin {
        self.origin
    }

    pub fn proposal_sha256(&self) -> &str {
        &self.proposal_sha256
    }

    pub fn plan_sha256(&self) -> &str {
        &self.plan_sha256
    }

    pub fn created_at_ms(&self) -> u64 {
        self.created_at_ms
    }

    pub fn expires_at_ms(&self) -> u64 {
        self.expires_at_ms
    }

    pub fn effects(&self) -> &[PreparedWorkspacePlanEffect] {
        &self.effects
    }

    pub fn retained_directory_count(&self) -> usize {
        self.directories.len()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkspacePlanPrepareError {
    UnsupportedPlatform,
    InvalidLifetime,
    InvalidWorkspaceRoot,
    WorkspaceOpenFailed,
    StatxUnavailable,
    PathResolutionFailed,
    SourceNotRegularFile,
    SourceReadFailed,
    DestinationExists,
    CrossMount,
    IdentityOverlap,
    TooManyFoldersSplitPlan,
    DigestEncoding,
}

impl WorkspacePlanPrepareError {
    pub fn user_message(self) -> &'static str {
        match self {
            Self::TooManyFoldersSplitPlan => "plan spans too many folders; split it",
            Self::IdentityOverlap => "plan touches the same file more than once",
            Self::SourceNotRegularFile => "plans may move regular files only",
            Self::DestinationExists => "a plan destination already exists",
            Self::CrossMount => "cross-filesystem moves are not supported",
            _ => "workspace plan preparation failed closed",
        }
    }
}

pub struct WorkspacePlanAuthorityResolver;

#[cfg(not(all(target_os = "linux", target_env = "gnu")))]
impl WorkspacePlanAuthorityResolver {
    pub fn prepare(
        _: &str,
        _: ValidatedWorkspacePlanProposal,
        reservation: WorkspacePlanCapacityReservation,
        _: u64,
        _: u64,
    ) -> Result<PreparedWorkspacePlan, WorkspacePlanPrepareError> {
        drop(reservation);
        Err(WorkspacePlanPrepareError::UnsupportedPlatform)
    }
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
fn linux_prepare_plan(
    workspace_root: &str,
    proposal: ValidatedWorkspacePlanProposal,
    reservation: WorkspacePlanCapacityReservation,
    created_at_ms: u64,
    expires_at_ms: u64,
) -> Result<PreparedWorkspacePlan, WorkspacePlanPrepareError> {
    use nix::fcntl::{OFlag, OpenHow, ResolveFlag, open, openat2};
    use nix::sys::stat::Mode;
    use std::fs::File;

    if expires_at_ms <= created_at_ms {
        return Err(WorkspacePlanPrepareError::InvalidLifetime);
    }
    crate::file_read::validate_selected_path(workspace_root)
        .map_err(|_| WorkspacePlanPrepareError::InvalidWorkspaceRoot)?;
    let relative_root = workspace_root
        .strip_prefix('/')
        .filter(|path| !path.is_empty())
        .ok_or(WorkspacePlanPrepareError::InvalidWorkspaceRoot)?;
    let slash = open(
        "/",
        OFlag::O_RDONLY | OFlag::O_DIRECTORY | OFlag::O_CLOEXEC,
        Mode::empty(),
    )
    .map_err(|_| WorkspacePlanPrepareError::WorkspaceOpenFailed)?;
    let root_how = OpenHow::new()
        .flags(OFlag::O_RDONLY | OFlag::O_DIRECTORY | OFlag::O_CLOEXEC)
        .resolve(
            ResolveFlag::RESOLVE_BENEATH
                | ResolveFlag::RESOLVE_NO_SYMLINKS
                | ResolveFlag::RESOLVE_NO_MAGICLINKS,
        );
    let root = File::from(
        openat2(&slash, relative_root, root_how)
            .map_err(|_| WorkspacePlanPrepareError::WorkspaceOpenFailed)?,
    );
    let workspace_identity = plan_directory_identity(&root)?;
    let mut directories = vec![RetainedWorkspaceDirectory {
        relative_path: String::new(),
        identity: workspace_identity.clone(),
        descriptor: root,
    }];
    let mut directory_indices = HashMap::from([(String::new(), 0usize)]);
    let mut planned_directories = HashMap::<String, usize>::new();
    let mut source_identities = HashSet::<(u64, u64)>::new();
    let mut prepared_effects = Vec::with_capacity(proposal.effects.len());

    for (effect_index, effect) in proposal.effects.iter().enumerate() {
        match effect {
            ValidatedWorkspacePlanEffect::CreateDirectory { path } => {
                let (parent_path, name) = split_relative(path.as_str());
                let parent = if let Some(index) = planned_directories.get(parent_path) {
                    WorkspacePlanParentAuthority::Planned {
                        effect_index: *index,
                    }
                } else {
                    let directory_index = retain_plan_directory(
                        parent_path,
                        &workspace_identity,
                        &mut directories,
                        &mut directory_indices,
                    )?;
                    ensure_destination_absent(&directories[directory_index].descriptor, name)?;
                    WorkspacePlanParentAuthority::Existing { directory_index }
                };
                planned_directories.insert(path.as_str().to_owned(), effect_index);
                prepared_effects.push(PreparedWorkspacePlanEffect::CreateDirectory {
                    path: WorkspaceRelativePath(path.as_str().to_owned()),
                    parent,
                    name: name.to_owned(),
                });
            }
            ValidatedWorkspacePlanEffect::MoveFile {
                source,
                destination,
            } => {
                let (source_parent, source_name) = split_relative(source.as_str());
                let source_parent_index = retain_plan_directory(
                    source_parent,
                    &workspace_identity,
                    &mut directories,
                    &mut directory_indices,
                )?;
                let source_identity = read_plan_file_identity(
                    &directories[source_parent_index].descriptor,
                    source_name,
                    workspace_identity.mount_id,
                )?;
                if !source_identities.insert((source_identity.mount_id, source_identity.inode)) {
                    return Err(WorkspacePlanPrepareError::IdentityOverlap);
                }

                let (destination_parent_path, destination_name) =
                    split_relative(destination.as_str());
                let destination_parent =
                    if let Some(index) = planned_directories.get(destination_parent_path) {
                        WorkspacePlanParentAuthority::Planned {
                            effect_index: *index,
                        }
                    } else {
                        let directory_index = retain_plan_directory(
                            destination_parent_path,
                            &workspace_identity,
                            &mut directories,
                            &mut directory_indices,
                        )?;
                        ensure_destination_absent(
                            &directories[directory_index].descriptor,
                            destination_name,
                        )?;
                        WorkspacePlanParentAuthority::Existing { directory_index }
                    };
                prepared_effects.push(PreparedWorkspacePlanEffect::MoveFile {
                    source: WorkspaceRelativePath(source.as_str().to_owned()),
                    source_parent_index,
                    source_name: source_name.to_owned(),
                    destination: WorkspaceRelativePath(destination.as_str().to_owned()),
                    destination_parent,
                    destination_name: destination_name.to_owned(),
                    source_identity,
                });
            }
        }
    }

    let digest_input = PreparedPlanDigestInput {
        workspace_root,
        workspace_identity: &workspace_identity,
        origin: proposal.origin,
        proposal_sha256: &proposal.proposal_sha256,
        created_at_ms,
        expires_at_ms,
    };
    let plan_sha256 = prepared_plan_digest(digest_input, &prepared_effects, &directories)?;
    Ok(PreparedWorkspacePlan {
        workspace_root: workspace_root.to_owned(),
        workspace_identity,
        origin: proposal.origin,
        proposal_sha256: proposal.proposal_sha256,
        plan_sha256,
        created_at_ms,
        expires_at_ms,
        effects: prepared_effects,
        directories,
        _reservation: reservation,
    })
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
fn split_relative(path: &str) -> (&str, &str) {
    path.rsplit_once('/').unwrap_or(("", path))
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
fn retain_plan_directory(
    relative_path: &str,
    workspace_identity: &WorkspacePlanDirectoryIdentity,
    directories: &mut Vec<RetainedWorkspaceDirectory>,
    indices: &mut HashMap<String, usize>,
) -> Result<usize, WorkspacePlanPrepareError> {
    use nix::fcntl::{OFlag, OpenHow, ResolveFlag, openat2};
    use std::fs::File;

    if let Some(index) = indices.get(relative_path) {
        return Ok(*index);
    }
    if directories.len() >= MAX_WORKSPACE_PLAN_DIRECTORY_AUTHORITIES {
        return Err(WorkspacePlanPrepareError::TooManyFoldersSplitPlan);
    }
    let how = OpenHow::new()
        .flags(OFlag::O_RDONLY | OFlag::O_DIRECTORY | OFlag::O_CLOEXEC)
        .resolve(
            ResolveFlag::RESOLVE_BENEATH
                | ResolveFlag::RESOLVE_NO_SYMLINKS
                | ResolveFlag::RESOLVE_NO_MAGICLINKS
                | ResolveFlag::RESOLVE_NO_XDEV,
        );
    let descriptor = File::from(
        openat2(&directories[0].descriptor, relative_path, how)
            .map_err(|_| WorkspacePlanPrepareError::PathResolutionFailed)?,
    );
    let identity = plan_directory_identity(&descriptor)?;
    if identity.mount_id != workspace_identity.mount_id {
        return Err(WorkspacePlanPrepareError::CrossMount);
    }
    let index = directories.len();
    directories.push(RetainedWorkspaceDirectory {
        relative_path: relative_path.to_owned(),
        identity,
        descriptor,
    });
    indices.insert(relative_path.to_owned(), index);
    Ok(index)
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
fn ensure_destination_absent(
    parent: &std::fs::File,
    name: &str,
) -> Result<(), WorkspacePlanPrepareError> {
    use nix::errno::Errno;
    use nix::fcntl::{OFlag, OpenHow, ResolveFlag, openat2};

    let how = OpenHow::new()
        .flags(OFlag::O_PATH | OFlag::O_CLOEXEC)
        .resolve(
            ResolveFlag::RESOLVE_BENEATH
                | ResolveFlag::RESOLVE_NO_SYMLINKS
                | ResolveFlag::RESOLVE_NO_MAGICLINKS
                | ResolveFlag::RESOLVE_NO_XDEV,
        );
    match openat2(parent, name, how) {
        Ok(_) => Err(WorkspacePlanPrepareError::DestinationExists),
        Err(Errno::ENOENT) => Ok(()),
        Err(_) => Err(WorkspacePlanPrepareError::PathResolutionFailed),
    }
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
fn plan_statx(file: &std::fs::File) -> Result<rustix::fs::Statx, WorkspacePlanPrepareError> {
    use rustix::fs::{AtFlags, StatxFlags, statx};

    let wanted = StatxFlags::TYPE
        | StatxFlags::INO
        | StatxFlags::SIZE
        | StatxFlags::MTIME
        | StatxFlags::CTIME
        | StatxFlags::MNT_ID;
    let value = statx(file, "", AtFlags::EMPTY_PATH, wanted)
        .map_err(|_| WorkspacePlanPrepareError::StatxUnavailable)?;
    if StatxFlags::from_bits_retain(value.stx_mask).contains(wanted) {
        Ok(value)
    } else {
        Err(WorkspacePlanPrepareError::StatxUnavailable)
    }
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
fn plan_directory_identity(
    file: &std::fs::File,
) -> Result<WorkspacePlanDirectoryIdentity, WorkspacePlanPrepareError> {
    let value = plan_statx(file)?;
    if u32::from(value.stx_mode) & libc::S_IFMT != libc::S_IFDIR || value.stx_mnt_id == 0 {
        return Err(WorkspacePlanPrepareError::PathResolutionFailed);
    }
    Ok(WorkspacePlanDirectoryIdentity {
        mount_id: value.stx_mnt_id,
        inode: value.stx_ino,
    })
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
fn read_plan_file_identity(
    parent: &std::fs::File,
    name: &str,
    expected_mount_id: u64,
) -> Result<WorkspacePlanFileIdentity, WorkspacePlanPrepareError> {
    use nix::fcntl::{OFlag, OpenHow, ResolveFlag, openat2};
    use std::fs::File;
    use std::io::Read;

    let how = OpenHow::new()
        .flags(OFlag::O_RDONLY | OFlag::O_CLOEXEC | OFlag::O_NONBLOCK)
        .resolve(
            ResolveFlag::RESOLVE_BENEATH
                | ResolveFlag::RESOLVE_NO_SYMLINKS
                | ResolveFlag::RESOLVE_NO_MAGICLINKS
                | ResolveFlag::RESOLVE_NO_XDEV,
        );
    let mut file = File::from(
        openat2(parent, name, how).map_err(|_| WorkspacePlanPrepareError::PathResolutionFailed)?,
    );
    let value = plan_statx(&file)?;
    if u32::from(value.stx_mode) & libc::S_IFMT != libc::S_IFREG {
        return Err(WorkspacePlanPrepareError::SourceNotRegularFile);
    }
    if value.stx_mnt_id != expected_mount_id {
        return Err(WorkspacePlanPrepareError::CrossMount);
    }
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|_| WorkspacePlanPrepareError::SourceReadFailed)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    let after_read = plan_statx(&file)?;
    if value.stx_mnt_id != after_read.stx_mnt_id
        || value.stx_ino != after_read.stx_ino
        || value.stx_size != after_read.stx_size
        || value.stx_mtime.tv_sec != after_read.stx_mtime.tv_sec
        || value.stx_mtime.tv_nsec != after_read.stx_mtime.tv_nsec
        || value.stx_ctime.tv_sec != after_read.stx_ctime.tv_sec
        || value.stx_ctime.tv_nsec != after_read.stx_ctime.tv_nsec
    {
        return Err(WorkspacePlanPrepareError::SourceReadFailed);
    }
    Ok(WorkspacePlanFileIdentity {
        mount_id: value.stx_mnt_id,
        device_major: value.stx_dev_major,
        device_minor: value.stx_dev_minor,
        inode: value.stx_ino,
        size: value.stx_size,
        mtime_seconds: value.stx_mtime.tv_sec,
        mtime_nanoseconds: value.stx_mtime.tv_nsec,
        ctime_seconds: value.stx_ctime.tv_sec,
        ctime_nanoseconds: value.stx_ctime.tv_nsec,
        content_sha256: hasher
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
    })
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
struct PreparedPlanDigestInput<'a> {
    workspace_root: &'a str,
    workspace_identity: &'a WorkspacePlanDirectoryIdentity,
    origin: WorkspacePlanOrigin,
    proposal_sha256: &'a str,
    created_at_ms: u64,
    expires_at_ms: u64,
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
fn prepared_plan_digest(
    input: PreparedPlanDigestInput<'_>,
    effects: &[PreparedWorkspacePlanEffect],
    directories: &[RetainedWorkspaceDirectory],
) -> Result<String, WorkspacePlanPrepareError> {
    #[derive(Serialize)]
    struct Directory<'a> {
        path: &'a str,
        identity: &'a WorkspacePlanDirectoryIdentity,
    }
    #[derive(Serialize)]
    #[serde(tag = "kind", rename_all = "snake_case")]
    enum Effect<'a> {
        CreateDirectory {
            path: &'a WorkspaceRelativePath,
            parent: WorkspacePlanParentAuthority,
            name: &'a str,
        },
        MoveFile {
            source: &'a WorkspaceRelativePath,
            source_parent_index: usize,
            source_name: &'a str,
            destination: &'a WorkspaceRelativePath,
            destination_parent: WorkspacePlanParentAuthority,
            destination_name: &'a str,
            source_identity: &'a WorkspacePlanFileIdentity,
        },
    }
    #[derive(Serialize)]
    struct Plan<'a> {
        schema: u16,
        workspace_root: &'a str,
        workspace_identity: &'a WorkspacePlanDirectoryIdentity,
        origin: WorkspacePlanOrigin,
        proposal_sha256: &'a str,
        created_at_ms: u64,
        expires_at_ms: u64,
        directories: Vec<Directory<'a>>,
        effects: Vec<Effect<'a>>,
    }

    let directories = directories
        .iter()
        .map(|directory| Directory {
            path: &directory.relative_path,
            identity: &directory.identity,
        })
        .collect();
    let effects = effects
        .iter()
        .map(|effect| match effect {
            PreparedWorkspacePlanEffect::CreateDirectory { path, parent, name } => {
                Effect::CreateDirectory {
                    path,
                    parent: *parent,
                    name,
                }
            }
            PreparedWorkspacePlanEffect::MoveFile {
                source,
                source_parent_index,
                source_name,
                destination,
                destination_parent,
                destination_name,
                source_identity,
            } => Effect::MoveFile {
                source,
                source_parent_index: *source_parent_index,
                source_name,
                destination,
                destination_parent: *destination_parent,
                destination_name,
                source_identity,
            },
        })
        .collect();
    let encoded = serde_json::to_vec(&Plan {
        schema: WORKSPACE_PLAN_SCHEMA_VERSION,
        workspace_root: input.workspace_root,
        workspace_identity: input.workspace_identity,
        origin: input.origin,
        proposal_sha256: input.proposal_sha256,
        created_at_ms: input.created_at_ms,
        expires_at_ms: input.expires_at_ms,
        directories,
        effects,
    })
    .map_err(|_| WorkspacePlanPrepareError::DigestEncoding)?;
    let digest = Sha256::digest(
        [
            b"blossom-prepared-workspace-plan-v1\0".as_slice(),
            encoded.as_slice(),
        ]
        .concat(),
    );
    Ok(digest.iter().map(|byte| format!("{byte:02x}")).collect())
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
impl WorkspacePlanAuthorityResolver {
    pub fn prepare(
        workspace_root: &str,
        proposal: ValidatedWorkspacePlanProposal,
        reservation: WorkspacePlanCapacityReservation,
        created_at_ms: u64,
        expires_at_ms: u64,
    ) -> Result<PreparedWorkspacePlan, WorkspacePlanPrepareError> {
        linux_prepare_plan(
            workspace_root,
            proposal,
            reservation,
            created_at_ms,
            expires_at_ms,
        )
    }
}

pub struct WorkspacePlanProposalResolver;

impl WorkspacePlanProposalResolver {
    pub fn resolve(
        input: &[u8],
        origin: WorkspacePlanOrigin,
    ) -> Result<ValidatedWorkspacePlanProposal, WorkspacePlanError> {
        if input.len() > MAX_WORKSPACE_PLAN_MESSAGE_BYTES {
            return Err(WorkspacePlanError::MessageTooLarge);
        }
        let wire: WorkspacePlanProposalWire =
            serde_json::from_slice(input).map_err(|_| WorkspacePlanError::Malformed)?;
        if wire.schema != WORKSPACE_PLAN_SCHEMA_VERSION {
            return Err(WorkspacePlanError::UnsupportedSchema);
        }
        if wire.effects.is_empty() {
            return Err(WorkspacePlanError::EmptyPlan);
        }
        if wire.effects.len() > MAX_WORKSPACE_PLAN_EFFECTS {
            return Err(WorkspacePlanError::TooManyEffects);
        }

        let planned_directories = wire
            .effects
            .iter()
            .filter_map(|effect| match effect {
                WorkspacePlanEffectWire::CreateDirectory { path } => Some(path.clone()),
                WorkspacePlanEffectWire::MoveFile { .. } => None,
            })
            .collect::<HashSet<_>>();
        let mut effects = Vec::with_capacity(wire.effects.len());
        let mut sources = HashSet::new();
        let mut destinations = HashSet::new();
        let mut created_directories = HashSet::new();

        for effect in wire.effects {
            let effect = match effect {
                WorkspacePlanEffectWire::CreateDirectory { path } => {
                    let path = WorkspaceRelativePath::parse(path)?;
                    require_parent_created_earlier_if_planned(
                        &path,
                        &planned_directories,
                        &created_directories,
                    )?;
                    if !destinations.insert(path.as_str().to_owned()) {
                        return Err(WorkspacePlanError::DuplicateDestination);
                    }
                    created_directories.insert(path.as_str().to_owned());
                    ValidatedWorkspacePlanEffect::CreateDirectory { path }
                }
                WorkspacePlanEffectWire::MoveFile {
                    source,
                    destination,
                } => {
                    let source = WorkspaceRelativePath::parse(source)?;
                    let destination = WorkspaceRelativePath::parse(destination)?;
                    if source == destination {
                        return Err(WorkspacePlanError::SourceDestinationOverlap);
                    }
                    require_parent_created_earlier_if_planned(
                        &destination,
                        &planned_directories,
                        &created_directories,
                    )?;
                    if !sources.insert(source.as_str().to_owned()) {
                        return Err(WorkspacePlanError::DuplicateSource);
                    }
                    if !destinations.insert(destination.as_str().to_owned()) {
                        return Err(WorkspacePlanError::DuplicateDestination);
                    }
                    ValidatedWorkspacePlanEffect::MoveFile {
                        source,
                        destination,
                    }
                }
            };
            effects.push(effect);
        }

        if sources.iter().any(|source| destinations.contains(source)) {
            return Err(WorkspacePlanError::SourceDestinationOverlap);
        }

        let proposal_sha256 = proposal_digest(origin, &effects)?;
        Ok(ValidatedWorkspacePlanProposal {
            schema: WORKSPACE_PLAN_SCHEMA_VERSION,
            origin,
            effects,
            proposal_sha256,
        })
    }
}

fn require_parent_created_earlier_if_planned(
    path: &WorkspaceRelativePath,
    planned_directories: &HashSet<String>,
    created_directories: &HashSet<String>,
) -> Result<(), WorkspacePlanError> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    let mut prefix = String::new();
    for component in parent.split('/') {
        if !prefix.is_empty() {
            prefix.push('/');
        }
        prefix.push_str(component);
        if planned_directories.contains(&prefix) && !created_directories.contains(&prefix) {
            return Err(WorkspacePlanError::MissingEarlierDirectoryDependency);
        }
    }
    Ok(())
}

fn proposal_digest(
    origin: WorkspacePlanOrigin,
    effects: &[ValidatedWorkspacePlanEffect],
) -> Result<String, WorkspacePlanError> {
    #[derive(Serialize)]
    struct CanonicalProposal<'a> {
        schema: u16,
        origin: WorkspacePlanOrigin,
        effects: &'a [ValidatedWorkspacePlanEffect],
    }

    let encoded = serde_json::to_vec(&CanonicalProposal {
        schema: WORKSPACE_PLAN_SCHEMA_VERSION,
        origin,
        effects,
    })
    .map_err(|_| WorkspacePlanError::DigestEncoding)?;
    let digest = Sha256::digest(
        [
            b"blossom-workspace-plan-proposal-v1\0".as_slice(),
            encoded.as_slice(),
        ]
        .concat(),
    );
    Ok(digest.iter().map(|byte| format!("{byte:02x}")).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    struct TestWorkspace(std::path::PathBuf);

    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    impl TestWorkspace {
        fn new() -> Self {
            use std::time::{SystemTime, UNIX_EPOCH};

            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "blossom-plan-resolver-{}-{nonce}",
                std::process::id()
            ));
            std::fs::create_dir(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &std::path::Path {
            &self.0
        }

        fn root(&self) -> &str {
            self.0.to_str().unwrap()
        }
    }

    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    impl Drop for TestWorkspace {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }

    fn resolve(
        value: serde_json::Value,
    ) -> Result<ValidatedWorkspacePlanProposal, WorkspacePlanError> {
        WorkspacePlanProposalResolver::resolve(
            &serde_json::to_vec(&value).unwrap(),
            WorkspacePlanOrigin::DeterministicFacts,
        )
    }

    #[test]
    fn accepts_only_the_closed_ordered_effect_set() {
        let plan = resolve(json!({
            "schema": 1,
            "effects": [
                {"kind": "create_directory", "path": "by-type"},
                {"kind": "create_directory", "path": "by-type/pdf"},
                {"kind": "move_file", "source": "inbox/report.pdf", "destination": "by-type/pdf/report.pdf"}
            ]
        }))
        .unwrap();

        assert_eq!(plan.schema(), WORKSPACE_PLAN_SCHEMA_VERSION);
        assert_eq!(plan.origin(), WorkspacePlanOrigin::DeterministicFacts);
        assert_eq!(plan.effects().len(), 3);
        assert_eq!(plan.effects()[0].kind(), "create_directory");
        assert_eq!(
            plan.effects()[2].source().unwrap().as_str(),
            "inbox/report.pdf"
        );
        assert_eq!(
            plan.effects()[2].destination().as_str(),
            "by-type/pdf/report.pdf"
        );
        assert_eq!(plan.proposal_sha256().len(), 64);
    }

    #[test]
    fn rejects_identity_root_digest_and_unknown_field_injection() {
        for injected in [
            json!({"schema": 1, "workspace_root": "/tmp", "effects": [{"kind": "create_directory", "path": "x"}]}),
            json!({"schema": 1, "effects": [{"kind": "create_directory", "path": "x", "device": 1}]}),
            json!({"schema": 1, "effects": [{"kind": "move_file", "source": "a", "destination": "b", "inode": 2}]}),
            json!({"schema": 1, "effects": [{"kind": "move_file", "source": "a", "destination": "b", "content_sha256": "0".repeat(64)}]}),
        ] {
            assert_eq!(resolve(injected), Err(WorkspacePlanError::Malformed));
        }
    }

    #[test]
    fn rejects_empty_oversized_and_unknown_plans() {
        assert_eq!(
            resolve(json!({"schema": 1, "effects": []})),
            Err(WorkspacePlanError::EmptyPlan)
        );
        assert_eq!(
            resolve(json!({"schema": 2, "effects": [{"kind": "create_directory", "path": "x"}]})),
            Err(WorkspacePlanError::UnsupportedSchema)
        );
        let effects = (0..=MAX_WORKSPACE_PLAN_EFFECTS)
            .map(|index| json!({"kind": "create_directory", "path": format!("d{index}")}))
            .collect::<Vec<_>>();
        assert_eq!(
            resolve(json!({"schema": 1, "effects": effects})),
            Err(WorkspacePlanError::TooManyEffects)
        );
        assert_eq!(
            resolve(json!({"schema": 1, "effects": [{"kind": "delete_file", "path": "x"}]})),
            Err(WorkspacePlanError::Malformed)
        );
        assert_eq!(
            WorkspacePlanProposalResolver::resolve(
                &vec![b' '; MAX_WORKSPACE_PLAN_MESSAGE_BYTES + 1],
                WorkspacePlanOrigin::UserRequested,
            ),
            Err(WorkspacePlanError::MessageTooLarge)
        );
    }

    #[test]
    fn rejects_unsafe_or_invisible_paths() {
        for path in [
            "",
            "/absolute",
            "../escape",
            "a/../escape",
            "a/./b",
            "a//b",
            "trailing/",
            "hidden\u{202e}name",
            "zero\u{200b}width",
            "line\nbreak",
        ] {
            assert_eq!(
                resolve(
                    json!({"schema": 1, "effects": [{"kind": "create_directory", "path": path}]})
                ),
                Err(WorkspacePlanError::InvalidPath),
                "{path:?}"
            );
        }
    }

    #[test]
    fn rejects_duplicate_and_overlapping_effects() {
        assert_eq!(
            resolve(json!({"schema": 1, "effects": [
                {"kind": "move_file", "source": "a", "destination": "x"},
                {"kind": "move_file", "source": "a", "destination": "y"}
            ]})),
            Err(WorkspacePlanError::DuplicateSource)
        );
        assert_eq!(
            resolve(json!({"schema": 1, "effects": [
                {"kind": "create_directory", "path": "x"},
                {"kind": "move_file", "source": "a", "destination": "x"}
            ]})),
            Err(WorkspacePlanError::DuplicateDestination)
        );
        assert_eq!(
            resolve(json!({"schema": 1, "effects": [
                {"kind": "move_file", "source": "a", "destination": "b"},
                {"kind": "move_file", "source": "b", "destination": "c"}
            ]})),
            Err(WorkspacePlanError::SourceDestinationOverlap)
        );
    }

    #[test]
    fn proposal_digest_binds_origin_order_kind_and_every_path() {
        let first = resolve(json!({"schema": 1, "effects": [
            {"kind": "create_directory", "path": "x"},
            {"kind": "move_file", "source": "a", "destination": "x/a"}
        ]}))
        .unwrap();
        let reordered = resolve(json!({"schema": 1, "effects": [
            {"kind": "move_file", "source": "b", "destination": "y/b"},
            {"kind": "move_file", "source": "a", "destination": "x/a"}
        ]}))
        .unwrap();
        let ordered_moves = resolve(json!({"schema": 1, "effects": [
            {"kind": "move_file", "source": "a", "destination": "x/a"},
            {"kind": "move_file", "source": "b", "destination": "y/b"}
        ]}))
        .unwrap();
        let changed = resolve(json!({"schema": 1, "effects": [
            {"kind": "create_directory", "path": "x"},
            {"kind": "move_file", "source": "a", "destination": "x/b"}
        ]}))
        .unwrap();
        let other_origin = WorkspacePlanProposalResolver::resolve(
            &serde_json::to_vec(&json!({"schema": 1, "effects": [
                {"kind": "create_directory", "path": "x"},
                {"kind": "move_file", "source": "a", "destination": "x/a"}
            ]}))
            .unwrap(),
            WorkspacePlanOrigin::ModelProposedScheme,
        )
        .unwrap();

        assert_ne!(ordered_moves.proposal_sha256(), reordered.proposal_sha256());
        assert_ne!(first.proposal_sha256(), changed.proposal_sha256());
        assert_ne!(first.proposal_sha256(), other_origin.proposal_sha256());
    }

    #[test]
    fn declared_directory_parents_must_appear_before_dependants() {
        assert_eq!(
            resolve(json!({"schema": 1, "effects": [
                {"kind": "move_file", "source": "a", "destination": "x/a"},
                {"kind": "create_directory", "path": "x"}
            ]})),
            Err(WorkspacePlanError::MissingEarlierDirectoryDependency)
        );
        assert_eq!(
            resolve(json!({"schema": 1, "effects": [
                {"kind": "create_directory", "path": "x/y"},
                {"kind": "create_directory", "path": "x"}
            ]})),
            Err(WorkspacePlanError::MissingEarlierDirectoryDependency)
        );
    }

    #[test]
    fn fifth_plan_is_rejected_and_drop_returns_capacity() {
        let capacity = WorkspacePlanCapacity::default();
        let mut reservations = Vec::new();
        for expected in 1..=MAX_PENDING_WORKSPACE_PLANS {
            reservations.push(capacity.reserve().unwrap());
            assert_eq!(capacity.in_use(), expected);
        }
        assert!(matches!(
            capacity.reserve(),
            Err(WorkspacePlanCapacityError::Exhausted)
        ));
        reservations.pop();
        assert_eq!(capacity.in_use(), MAX_PENDING_WORKSPACE_PLANS - 1);
        let replacement = capacity.reserve().unwrap();
        assert_eq!(capacity.in_use(), MAX_PENDING_WORKSPACE_PLANS);
        drop(replacement);
        drop(reservations);
        assert_eq!(capacity.in_use(), 0);
    }

    #[cfg(not(all(target_os = "linux", target_env = "gnu")))]
    #[test]
    fn unsupported_prepare_releases_its_capacity_reservation() {
        let capacity = WorkspacePlanCapacity::default();
        let reservation = capacity.reserve().unwrap();
        let proposal = resolve(json!({
            "schema": 1,
            "effects": [{"kind": "create_directory", "path": "sorted"}]
        }))
        .unwrap();
        assert!(matches!(
            WorkspacePlanAuthorityResolver::prepare("/tmp/workspace", proposal, reservation, 1, 2,),
            Err(WorkspacePlanPrepareError::UnsupportedPlatform)
        ));
        assert_eq!(capacity.in_use(), 0);
    }

    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    #[test]
    fn prepares_only_folder_authority_and_binds_source_identity() {
        let workspace = TestWorkspace::new();
        std::fs::create_dir(workspace.path().join("inbox")).unwrap();
        std::fs::write(workspace.path().join("inbox/report.txt"), b"evidence").unwrap();
        let proposal = resolve(json!({
            "schema": 1,
            "effects": [
                {"kind": "create_directory", "path": "sorted"},
                {"kind": "move_file", "source": "inbox/report.txt", "destination": "sorted/report.txt"}
            ]
        }))
        .unwrap();
        let capacity = WorkspacePlanCapacity::default();
        let prepared = WorkspacePlanAuthorityResolver::prepare(
            workspace.root(),
            proposal,
            capacity.reserve().unwrap(),
            10,
            20,
        )
        .unwrap();

        assert_eq!(prepared.retained_directory_count(), 2);
        assert_eq!(prepared.plan_sha256().len(), 64);
        assert_ne!(prepared.plan_sha256(), prepared.proposal_sha256());
        let PreparedWorkspacePlanEffect::MoveFile {
            destination_parent,
            source_identity,
            ..
        } = &prepared.effects()[1]
        else {
            panic!("expected prepared move");
        };
        assert_eq!(source_identity.size(), 8);
        assert_eq!(source_identity.content_sha256(), digest_bytes(b"evidence"));
        assert_eq!(
            *destination_parent,
            WorkspacePlanParentAuthority::Planned { effect_index: 0 }
        );
        assert_eq!(capacity.in_use(), 1);
        drop(prepared);
        assert_eq!(capacity.in_use(), 0);
    }

    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    #[test]
    fn rejects_hardlink_overlap_non_regular_sources_and_symlink_traversal() {
        let workspace = TestWorkspace::new();
        std::fs::create_dir(workspace.path().join("inbox")).unwrap();
        std::fs::create_dir(workspace.path().join("out")).unwrap();
        std::fs::write(workspace.path().join("inbox/a"), b"same inode").unwrap();
        std::fs::hard_link(
            workspace.path().join("inbox/a"),
            workspace.path().join("inbox/b"),
        )
        .unwrap();
        std::os::unix::fs::symlink("inbox", workspace.path().join("linked")).unwrap();
        std::fs::create_dir(workspace.path().join("directory-source")).unwrap();

        let cases = [
            (
                json!({"schema": 1, "effects": [
                    {"kind": "move_file", "source": "inbox/a", "destination": "out/a"},
                    {"kind": "move_file", "source": "inbox/b", "destination": "out/b"}
                ]}),
                WorkspacePlanPrepareError::IdentityOverlap,
            ),
            (
                json!({"schema": 1, "effects": [
                    {"kind": "move_file", "source": "directory-source", "destination": "out/d"}
                ]}),
                WorkspacePlanPrepareError::SourceNotRegularFile,
            ),
            (
                json!({"schema": 1, "effects": [
                    {"kind": "move_file", "source": "linked/a", "destination": "out/c"}
                ]}),
                WorkspacePlanPrepareError::PathResolutionFailed,
            ),
        ];
        for (value, expected) in cases {
            let capacity = WorkspacePlanCapacity::default();
            let result = WorkspacePlanAuthorityResolver::prepare(
                workspace.root(),
                resolve(value).unwrap(),
                capacity.reserve().unwrap(),
                10,
                20,
            );
            assert!(matches!(result, Err(actual) if actual == expected));
            assert_eq!(capacity.in_use(), 0);
        }
    }

    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    #[test]
    fn sixty_fifth_directory_authority_is_rejected_with_split_plan_reason() {
        let workspace = TestWorkspace::new();
        let mut effects = Vec::new();
        for index in 0..MAX_WORKSPACE_PLAN_DIRECTORY_AUTHORITIES {
            let directory = format!("source-{index}");
            std::fs::create_dir(workspace.path().join(&directory)).unwrap();
            std::fs::write(workspace.path().join(&directory).join("file"), b"x").unwrap();
            effects.push(json!({
                "kind": "move_file",
                "source": format!("{directory}/file"),
                "destination": format!("moved-{index}")
            }));
        }
        let capacity = WorkspacePlanCapacity::default();
        let result = WorkspacePlanAuthorityResolver::prepare(
            workspace.root(),
            resolve(json!({"schema": 1, "effects": effects})).unwrap(),
            capacity.reserve().unwrap(),
            10,
            20,
        );
        assert!(matches!(
            result,
            Err(WorkspacePlanPrepareError::TooManyFoldersSplitPlan)
        ));
        assert_eq!(
            WorkspacePlanPrepareError::TooManyFoldersSplitPlan.user_message(),
            "plan spans too many folders; split it"
        );
        assert_eq!(capacity.in_use(), 0);
    }

    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    fn digest_bytes(bytes: &[u8]) -> String {
        Sha256::digest(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }
}
