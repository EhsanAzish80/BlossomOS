use crate::{
    DirectWorkspaceCreateParse, ModelWorkspaceCreateProposal, ModelWorkspaceProposalError,
    ShellPeerId, parse_obvious_workspace_create,
};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

pub const CREATE_FILE_TEMPLATE: &str = "create  containing ";
pub const UNSUPPORTED_RESPONSE: &str =
    "Blossom can't do this yet. Today it can create one file in your workspace.";
pub const MAX_COMMAND_QUERY_BYTES: usize = 4096;
pub const MAX_COMMAND_RESULTS: usize = 12;
pub const MAX_COMMAND_PEERS: usize = 32;
pub const MAX_DESKTOP_ENTRIES: usize = 512;
pub const MAX_DESKTOP_ENTRY_BYTES: u64 = 64 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApplicationOrigin {
    System,
    UserInstalled,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalApplication {
    pub desktop_id: String,
    pub name: String,
    match_names: Vec<String>,
    desktop_entry_path: String,
    desktop_entry_sha256: String,
    launch_argv: Vec<String>,
    origin: ApplicationOrigin,
}

impl LocalApplication {
    pub fn new(
        desktop_id: String,
        name: String,
        desktop_entry_path: String,
        desktop_entry_bytes: &[u8],
        origin: ApplicationOrigin,
    ) -> Option<Self> {
        if desktop_id.is_empty()
            || name.is_empty()
            || desktop_id.len() > 255
            || name.len() > 255
            || !desktop_entry_path.starts_with('/')
            || !desktop_entry_path.ends_with(".desktop")
            || desktop_entry_path.contains('\0')
            || desktop_entry_bytes.is_empty()
        {
            return None;
        }
        Some(Self {
            desktop_id,
            match_names: vec![name.clone()],
            name,
            desktop_entry_path,
            desktop_entry_sha256: sha256(desktop_entry_bytes),
            launch_argv: Vec::new(),
            origin,
        })
    }

    pub fn origin(&self) -> ApplicationOrigin {
        self.origin
    }

    pub fn launch_argv(&self) -> &[String] {
        &self.launch_argv
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DesktopEntryError {
    InvalidEncoding,
    InvalidEntry,
    Hidden,
    ExcludedDesktop,
    UnsupportedLaunchMode,
    MissingExecutable,
}

/// Reads a bounded snapshot of application entries. Callers retain the result
/// and rescan only when their directory watcher reports a change.
pub fn discover_local_applications(
    directories: &[(PathBuf, ApplicationOrigin)],
    locale: &str,
    current_desktop: &str,
    path: &str,
) -> Vec<LocalApplication> {
    let mut applications = Vec::new();
    let mut scanned = 0usize;
    let mut seen_ids = HashSet::new();
    for (directory, origin) in directories {
        let Ok(entries) = std::fs::read_dir(directory) else {
            continue;
        };
        let mut entries = entries.flatten().collect::<Vec<_>>();
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            if scanned >= MAX_DESKTOP_ENTRIES {
                return applications;
            }
            scanned += 1;
            let file_name = entry.file_name();
            let Some(desktop_id) = file_name.to_str() else {
                continue;
            };
            if !desktop_id.ends_with(".desktop") {
                continue;
            }
            if seen_ids.contains(desktop_id) {
                continue;
            }
            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            if !metadata.is_file() || metadata.len() > MAX_DESKTOP_ENTRY_BYTES {
                continue;
            }
            let Ok(bytes) = std::fs::read(entry.path()) else {
                continue;
            };
            if let Ok(application) = parse_desktop_entry(
                desktop_id,
                &entry.path(),
                &bytes,
                *origin,
                locale,
                current_desktop,
                |program| executable_exists(program, path),
            ) {
                seen_ids.insert(desktop_id.to_owned());
                applications.push(application);
            }
        }
    }
    applications
}

pub fn parse_desktop_entry(
    desktop_id: &str,
    entry_path: &Path,
    bytes: &[u8],
    origin: ApplicationOrigin,
    locale: &str,
    current_desktop: &str,
    executable_exists: impl Fn(&str) -> bool,
) -> Result<LocalApplication, DesktopEntryError> {
    if bytes.is_empty() || bytes.len() as u64 > MAX_DESKTOP_ENTRY_BYTES {
        return Err(DesktopEntryError::InvalidEntry);
    }
    let text = std::str::from_utf8(bytes).map_err(|_| DesktopEntryError::InvalidEncoding)?;
    let mut values = HashMap::<String, String>::new();
    let mut in_desktop_entry = false;
    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            in_desktop_entry = line == "[Desktop Entry]";
            continue;
        }
        if !in_desktop_entry || line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(DesktopEntryError::InvalidEntry);
        };
        values.entry(key.into()).or_insert_with(|| value.into());
    }
    if values.get("Type").map(String::as_str) != Some("Application")
        || truthy(values.get("Hidden"))
        || truthy(values.get("NoDisplay"))
    {
        return Err(DesktopEntryError::Hidden);
    }
    if truthy(values.get("Terminal")) || truthy(values.get("DBusActivatable")) {
        return Err(DesktopEntryError::UnsupportedLaunchMode);
    }
    if !desktop_allowed(&values, current_desktop) {
        return Err(DesktopEntryError::ExcludedDesktop);
    }
    if let Some(try_exec) = values.get("TryExec")
        && (try_exec.is_empty() || !executable_exists(try_exec))
    {
        return Err(DesktopEntryError::MissingExecutable);
    }
    let plain_name = values
        .get("Name")
        .filter(|value| valid_display_name(value))
        .ok_or(DesktopEntryError::InvalidEntry)?;
    let display_name = localized_name(&values, locale).unwrap_or(plain_name);
    let exec = values.get("Exec").ok_or(DesktopEntryError::InvalidEntry)?;
    let launch_argv = parse_exec(exec)?;
    if launch_argv.is_empty() || !executable_exists(&launch_argv[0]) {
        return Err(DesktopEntryError::MissingExecutable);
    }
    let path = entry_path
        .to_str()
        .filter(|value| value.starts_with('/') && value.ends_with(".desktop"))
        .ok_or(DesktopEntryError::InvalidEntry)?;
    let mut match_names = vec![plain_name.clone()];
    if display_name != plain_name {
        match_names.push(display_name.clone());
    }
    Ok(LocalApplication {
        desktop_id: desktop_id.into(),
        name: display_name.clone(),
        match_names,
        desktop_entry_path: path.into(),
        desktop_entry_sha256: sha256(bytes),
        launch_argv,
        origin,
    })
}

fn truthy(value: Option<&String>) -> bool {
    value.is_some_and(|value| value.eq_ignore_ascii_case("true"))
}

fn desktop_allowed(values: &HashMap<String, String>, current_desktop: &str) -> bool {
    let current = current_desktop
        .split(':')
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    let list = |key: &str| {
        values
            .get(key)
            .map(|value| {
                value
                    .split(';')
                    .filter(|item| !item.is_empty())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    };
    let only = list("OnlyShowIn");
    let excluded = list("NotShowIn");
    (only.is_empty() || only.iter().any(|item| current.contains(item)))
        && !excluded.iter().any(|item| current.contains(item))
}

fn localized_name<'a>(values: &'a HashMap<String, String>, locale: &str) -> Option<&'a String> {
    let normalized = locale.split('.').next().unwrap_or(locale);
    let language = normalized.split('_').next().unwrap_or(normalized);
    [format!("Name[{normalized}]"), format!("Name[{language}]")]
        .into_iter()
        .find_map(|key| values.get(&key).filter(|value| valid_display_name(value)))
}

fn valid_display_name(value: &str) -> bool {
    !value.is_empty() && value.len() <= 255 && !value.chars().any(char::is_control)
}

fn parse_exec(value: &str) -> Result<Vec<String>, DesktopEntryError> {
    let mut arguments = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut escaped = false;
    let mut chars = value.chars().peekable();
    while let Some(character) = chars.next() {
        if escaped {
            current.push(character);
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character == '"' {
            quoted = !quoted;
        } else if character == '%' {
            let Some(code) = chars.next() else {
                return Err(DesktopEntryError::InvalidEntry);
            };
            if code == '%' {
                current.push('%');
            } else if !code.is_ascii_alphabetic() {
                return Err(DesktopEntryError::InvalidEntry);
            }
        } else if character.is_whitespace() && !quoted {
            if !current.is_empty() {
                arguments.push(std::mem::take(&mut current));
            }
        } else if character.is_control() {
            return Err(DesktopEntryError::InvalidEntry);
        } else {
            current.push(character);
        }
    }
    if quoted || escaped {
        return Err(DesktopEntryError::InvalidEntry);
    }
    if !current.is_empty() {
        arguments.push(current);
    }
    Ok(arguments)
}

fn executable_exists(program: &str, path: &str) -> bool {
    if program.contains('/') {
        return is_executable_file(Path::new(program));
    }
    path.split(':')
        .filter(|directory| !directory.is_empty())
        .any(|directory| is_executable_file(&Path::new(directory).join(program)))
}

fn is_executable_file(path: &Path) -> bool {
    let Ok(metadata) = path.metadata() else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceFile {
    pub relative_path: String,
    pub display_name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandRow {
    OpenApplication(LocalApplication),
    OpenWorkspaceFile(WorkspaceFile),
    CreateWorkspaceFile(ModelWorkspaceCreateProposal),
    AskBlossom { prompt: String },
    InvalidCreate(ModelWorkspaceProposalError),
    Unsupported { message: &'static str },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandRoute {
    pub rows: Vec<CommandRow>,
    pub selected_index: Option<usize>,
}

impl CommandRoute {
    fn from_rows(rows: Vec<CommandRow>) -> Self {
        Self {
            selected_index: (!rows.is_empty()).then_some(0),
            rows,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandActivation {
    OpenApplication {
        desktop_id: String,
        desktop_entry_path: String,
        desktop_entry_sha256: String,
        origin: ApplicationOrigin,
        launch_argv: Vec<String>,
    },
    OpenWorkspaceFile {
        relative_path: String,
    },
    BeginWorkspaceCreate(ModelWorkspaceCreateProposal),
    BeginAgentTurn {
        prompt: String,
    },
    ShowInvalidCreate(ModelWorkspaceProposalError),
    ShowUnsupported {
        message: &'static str,
    },
}

/// Produces display rows only. Model invocation is deliberately absent from
/// routing and can occur only after activating an `AskBlossom` row.
pub fn route_command(
    query: &str,
    applications: &[LocalApplication],
    workspace_files: &[WorkspaceFile],
) -> CommandRoute {
    let query = query.trim();
    if query.is_empty() {
        return CommandRoute::from_rows(Vec::new());
    }

    let normalized_query = query.to_ascii_lowercase();
    let mut applications = applications
        .iter()
        .filter(|application| {
            application
                .match_names
                .iter()
                .any(|name| normalized_match(name, &normalized_query))
        })
        .cloned()
        .collect::<Vec<_>>();
    applications.sort_by(|left, right| {
        (left.origin != ApplicationOrigin::System)
            .cmp(&(right.origin != ApplicationOrigin::System))
            .then_with(|| {
                left.name
                    .to_ascii_lowercase()
                    .cmp(&right.name.to_ascii_lowercase())
            })
            .then_with(|| left.desktop_id.cmp(&right.desktop_id))
    });

    let mut workspace_files = workspace_files
        .iter()
        .filter(|file| normalized_match(&file.display_name, &normalized_query))
        .cloned()
        .collect::<Vec<_>>();
    workspace_files.sort_by(|left, right| {
        left.display_name
            .to_ascii_lowercase()
            .cmp(&right.display_name.to_ascii_lowercase())
            .then_with(|| left.relative_path.cmp(&right.relative_path))
    });

    let has_local_matches = !applications.is_empty() || !workspace_files.is_empty();
    let mut rows = applications
        .into_iter()
        .map(CommandRow::OpenApplication)
        .chain(
            workspace_files
                .into_iter()
                .map(CommandRow::OpenWorkspaceFile),
        )
        .collect::<Vec<_>>();

    if has_local_matches {
        rows.push(CommandRow::AskBlossom {
            prompt: query.into(),
        });
        return CommandRoute::from_rows(rows);
    }

    match parse_obvious_workspace_create(query) {
        DirectWorkspaceCreateParse::Valid(proposal) => {
            return CommandRoute::from_rows(vec![CommandRow::CreateWorkspaceFile(proposal)]);
        }
        DirectWorkspaceCreateParse::Invalid(error) => {
            return CommandRoute::from_rows(vec![CommandRow::InvalidCreate(error)]);
        }
        DirectWorkspaceCreateParse::NoMatch => {}
    }

    rows.push(CommandRow::AskBlossom {
        prompt: query.into(),
    });

    CommandRoute::from_rows(rows)
}

/// Produces the terminal display state only after trusted orchestration has
/// accepted the model's closed `unsupported` result. Routing never guesses
/// support from words in the user's query.
pub fn unsupported_command_result() -> CommandRoute {
    CommandRoute::from_rows(vec![CommandRow::Unsupported {
        message: UNSUPPORTED_RESPONSE,
    }])
}

pub fn activate_command(row: &CommandRow) -> CommandActivation {
    match row {
        CommandRow::OpenApplication(application) => CommandActivation::OpenApplication {
            desktop_id: application.desktop_id.clone(),
            desktop_entry_path: application.desktop_entry_path.clone(),
            desktop_entry_sha256: application.desktop_entry_sha256.clone(),
            origin: application.origin,
            launch_argv: application.launch_argv.clone(),
        },
        CommandRow::OpenWorkspaceFile(file) => CommandActivation::OpenWorkspaceFile {
            relative_path: file.relative_path.clone(),
        },
        CommandRow::CreateWorkspaceFile(proposal) => {
            CommandActivation::BeginWorkspaceCreate(proposal.clone())
        }
        CommandRow::AskBlossom { prompt } => CommandActivation::BeginAgentTurn {
            prompt: prompt.clone(),
        },
        CommandRow::InvalidCreate(error) => CommandActivation::ShowInvalidCreate(*error),
        CommandRow::Unsupported { message } => CommandActivation::ShowUnsupported { message },
    }
}

impl CommandActivation {
    pub fn application_entry_matches(&self, bytes: &[u8]) -> bool {
        match self {
            Self::OpenApplication {
                desktop_entry_sha256,
                ..
            } => sha256(bytes) == *desktop_entry_sha256,
            _ => false,
        }
    }

    pub fn application_launch_argv(&self) -> Option<&[String]> {
        match self {
            Self::OpenApplication { launch_argv, .. } => Some(launch_argv),
            _ => None,
        }
    }
}

fn normalized_match(candidate: &str, normalized_query: &str) -> bool {
    candidate.to_ascii_lowercase().starts_with(normalized_query)
}

fn sha256(bytes: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(bytes);
    digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrokerCommandRow {
    pub id: String,
    pub kind: &'static str,
    pub title: String,
    pub detail: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandStoreError {
    InvalidQuery,
    TooManyPeers,
    RandomnessUnavailable,
    UnknownOrStaleRow,
}

struct LiveCommandRows {
    generation: u64,
    actions: HashMap<String, CommandActivation>,
}

/// Holds only the latest result set for each authenticated peer. Activating any
/// row consumes that peer's complete set, making every sibling ID stale.
#[derive(Default)]
pub struct BrokerCommandStore {
    peers: HashMap<ShellPeerId, LiveCommandRows>,
    generations: HashMap<ShellPeerId, u64>,
}

impl BrokerCommandStore {
    pub fn replace(
        &mut self,
        peer: ShellPeerId,
        query: &str,
        route: CommandRoute,
    ) -> Result<Vec<BrokerCommandRow>, CommandStoreError> {
        if query.is_empty() || query.len() > MAX_COMMAND_QUERY_BYTES || query.contains('\0') {
            return Err(CommandStoreError::InvalidQuery);
        }
        if !self.peers.contains_key(&peer) && self.peers.len() >= MAX_COMMAND_PEERS {
            return Err(CommandStoreError::TooManyPeers);
        }
        let generation = self
            .generations
            .get(&peer)
            .copied()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or(CommandStoreError::InvalidQuery)?;
        self.generations.insert(peer.clone(), generation);

        let mut actions = HashMap::new();
        let mut projection = Vec::new();
        for (index, row) in route.rows.into_iter().take(MAX_COMMAND_RESULTS).enumerate() {
            let id = random_row_id(&peer, generation, index)?;
            let (kind, title, detail) = row_projection(&row);
            actions.insert(id.clone(), activate_command(&row));
            projection.push(BrokerCommandRow {
                id,
                kind,
                title,
                detail,
            });
        }
        self.peers.insert(
            peer,
            LiveCommandRows {
                generation,
                actions,
            },
        );
        Ok(projection)
    }

    pub fn activate(
        &mut self,
        peer: &ShellPeerId,
        id: &str,
    ) -> Result<CommandActivation, CommandStoreError> {
        let live = self
            .peers
            .get(peer)
            .ok_or(CommandStoreError::UnknownOrStaleRow)?;
        if live.generation != self.generations.get(peer).copied().unwrap_or_default() {
            return Err(CommandStoreError::UnknownOrStaleRow);
        }
        let action = live
            .actions
            .get(id)
            .cloned()
            .ok_or(CommandStoreError::UnknownOrStaleRow)?;
        self.peers.remove(peer);
        Ok(action)
    }

    pub fn disconnect(&mut self, peer: &ShellPeerId) {
        self.peers.remove(peer);
        self.generations.remove(peer);
    }

    pub fn live_peer_count(&self) -> usize {
        self.peers.len()
    }
}

fn random_row_id(
    peer: &ShellPeerId,
    generation: u64,
    index: usize,
) -> Result<String, CommandStoreError> {
    let mut random = [0u8; 16];
    getrandom::fill(&mut random).map_err(|_| CommandStoreError::RandomnessUnavailable)?;
    let mut digest = Sha256::new();
    digest.update(b"blossom-command-row-v1\0");
    digest.update(peer.as_str().as_bytes());
    digest.update(generation.to_le_bytes());
    digest.update(index.to_le_bytes());
    digest.update(random);
    let digest = digest.finalize();
    Ok(digest[..16]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn row_projection(row: &CommandRow) -> (&'static str, String, String) {
    match row {
        CommandRow::OpenApplication(application) => (
            "application",
            application.name.clone(),
            match application.origin {
                ApplicationOrigin::System => "System application · no model".into(),
                ApplicationOrigin::UserInstalled => "User-installed application · no model".into(),
            },
        ),
        CommandRow::OpenWorkspaceFile(file) => (
            "workspace_file",
            file.display_name.clone(),
            file.relative_path.clone(),
        ),
        CommandRow::CreateWorkspaceFile(proposal) => (
            "workspace_create",
            format!("Create {}", proposal.name),
            "Parsed directly · approval required".into(),
        ),
        CommandRow::AskBlossom { prompt } => (
            "ask_blossom",
            format!("Ask Blossom: {prompt}"),
            "Uses the local model".into(),
        ),
        CommandRow::InvalidCreate(_) => (
            "invalid_create",
            "Invalid create request".into(),
            "Nothing will run".into(),
        ),
        CommandRow::Unsupported { message } => (
            "unsupported",
            (*message).into(),
            "No action available".into(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn applications() -> Vec<LocalApplication> {
        vec![
            LocalApplication::new(
                "org.mozilla.firefox.desktop".into(),
                "Firefox".into(),
                "/usr/share/applications/org.mozilla.firefox.desktop".into(),
                b"[Desktop Entry]\nExec=firefox %u\n",
                ApplicationOrigin::System,
            )
            .unwrap(),
            LocalApplication::new(
                "org.example.firewatch.desktop".into(),
                "Firewatch".into(),
                "/home/blossom/.local/share/applications/org.example.firewatch.desktop".into(),
                b"[Desktop Entry]\nExec=firewatch\n",
                ApplicationOrigin::UserInstalled,
            )
            .unwrap(),
        ]
    }

    fn files() -> Vec<WorkspaceFile> {
        vec![
            WorkspaceFile {
                relative_path: "briefs/field-notes.txt".into(),
                display_name: "Field notes.txt".into(),
            },
            WorkspaceFile {
                relative_path: "briefs/final-notes.txt".into(),
                display_name: "Final notes.txt".into(),
            },
        ]
    }

    fn parse_entry(text: &str, locale: &str) -> Result<LocalApplication, DesktopEntryError> {
        parse_desktop_entry(
            "example.desktop",
            Path::new("/usr/share/applications/example.desktop"),
            text.as_bytes(),
            ApplicationOrigin::System,
            locale,
            "Blossom",
            |program| matches!(program, "example" | "/usr/bin/example"),
        )
    }

    #[test]
    fn desktop_entry_uses_localized_and_plain_names_and_strips_all_field_codes() {
        let application = parse_entry(
            "[Desktop Entry]\nType=Application\nName=Example\nName[tr]=Ornek\nExec=example --open=%f %U --literal=%%\nOnlyShowIn=Blossom;GNOME;\n",
            "tr_TR.UTF-8",
        )
        .unwrap();
        assert_eq!(application.name, "Ornek");
        assert_eq!(
            application.launch_argv(),
            ["example", "--open=", "--literal=%"]
        );
        assert_eq!(
            route_command("exa", std::slice::from_ref(&application), &[])
                .rows
                .len(),
            2
        );
        assert_eq!(route_command("orn", &[application], &[]).rows.len(), 2);
    }

    #[test]
    fn desktop_entry_exclusions_fail_closed() {
        let cases = [
            ("Hidden=true", DesktopEntryError::Hidden),
            ("NoDisplay=true", DesktopEntryError::Hidden),
            ("Terminal=true", DesktopEntryError::UnsupportedLaunchMode),
            (
                "DBusActivatable=true",
                DesktopEntryError::UnsupportedLaunchMode,
            ),
            ("OnlyShowIn=GNOME;", DesktopEntryError::ExcludedDesktop),
            ("NotShowIn=Blossom;", DesktopEntryError::ExcludedDesktop),
            ("TryExec=missing", DesktopEntryError::MissingExecutable),
        ];
        for (extra, expected) in cases {
            let entry =
                format!("[Desktop Entry]\nType=Application\nName=Example\nExec=example\n{extra}\n");
            assert_eq!(parse_entry(&entry, "en"), Err(expected), "{extra}");
        }
    }

    #[test]
    fn desktop_entry_rejects_unsupported_shapes_and_missing_exec() {
        for entry in [
            "[Desktop Entry]\nType=Link\nName=Example\nExec=example\n",
            "[Desktop Entry]\nType=Application\nName=Example\n",
            "[Desktop Entry]\nType=Application\nName=Example\nExec=missing\n",
            "[Desktop Entry]\nType=Application\nName=Example\nExec=\"example\n",
        ] {
            assert!(parse_entry(entry, "en").is_err(), "{entry}");
        }
    }

    #[test]
    fn routing_table_is_ordered_and_local_rows_never_activate_the_model() {
        struct Case<'a> {
            query: &'a str,
            expected_rows: usize,
            expected_first: CommandActivation,
            ask_last: bool,
        }

        let cases = [
            Case {
                query: "fire",
                expected_rows: 3,
                expected_first: CommandActivation::OpenApplication {
                    desktop_id: "org.mozilla.firefox.desktop".into(),
                    desktop_entry_path: "/usr/share/applications/org.mozilla.firefox.desktop"
                        .into(),
                    desktop_entry_sha256: sha256(b"[Desktop Entry]\nExec=firefox %u\n"),
                    origin: ApplicationOrigin::System,
                    launch_argv: Vec::new(),
                },
                ask_last: true,
            },
            Case {
                query: "fi",
                expected_rows: 5,
                expected_first: CommandActivation::OpenApplication {
                    desktop_id: "org.mozilla.firefox.desktop".into(),
                    desktop_entry_path: "/usr/share/applications/org.mozilla.firefox.desktop"
                        .into(),
                    desktop_entry_sha256: sha256(b"[Desktop Entry]\nExec=firefox %u\n"),
                    origin: ApplicationOrigin::System,
                    launch_argv: Vec::new(),
                },
                ask_last: true,
            },
            Case {
                query: "create notes.txt containing Meeting notes",
                expected_rows: 1,
                expected_first: CommandActivation::BeginWorkspaceCreate(
                    ModelWorkspaceCreateProposal {
                        name: "notes.txt".into(),
                        content: "Meeting notes".into(),
                    },
                ),
                ask_last: false,
            },
        ];

        for case in cases {
            let route = route_command(case.query, &applications(), &files());
            assert_eq!(route.rows.len(), case.expected_rows, "{}", case.query);
            assert_eq!(route.selected_index, Some(0), "{}", case.query);
            assert_eq!(
                activate_command(&route.rows[0]),
                case.expected_first,
                "{}",
                case.query
            );
            assert_eq!(
                matches!(route.rows.last(), Some(CommandRow::AskBlossom { .. })),
                case.ask_last,
                "{}",
                case.query
            );
            assert!(
                !matches!(
                    activate_command(&route.rows[0]),
                    CommandActivation::BeginAgentTurn { .. }
                ),
                "a selected local or deterministic row must not reach the model"
            );
        }
    }

    #[test]
    fn ambiguous_local_matches_are_listed_without_guessing_and_ask_is_last() {
        let route = route_command("fi", &applications(), &files());
        assert!(matches!(route.rows[0], CommandRow::OpenApplication(_)));
        assert!(matches!(route.rows[1], CommandRow::OpenApplication(_)));
        assert!(matches!(route.rows[2], CommandRow::OpenWorkspaceFile(_)));
        assert!(matches!(route.rows[3], CommandRow::OpenWorkspaceFile(_)));
        assert!(matches!(route.rows[4], CommandRow::AskBlossom { .. }));
        assert_eq!(route.selected_index, Some(0));
    }

    #[test]
    fn invalid_direct_create_is_not_offered_to_the_model() {
        let route = route_command(
            "create Bad Name.txt containing do not repair this",
            &applications(),
            &files(),
        );
        assert_eq!(
            route.rows,
            vec![CommandRow::InvalidCreate(
                ModelWorkspaceProposalError::InvalidName
            )]
        );
    }

    #[test]
    fn first_containing_separator_preserves_later_content() {
        let route = route_command(
            "create notes.txt containing a sentence containing another word",
            &[],
            &[],
        );
        assert_eq!(
            route.rows,
            vec![CommandRow::CreateWorkspaceFile(
                ModelWorkspaceCreateProposal {
                    name: "notes.txt".into(),
                    content: "a sentence containing another word".into(),
                }
            )]
        );
    }

    #[test]
    fn supported_agent_request_requires_explicit_row_activation() {
        let route = route_command("Make a file called brief.txt saying source notes", &[], &[]);
        assert_eq!(route.rows.len(), 1);
        assert_eq!(
            activate_command(&route.rows[0]),
            CommandActivation::BeginAgentTurn {
                prompt: "Make a file called brief.txt saying source notes".into()
            }
        );
    }

    #[test]
    fn unresolved_input_is_offered_to_the_agent_without_keyword_guessing() {
        let route = route_command("Organize my downloads", &applications(), &files());
        assert_eq!(
            route.rows,
            vec![CommandRow::AskBlossom {
                prompt: "Organize my downloads".into()
            }]
        );
    }

    #[test]
    fn closed_agent_unsupported_result_has_no_action() {
        let route = unsupported_command_result();
        assert_eq!(
            route.rows,
            vec![CommandRow::Unsupported {
                message: UNSUPPORTED_RESPONSE
            }]
        );
        assert_eq!(
            activate_command(&route.rows[0]),
            CommandActivation::ShowUnsupported {
                message: UNSUPPORTED_RESPONSE
            }
        );
    }

    #[test]
    fn empty_input_has_no_rows_or_selection() {
        assert_eq!(
            route_command("  ", &applications(), &files()),
            CommandRoute {
                rows: Vec::new(),
                selected_index: None
            }
        );
        assert_eq!(CREATE_FILE_TEMPLATE, "create  containing ");
    }

    #[test]
    fn broker_rows_are_random_peer_bound_latest_query_only_and_consumed_once() {
        let first_peer = ShellPeerId::from_bus_unique_name(":1.101").unwrap();
        let second_peer = ShellPeerId::from_bus_unique_name(":1.102").unwrap();
        let mut store = BrokerCommandStore::default();
        let first = store
            .replace(
                first_peer.clone(),
                "fire",
                route_command("fire", &applications(), &files()),
            )
            .unwrap();
        assert!(first.iter().all(|row| {
            row.id.len() == 32 && row.id.bytes().all(|byte| byte.is_ascii_hexdigit())
        }));
        assert_eq!(store.live_peer_count(), 1);
        assert_eq!(
            store.activate(&second_peer, &first[0].id),
            Err(CommandStoreError::UnknownOrStaleRow)
        );

        let stale_id = first[0].id.clone();
        let latest_after_stale = store
            .replace(
                first_peer.clone(),
                "field",
                route_command("field", &applications(), &files()),
            )
            .unwrap();
        assert_eq!(
            store.activate(&first_peer, &stale_id),
            Err(CommandStoreError::UnknownOrStaleRow)
        );
        assert_eq!(store.live_peer_count(), 1);
        assert!(matches!(
            store
                .activate(&first_peer, &latest_after_stale[0].id)
                .unwrap(),
            CommandActivation::OpenWorkspaceFile { .. }
        ));
        assert_eq!(store.live_peer_count(), 0);

        let latest = store
            .replace(
                first_peer.clone(),
                "field",
                route_command("field", &applications(), &files()),
            )
            .unwrap();
        assert!(matches!(
            store.activate(&first_peer, &latest[0].id).unwrap(),
            CommandActivation::OpenWorkspaceFile { .. }
        ));
        assert_eq!(
            store.activate(&first_peer, &latest[0].id),
            Err(CommandStoreError::UnknownOrStaleRow)
        );
    }

    #[test]
    fn disconnect_clears_rows_and_application_digest_detects_substitution() {
        let peer = ShellPeerId::from_bus_unique_name(":1.103").unwrap();
        let mut store = BrokerCommandStore::default();
        let rows = store
            .replace(
                peer.clone(),
                "firefox",
                route_command("firefox", &applications(), &files()),
            )
            .unwrap();
        let activation = store.activate(&peer, &rows[0].id).unwrap();
        assert!(activation.application_entry_matches(b"[Desktop Entry]\nExec=firefox %u\n"));
        assert!(!activation.application_entry_matches(b"[Desktop Entry]\nExec=malware\n"));

        let rows = store
            .replace(
                peer.clone(),
                "firefox",
                route_command("firefox", &applications(), &files()),
            )
            .unwrap();
        store.disconnect(&peer);
        assert_eq!(store.live_peer_count(), 0);
        assert_eq!(
            store.activate(&peer, &rows[0].id),
            Err(CommandStoreError::UnknownOrStaleRow)
        );
    }

    #[test]
    fn store_allows_only_one_bounded_result_set_per_peer() {
        let mut store = BrokerCommandStore::default();
        for index in 1..=MAX_COMMAND_PEERS {
            let peer = ShellPeerId::from_bus_unique_name(&format!(":1.{index}")).unwrap();
            store
                .replace(peer, "query", route_command("query", &[], &[]))
                .unwrap();
        }
        assert_eq!(store.live_peer_count(), MAX_COMMAND_PEERS);
        let extra = ShellPeerId::from_bus_unique_name(":1.999").unwrap();
        assert_eq!(
            store.replace(extra, "query", route_command("query", &[], &[])),
            Err(CommandStoreError::TooManyPeers)
        );
    }
}
