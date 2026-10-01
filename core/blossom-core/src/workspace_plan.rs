//! Closed, non-authorizing workspace organization proposals.
//!
//! This module validates untrusted plan shape before any filesystem authority is
//! opened. A validated proposal is still data: a later resolver must derive
//! identities, retain directory authority and build the approval-bound prepared
//! plan described by ADR-0036.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;

pub const WORKSPACE_PLAN_SCHEMA_VERSION: u16 = 1;
pub const MAX_WORKSPACE_PLAN_EFFECTS: usize = 128;
pub const MAX_WORKSPACE_PLAN_MESSAGE_BYTES: usize = 64 * 1024;
pub const MAX_WORKSPACE_RELATIVE_PATH_BYTES: usize = 1024;
pub const MAX_WORKSPACE_PATH_COMPONENT_BYTES: usize = 255;
pub const MAX_WORKSPACE_PATH_DEPTH: usize = 16;

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
}
