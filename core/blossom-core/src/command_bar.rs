use crate::{
    DirectWorkspaceCreateParse, ModelWorkspaceCreateProposal, ModelWorkspaceProposalError,
    parse_obvious_workspace_create,
};

pub const CREATE_FILE_TEMPLATE: &str = "create  containing ";
pub const UNSUPPORTED_RESPONSE: &str =
    "Blossom can't do this yet. Today it can create one file in your workspace.";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalApplication {
    pub desktop_id: String,
    pub name: String,
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
    OpenApplication { desktop_id: String },
    OpenWorkspaceFile { relative_path: String },
    BeginWorkspaceCreate(ModelWorkspaceCreateProposal),
    BeginAgentTurn { prompt: String },
    ShowInvalidCreate(ModelWorkspaceProposalError),
    ShowUnsupported { message: &'static str },
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
        .filter(|application| normalized_match(&application.name, &normalized_query))
        .cloned()
        .collect::<Vec<_>>();
    applications.sort_by(|left, right| {
        left.name
            .to_ascii_lowercase()
            .cmp(&right.name.to_ascii_lowercase())
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

fn normalized_match(candidate: &str, normalized_query: &str) -> bool {
    candidate.to_ascii_lowercase().starts_with(normalized_query)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn applications() -> Vec<LocalApplication> {
        vec![
            LocalApplication {
                desktop_id: "org.mozilla.firefox.desktop".into(),
                name: "Firefox".into(),
            },
            LocalApplication {
                desktop_id: "org.example.firewatch.desktop".into(),
                name: "Firewatch".into(),
            },
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
                },
                ask_last: true,
            },
            Case {
                query: "fi",
                expected_rows: 5,
                expected_first: CommandActivation::OpenApplication {
                    desktop_id: "org.mozilla.firefox.desktop".into(),
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
}
