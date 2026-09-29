use crate::file_read::{MAX_FILE_CONTENT_BYTES, MAX_SELECTED_PATH_BYTES, validate_selected_path};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;

pub const WORKSPACE_FILE_MODE: u32 = 0o600;
pub const MAX_MODEL_WORKSPACE_CONTENT_BYTES: usize = 4096;
pub const MODEL_WORKSPACE_NAME_PATTERN: &str = "^[a-z0-9][a-z0-9._-]{0,63}$";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelWorkspaceCreateProposal {
    pub name: String,
    pub content: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelWorkspaceProposalError {
    InvalidName,
    InvalidContent,
    ResolutionFailed,
}

fn safe_model_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    (1..=64).contains(&bytes.len())
        && bytes
            .first()
            .is_some_and(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        && bytes[1..].iter().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
        })
}

fn unsafe_display_character(character: char) -> bool {
    character == '\0'
        || character == '\u{001b}'
        || matches!(
            character,
            '\u{061c}'
                | '\u{200b}'..='\u{200f}'
                | '\u{202a}'..='\u{202e}'
                | '\u{2060}'..='\u{206f}'
                | '\u{feff}'
        )
}

pub fn validate_model_workspace_proposal(
    proposal: &ModelWorkspaceCreateProposal,
) -> Result<(), ModelWorkspaceProposalError> {
    if !safe_model_name(&proposal.name) {
        return Err(ModelWorkspaceProposalError::InvalidName);
    }
    if proposal.content.len() > MAX_MODEL_WORKSPACE_CONTENT_BYTES
        || proposal.content.chars().any(unsafe_display_character)
    {
        return Err(ModelWorkspaceProposalError::InvalidContent);
    }
    Ok(())
}

pub fn model_workspace_proposal_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["name", "content"],
        "properties": {
            "name": {"type": "string", "pattern": MODEL_WORKSPACE_NAME_PATTERN, "maxLength": 64},
            "content": {"type": "string", "maxLength": MAX_MODEL_WORKSPACE_CONTENT_BYTES}
        }
    })
}

pub fn model_workspace_proposal_schema_bytes() -> Vec<u8> {
    serde_json::to_vec(&model_workspace_proposal_schema())
        .expect("closed workspace proposal schema must serialize")
}

pub fn model_workspace_constraint_digests() -> (String, String) {
    (
        digest(&model_workspace_proposal_schema_bytes()),
        digest(model_workspace_proposal_grammar().as_bytes()),
    )
}

pub fn model_workspace_proposal_grammar() -> String {
    format!(
        concat!(
            "root ::= proposal\n",
            "proposal ::= \"{{\\\"name\\\":\\\"\" name \"\\\",\\\"content\\\":\\\"\" content \"\\\"}}\"\n",
            "name ::= [a-z0-9] [a-z0-9._-]{{0,63}}\n",
            "content ::= content-char{{0,{}}}\n",
            "content-char ::= [ -!#-Z\\[\\]-~] | \"\\\\\\\"\" | \"\\\\\\\\\" | \"\\\\n\"\n"
        ),
        MAX_MODEL_WORKSPACE_CONTENT_BYTES
    )
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
pub fn resolve_model_workspace_proposal(
    session_workspace_root: &str,
    proposal: &ModelWorkspaceCreateProposal,
) -> Result<AtomicWorkspaceFileCreator, ModelWorkspaceProposalError> {
    validate_model_workspace_proposal(proposal)?;
    AtomicWorkspaceFileCreator::select(session_workspace_root, &proposal.name, &proposal.content)
        .map_err(|_| ModelWorkspaceProposalError::ResolutionFailed)
}

#[cfg(not(all(target_os = "linux", target_env = "gnu")))]
pub fn resolve_model_workspace_proposal(
    _: &str,
    proposal: &ModelWorkspaceCreateProposal,
) -> Result<AtomicWorkspaceFileCreator, ModelWorkspaceProposalError> {
    validate_model_workspace_proposal(proposal)?;
    Err(ModelWorkspaceProposalError::ResolutionFailed)
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
pub struct DirectoryIdentity {
    pub device: u64,
    pub inode: u64,
}

impl DirectoryIdentity {
    pub fn is_valid(&self) -> bool {
        self.device > 0 && self.inode > 0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
pub struct WorkspaceCreateSelection {
    pub workspace_root: String,
    pub root_identity: DirectoryIdentity,
    pub parent_identity: DirectoryIdentity,
    pub relative_destination: String,
    pub content: String,
    pub content_sha256: String,
    pub mode: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceCreateState {
    DurableCreated,
    PublishedDurabilityUncertain,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct WorkspaceFileCreated {
    pub workspace_root: String,
    pub relative_destination: String,
    pub root_identity: DirectoryIdentity,
    pub parent_identity: DirectoryIdentity,
    pub created_device: u64,
    pub created_inode: u64,
    pub source_bytes: usize,
    pub source_sha256: String,
    pub mode: u32,
    pub state: WorkspaceCreateState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceCreateError {
    UnsupportedPlatform,
    InvalidWorkspaceRoot,
    InvalidDestination,
    InvalidContent,
    SelectionFailed,
    DestinationExists,
    IdentityChanged,
    UnnamedCreateFailed,
    WriteFailed,
    VerificationFailed,
    PublishConflict,
    PublishFailed,
}

impl fmt::Display for WorkspaceCreateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::UnsupportedPlatform => {
                "workspace creation requires Linux openat2 and O_TMPFILE publication"
            }
            Self::InvalidWorkspaceRoot => "workspace root is not an exact absolute path",
            Self::InvalidDestination => "destination is not an exact relative file path",
            Self::InvalidContent => "content or its digest is invalid",
            Self::SelectionFailed => "workspace or parent selection failed",
            Self::DestinationExists => "the destination already exists",
            Self::IdentityChanged => "the retained workspace identity changed",
            Self::UnnamedCreateFailed => "the private unnamed file could not be created",
            Self::WriteFailed => "the private unnamed file could not be written durably",
            Self::VerificationFailed => "unnamed file verification failed",
            Self::PublishConflict => "the destination appeared before atomic publication",
            Self::PublishFailed => "atomic no-replace publication failed",
        })
    }
}

impl std::error::Error for WorkspaceCreateError {}

pub trait WorkspaceCreateProvider {
    fn selection(&self) -> &WorkspaceCreateSelection;
    fn create_selected_file(
        &mut self,
        expected: &WorkspaceCreateSelection,
    ) -> Result<WorkspaceFileCreated, WorkspaceCreateError>;
}

#[derive(Clone, Debug)]
pub struct UnavailableWorkspaceCreateProvider {
    selection: WorkspaceCreateSelection,
}

impl Default for UnavailableWorkspaceCreateProvider {
    fn default() -> Self {
        Self {
            selection: WorkspaceCreateSelection {
                workspace_root: "/unavailable".into(),
                root_identity: DirectoryIdentity {
                    device: 1,
                    inode: 1,
                },
                parent_identity: DirectoryIdentity {
                    device: 1,
                    inode: 1,
                },
                relative_destination: "unavailable".into(),
                content: String::new(),
                content_sha256: digest(&[]),
                mode: WORKSPACE_FILE_MODE,
            },
        }
    }
}

impl WorkspaceCreateProvider for UnavailableWorkspaceCreateProvider {
    fn selection(&self) -> &WorkspaceCreateSelection {
        &self.selection
    }
    fn create_selected_file(
        &mut self,
        _: &WorkspaceCreateSelection,
    ) -> Result<WorkspaceFileCreated, WorkspaceCreateError> {
        Err(WorkspaceCreateError::UnsupportedPlatform)
    }
}

pub fn validate_relative_destination(path: &str) -> Result<(), WorkspaceCreateError> {
    if path.is_empty()
        || path.len() > MAX_SELECTED_PATH_BYTES
        || path.starts_with('/')
        || path.ends_with('/')
        || path.contains('\0')
        || path.chars().any(char::is_control)
        || path
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(WorkspaceCreateError::InvalidDestination);
    }
    Ok(())
}

pub fn validate_workspace_selection(
    selection: &WorkspaceCreateSelection,
) -> Result<(), WorkspaceCreateError> {
    validate_selected_path(&selection.workspace_root)
        .map_err(|_| WorkspaceCreateError::InvalidWorkspaceRoot)?;
    validate_relative_destination(&selection.relative_destination)?;
    if !selection.root_identity.is_valid()
        || !selection.parent_identity.is_valid()
        || selection.root_identity.device != selection.parent_identity.device
        || selection.content.len() > MAX_FILE_CONTENT_BYTES
        || selection.mode != WORKSPACE_FILE_MODE
        || selection.content_sha256 != digest(selection.content.as_bytes())
    {
        return Err(WorkspaceCreateError::InvalidContent);
    }
    Ok(())
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
#[derive(Debug)]
pub struct AtomicWorkspaceFileCreator {
    selection: WorkspaceCreateSelection,
    root: std::fs::File,
    parent: std::fs::File,
}

#[cfg(not(all(target_os = "linux", target_env = "gnu")))]
#[derive(Clone, Copy, Debug, Default)]
pub struct AtomicWorkspaceFileCreator;

#[cfg(not(all(target_os = "linux", target_env = "gnu")))]
impl AtomicWorkspaceFileCreator {
    pub fn select(_: &str, _: &str, _: &str) -> Result<Self, WorkspaceCreateError> {
        Err(WorkspaceCreateError::UnsupportedPlatform)
    }
}

#[cfg(not(all(target_os = "linux", target_env = "gnu")))]
impl WorkspaceCreateProvider for AtomicWorkspaceFileCreator {
    fn selection(&self) -> &WorkspaceCreateSelection {
        panic!("Linux GNU-only provider cannot be constructed")
    }
    fn create_selected_file(
        &mut self,
        _: &WorkspaceCreateSelection,
    ) -> Result<WorkspaceFileCreated, WorkspaceCreateError> {
        Err(WorkspaceCreateError::UnsupportedPlatform)
    }
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
impl AtomicWorkspaceFileCreator {
    pub fn select(
        root_path: &str,
        destination: &str,
        content: &str,
    ) -> Result<Self, WorkspaceCreateError> {
        use nix::errno::Errno;
        use nix::fcntl::{OFlag, OpenHow, ResolveFlag, open, openat2};
        use nix::sys::stat::Mode;
        use std::fs::File;

        validate_selected_path(root_path)
            .map_err(|_| WorkspaceCreateError::InvalidWorkspaceRoot)?;
        validate_relative_destination(destination)?;
        if content.len() > MAX_FILE_CONTENT_BYTES {
            return Err(WorkspaceCreateError::InvalidContent);
        }
        let slash_root = open(
            "/",
            OFlag::O_RDONLY | OFlag::O_DIRECTORY | OFlag::O_CLOEXEC,
            Mode::empty(),
        )
        .map_err(|_| WorkspaceCreateError::SelectionFailed)?;
        let root_relative = root_path
            .strip_prefix('/')
            .ok_or(WorkspaceCreateError::InvalidWorkspaceRoot)?;
        let directory_how = OpenHow::new()
            .flags(OFlag::O_RDONLY | OFlag::O_DIRECTORY | OFlag::O_CLOEXEC)
            .resolve(
                ResolveFlag::RESOLVE_BENEATH
                    | ResolveFlag::RESOLVE_NO_SYMLINKS
                    | ResolveFlag::RESOLVE_NO_MAGICLINKS,
            );
        let root = File::from(
            openat2(&slash_root, root_relative, directory_how)
                .map_err(|_| WorkspaceCreateError::SelectionFailed)?,
        );
        let root_identity = directory_identity(&root)?;
        let (parent_path, final_name) = destination.rsplit_once('/').unwrap_or(("", destination));
        let parent = if parent_path.is_empty() {
            root.try_clone()
                .map_err(|_| WorkspaceCreateError::SelectionFailed)?
        } else {
            let parent_how = OpenHow::new()
                .flags(OFlag::O_RDONLY | OFlag::O_DIRECTORY | OFlag::O_CLOEXEC)
                .resolve(
                    ResolveFlag::RESOLVE_BENEATH
                        | ResolveFlag::RESOLVE_NO_SYMLINKS
                        | ResolveFlag::RESOLVE_NO_MAGICLINKS
                        | ResolveFlag::RESOLVE_NO_XDEV,
                );
            File::from(
                openat2(&root, parent_path, parent_how)
                    .map_err(|_| WorkspaceCreateError::SelectionFailed)?,
            )
        };
        let parent_identity = directory_identity(&parent)?;
        if parent_identity.device != root_identity.device {
            return Err(WorkspaceCreateError::SelectionFailed);
        }
        let probe_how = OpenHow::new()
            .flags(OFlag::O_RDONLY | OFlag::O_CLOEXEC | OFlag::O_NONBLOCK)
            .resolve(
                ResolveFlag::RESOLVE_BENEATH
                    | ResolveFlag::RESOLVE_NO_SYMLINKS
                    | ResolveFlag::RESOLVE_NO_MAGICLINKS
                    | ResolveFlag::RESOLVE_NO_XDEV,
            );
        match openat2(&parent, final_name, probe_how) {
            Ok(_) => return Err(WorkspaceCreateError::DestinationExists),
            Err(Errno::ENOENT) => {}
            Err(_) => return Err(WorkspaceCreateError::SelectionFailed),
        }
        let selection = WorkspaceCreateSelection {
            workspace_root: root_path.into(),
            root_identity,
            parent_identity,
            relative_destination: destination.into(),
            content: content.into(),
            content_sha256: digest(content.as_bytes()),
            mode: WORKSPACE_FILE_MODE,
        };
        Ok(Self {
            selection,
            root,
            parent,
        })
    }

    fn create_with_directory_sync(
        &mut self,
        expected: &WorkspaceCreateSelection,
        sync_directory: bool,
    ) -> Result<WorkspaceFileCreated, WorkspaceCreateError> {
        use nix::errno::Errno;
        use nix::fcntl::{AtFlags, OFlag, openat};
        use nix::sys::stat::{Mode, fchmod};
        use nix::unistd::{fsync, linkat};
        use std::fs::File;
        use std::io::{Read, Seek, SeekFrom, Write};

        if expected != &self.selection
            || validate_workspace_selection(expected).is_err()
            || directory_identity(&self.root)? != expected.root_identity
            || directory_identity(&self.parent)? != expected.parent_identity
        {
            return Err(WorkspaceCreateError::IdentityChanged);
        }
        let final_name = expected
            .relative_destination
            .rsplit('/')
            .next()
            .ok_or(WorkspaceCreateError::InvalidDestination)?;
        let mut temp = File::from(
            openat(
                &self.parent,
                ".",
                OFlag::O_RDWR | OFlag::O_TMPFILE | OFlag::O_CLOEXEC,
                Mode::S_IRUSR | Mode::S_IWUSR,
            )
            .map_err(|_| WorkspaceCreateError::UnnamedCreateFailed)?,
        );
        let prepublish = (|| -> Result<(u64, u64), WorkspaceCreateError> {
            fchmod(&temp, Mode::S_IRUSR | Mode::S_IWUSR)
                .map_err(|_| WorkspaceCreateError::WriteFailed)?;
            temp.write_all(expected.content.as_bytes())
                .map_err(|_| WorkspaceCreateError::WriteFailed)?;
            fsync(&temp).map_err(|_| WorkspaceCreateError::WriteFailed)?;
            temp.seek(SeekFrom::Start(0))
                .map_err(|_| WorkspaceCreateError::VerificationFailed)?;
            let mut bytes = Vec::new();
            Read::by_ref(&mut temp)
                .take((MAX_FILE_CONTENT_BYTES + 1) as u64)
                .read_to_end(&mut bytes)
                .map_err(|_| WorkspaceCreateError::VerificationFailed)?;
            let metadata = temp
                .metadata()
                .map_err(|_| WorkspaceCreateError::VerificationFailed)?;
            use std::os::unix::fs::{MetadataExt, PermissionsExt};
            if !metadata.file_type().is_file()
                || bytes.len() > MAX_FILE_CONTENT_BYTES
                || bytes != expected.content.as_bytes()
                || digest(&bytes) != expected.content_sha256
                || metadata.permissions().mode() & 0o777 != WORKSPACE_FILE_MODE
            {
                return Err(WorkspaceCreateError::VerificationFailed);
            }
            Ok((metadata.dev(), metadata.ino()))
        })();
        let (created_device, created_inode) = prepublish?;
        match linkat(&temp, "", &self.parent, final_name, AtFlags::AT_EMPTY_PATH) {
            Ok(()) => {}
            Err(error) => {
                return Err(if error == Errno::EEXIST {
                    WorkspaceCreateError::PublishConflict
                } else {
                    WorkspaceCreateError::PublishFailed
                });
            }
        }
        let state = if sync_directory && fsync(&self.parent).is_ok() {
            WorkspaceCreateState::DurableCreated
        } else {
            WorkspaceCreateState::PublishedDurabilityUncertain
        };
        Ok(WorkspaceFileCreated {
            workspace_root: expected.workspace_root.clone(),
            relative_destination: expected.relative_destination.clone(),
            root_identity: expected.root_identity.clone(),
            parent_identity: expected.parent_identity.clone(),
            created_device,
            created_inode,
            source_bytes: expected.content.len(),
            source_sha256: expected.content_sha256.clone(),
            mode: WORKSPACE_FILE_MODE,
            state,
        })
    }
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
impl WorkspaceCreateProvider for AtomicWorkspaceFileCreator {
    fn selection(&self) -> &WorkspaceCreateSelection {
        &self.selection
    }
    fn create_selected_file(
        &mut self,
        expected: &WorkspaceCreateSelection,
    ) -> Result<WorkspaceFileCreated, WorkspaceCreateError> {
        self.create_with_directory_sync(expected, true)
    }
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
fn directory_identity(file: &std::fs::File) -> Result<DirectoryIdentity, WorkspaceCreateError> {
    use std::os::unix::fs::MetadataExt;
    let metadata = file
        .metadata()
        .map_err(|_| WorkspaceCreateError::SelectionFailed)?;
    if !metadata.file_type().is_dir() {
        return Err(WorkspaceCreateError::SelectionFailed);
    }
    Ok(DirectoryIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
    })
}

pub fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_exact_relative_destinations_and_bound_selection() {
        assert_eq!(validate_relative_destination("docs/note.txt"), Ok(()));
        for invalid in [
            "",
            "/absolute",
            "../escape",
            "a/../b",
            "a//b",
            "a/./b",
            "file/",
            "bad\nname",
        ] {
            assert_eq!(
                validate_relative_destination(invalid),
                Err(WorkspaceCreateError::InvalidDestination)
            );
        }
    }

    #[test]
    fn model_workspace_proposal_is_one_safe_component_with_visible_content() {
        let valid = ModelWorkspaceCreateProposal {
            name: "agent-note_1.txt".into(),
            content: "bounded printable content\n".into(),
        };
        assert_eq!(validate_model_workspace_proposal(&valid), Ok(()));
        let invalid_names = vec![
            "".to_string(),
            "../x".into(),
            "/x".into(),
            "a/b".into(),
            "Ä.txt".into(),
            "-hidden".into(),
            "a".repeat(65),
        ];
        for name in invalid_names {
            let proposal = ModelWorkspaceCreateProposal {
                name,
                content: "safe".into(),
            };
            assert_eq!(
                validate_model_workspace_proposal(&proposal),
                Err(ModelWorkspaceProposalError::InvalidName)
            );
        }
        for content in [
            "nul\0byte",
            "escape\u{1b}[2J",
            "bidi\u{202e}txt",
            "zero\u{200b}width",
        ] {
            let proposal = ModelWorkspaceCreateProposal {
                name: "note.txt".into(),
                content: content.into(),
            };
            assert_eq!(
                validate_model_workspace_proposal(&proposal),
                Err(ModelWorkspaceProposalError::InvalidContent)
            );
        }
        let oversized = ModelWorkspaceCreateProposal {
            name: "note.txt".into(),
            content: "x".repeat(MAX_MODEL_WORKSPACE_CONTENT_BYTES + 1),
        };
        assert_eq!(
            validate_model_workspace_proposal(&oversized),
            Err(ModelWorkspaceProposalError::InvalidContent)
        );

        let escaped_bidi: ModelWorkspaceCreateProposal =
            serde_json::from_str(r#"{"name":"note.txt","content":"hidden\u202eexe"}"#)
                .expect("valid JSON decodes the escape");
        assert_eq!(
            validate_model_workspace_proposal(&escaped_bidi),
            Err(ModelWorkspaceProposalError::InvalidContent)
        );
    }

    #[test]
    fn proposal_grammar_is_a_single_object_subset_of_the_validator() {
        let grammar = model_workspace_proposal_grammar();
        assert!(grammar.starts_with("root ::= proposal\n"));
        assert!(!grammar.contains("\\\\u"));
        assert!(grammar.contains("content-char{0,4096}"));
        assert_eq!(
            model_workspace_proposal_schema()["properties"]["name"]["pattern"],
            MODEL_WORKSPACE_NAME_PATTERN
        );
        for (name, content) in [
            ("a", ""),
            ("note.txt", "printable ASCII"),
            ("agent_1.md", "line one\nline two"),
            ("x-2", "quotes \" and slash \\"),
        ] {
            assert_eq!(
                validate_model_workspace_proposal(&ModelWorkspaceCreateProposal {
                    name: name.into(),
                    content: content.into(),
                }),
                Ok(())
            );
        }
    }

    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    mod linux {
        use super::*;
        use std::fs;
        use std::os::unix::fs::{PermissionsExt, symlink};
        use std::path::PathBuf;

        struct Root(PathBuf);
        impl Root {
            fn new(label: &str) -> Self {
                let path = std::env::temp_dir().join(format!(
                    "blossom-workspace-{label}-{}-{}",
                    std::process::id(),
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .expect("clock")
                        .as_nanos()
                ));
                fs::create_dir(&path).expect("root");
                Self(path)
            }
            fn path(&self) -> &str {
                self.0.to_str().expect("path")
            }
        }
        impl Drop for Root {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.0);
            }
        }

        #[test]
        fn creates_verified_private_file_and_never_overwrites_existing_destination() {
            let root = Root::new("create");
            fs::create_dir(root.0.join("docs")).expect("parent");
            let mut creator =
                AtomicWorkspaceFileCreator::select(root.path(), "docs/new.txt", "hello")
                    .expect("selection");
            let selection = creator.selection().clone();
            let result = creator.create_selected_file(&selection).expect("create");
            assert_eq!(result.state, WorkspaceCreateState::DurableCreated);
            assert_eq!(
                fs::read_to_string(root.0.join("docs/new.txt")).expect("content"),
                "hello"
            );
            assert_eq!(
                fs::metadata(root.0.join("docs/new.txt"))
                    .expect("metadata")
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
            assert_eq!(
                AtomicWorkspaceFileCreator::select(root.path(), "docs/new.txt", "replacement")
                    .unwrap_err(),
                WorkspaceCreateError::DestinationExists
            );
            assert_eq!(
                fs::read_to_string(root.0.join("docs/new.txt")).expect("unchanged"),
                "hello"
            );
        }

        #[test]
        fn destination_race_fails_without_overwrite_and_cleans_private_temp() {
            let root = Root::new("race");
            fs::create_dir(root.0.join("docs")).expect("parent");
            let mut creator =
                AtomicWorkspaceFileCreator::select(root.path(), "docs/new.txt", "approved")
                    .expect("selection");
            let selection = creator.selection().clone();
            fs::write(root.0.join("docs/new.txt"), "raced-in").expect("race");
            assert_eq!(
                creator.create_selected_file(&selection),
                Err(WorkspaceCreateError::PublishConflict)
            );
            assert_eq!(
                fs::read_to_string(root.0.join("docs/new.txt")).expect("raced file"),
                "raced-in"
            );
            assert!(
                fs::read_dir(root.0.join("docs"))
                    .expect("entries")
                    .all(|entry| !entry
                        .expect("entry")
                        .file_name()
                        .to_string_lossy()
                        .starts_with(".blossom-tmp-"))
            );
        }

        #[test]
        fn rejects_root_parent_and_final_symlinks() {
            let root = Root::new("symlinks");
            let real = root.0.join("real");
            fs::create_dir(&real).expect("real");
            fs::create_dir(real.join("parent")).expect("parent");
            symlink(&real, root.0.join("root-link")).expect("root link");
            assert!(
                AtomicWorkspaceFileCreator::select(
                    root.0.join("root-link").to_str().expect("path"),
                    "new.txt",
                    "x"
                )
                .is_err()
            );
            symlink(real.join("parent"), real.join("parent-link")).expect("parent link");
            assert!(
                AtomicWorkspaceFileCreator::select(
                    real.to_str().expect("path"),
                    "parent-link/new.txt",
                    "x"
                )
                .is_err()
            );
            symlink("missing", real.join("new.txt")).expect("final link");
            assert!(
                AtomicWorkspaceFileCreator::select(real.to_str().expect("path"), "new.txt", "x")
                    .is_err()
            );
        }

        #[test]
        fn unnamed_publication_creates_no_visible_temporary_path() {
            let root = Root::new("unnamed");
            let mut creator =
                AtomicWorkspaceFileCreator::select(root.path(), "new.txt", "x").expect("selection");
            let selection = creator.selection().clone();
            creator.create_selected_file(&selection).expect("create");
            let names = fs::read_dir(&root.0)
                .expect("directory")
                .map(|entry| {
                    entry
                        .expect("entry")
                        .file_name()
                        .to_string_lossy()
                        .into_owned()
                })
                .collect::<Vec<_>>();
            assert_eq!(names, vec!["new.txt"]);
        }

        #[test]
        fn retained_parent_cannot_be_redirected_and_uncertain_durability_is_explicit() {
            let root = Root::new("retained");
            fs::create_dir(root.0.join("parent")).expect("parent");
            let mut creator =
                AtomicWorkspaceFileCreator::select(root.path(), "parent/new.txt", "approved")
                    .expect("selection");
            let selection = creator.selection().clone();
            fs::rename(root.0.join("parent"), root.0.join("moved-parent"))
                .expect("move selected parent");
            fs::create_dir(root.0.join("parent")).expect("replacement parent");
            let result = creator
                .create_with_directory_sync(&selection, false)
                .expect("published result");
            assert_eq!(
                result.state,
                WorkspaceCreateState::PublishedDurabilityUncertain
            );
            assert_eq!(
                fs::read_to_string(root.0.join("moved-parent/new.txt"))
                    .expect("selected parent content"),
                "approved"
            );
            assert!(!root.0.join("parent/new.txt").exists());
        }
    }
}
