use crate::file_read::{FileSelection, validate_selected_path};
use crate::service_status::{ServiceSelection, validate_service_unit};
use crate::workspace_create::{WorkspaceCreateSelection, validate_workspace_selection};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

const MAX_REQUEST_ID_BYTES: usize = 64;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
pub struct RequestId(String);

impl RequestId {
    pub fn parse(value: String) -> Result<Self, RequestError> {
        let valid = !value.is_empty()
            && value.len() <= MAX_REQUEST_ID_BYTES
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'));
        if valid {
            Ok(Self(value))
        } else {
            Err(RequestError::InvalidRequestId)
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
pub enum ToolRequest {
    SystemUname {
        request_id: RequestId,
    },
    SystemOsIdentity {
        request_id: RequestId,
    },
    SystemUptime {
        request_id: RequestId,
    },
    SystemMemorySummary {
        request_id: RequestId,
    },
    SystemStorageSummary {
        request_id: RequestId,
    },
    ProcessSelf {
        request_id: RequestId,
    },
    ProcessList {
        request_id: RequestId,
    },
    FilesReadContent {
        request_id: RequestId,
        selection: FileSelection,
    },
    FilesWriteCreate {
        request_id: RequestId,
        selection: WorkspaceCreateSelection,
    },
    ServicesReadStatus {
        request_id: RequestId,
        selection: ServiceSelection,
    },
}

impl ToolRequest {
    pub fn parse_json(input: &str) -> Result<Self, RequestError> {
        let envelope: RequestEnvelope =
            serde_json::from_str(input).map_err(|error| RequestError::MalformedJson {
                message: error.to_string(),
            })?;
        let request_id = RequestId::parse(envelope.request_id)?;
        match envelope.tool.as_str() {
            "system.uname" => {
                serde_json::from_value::<NoArguments>(envelope.arguments).map_err(|error| {
                    RequestError::InvalidArguments {
                        message: error.to_string(),
                    }
                })?;
                Ok(Self::SystemUname { request_id })
            }
            "system.os.identity" => {
                serde_json::from_value::<NoArguments>(envelope.arguments).map_err(|error| {
                    RequestError::InvalidArguments {
                        message: error.to_string(),
                    }
                })?;
                Ok(Self::SystemOsIdentity { request_id })
            }
            "system.uptime" => {
                serde_json::from_value::<NoArguments>(envelope.arguments).map_err(|error| {
                    RequestError::InvalidArguments {
                        message: error.to_string(),
                    }
                })?;
                Ok(Self::SystemUptime { request_id })
            }
            "system.memory.summary" => {
                serde_json::from_value::<NoArguments>(envelope.arguments).map_err(|error| {
                    RequestError::InvalidArguments {
                        message: error.to_string(),
                    }
                })?;
                Ok(Self::SystemMemorySummary { request_id })
            }
            "system.storage.summary" => {
                serde_json::from_value::<NoArguments>(envelope.arguments).map_err(|error| {
                    RequestError::InvalidArguments {
                        message: error.to_string(),
                    }
                })?;
                Ok(Self::SystemStorageSummary { request_id })
            }
            "process.self" => {
                serde_json::from_value::<NoArguments>(envelope.arguments).map_err(|error| {
                    RequestError::InvalidArguments {
                        message: error.to_string(),
                    }
                })?;
                Ok(Self::ProcessSelf { request_id })
            }
            "process.list" => {
                serde_json::from_value::<NoArguments>(envelope.arguments).map_err(|error| {
                    RequestError::InvalidArguments {
                        message: error.to_string(),
                    }
                })?;
                Ok(Self::ProcessList { request_id })
            }
            "files.read.content" => {
                let arguments = serde_json::from_value::<FileReadArguments>(envelope.arguments)
                    .map_err(|error| RequestError::InvalidArguments {
                        message: error.to_string(),
                    })?;
                validate_selected_path(&arguments.selection.absolute_path).map_err(|error| {
                    RequestError::InvalidArguments {
                        message: error.to_string(),
                    }
                })?;
                if !arguments.selection.identity.is_valid() {
                    return Err(RequestError::InvalidArguments {
                        message: "invalid selected file identity".into(),
                    });
                }
                Ok(Self::FilesReadContent {
                    request_id,
                    selection: arguments.selection,
                })
            }
            "files.write.create" => {
                let arguments =
                    serde_json::from_value::<WorkspaceCreateArguments>(envelope.arguments)
                        .map_err(|error| RequestError::InvalidArguments {
                            message: error.to_string(),
                        })?;
                validate_workspace_selection(&arguments.selection).map_err(|error| {
                    RequestError::InvalidArguments {
                        message: error.to_string(),
                    }
                })?;
                Ok(Self::FilesWriteCreate {
                    request_id,
                    selection: arguments.selection,
                })
            }
            "services.read.status" => {
                let arguments = serde_json::from_value::<ServiceStatusArguments>(
                    envelope.arguments,
                )
                .map_err(|error| RequestError::InvalidArguments {
                    message: error.to_string(),
                })?;
                validate_service_unit(&arguments.selection.unit).map_err(|error| {
                    RequestError::InvalidArguments {
                        message: error.to_string(),
                    }
                })?;
                Ok(Self::ServicesReadStatus {
                    request_id,
                    selection: arguments.selection,
                })
            }
            _ => Err(RequestError::UnknownTool {
                tool: envelope.tool,
            }),
        }
    }

    pub fn request_id(&self) -> &RequestId {
        match self {
            Self::SystemUname { request_id }
            | Self::SystemOsIdentity { request_id }
            | Self::SystemUptime { request_id }
            | Self::SystemMemorySummary { request_id }
            | Self::SystemStorageSummary { request_id }
            | Self::ProcessSelf { request_id }
            | Self::ProcessList { request_id } => request_id,
            Self::FilesReadContent { request_id, .. } => request_id,
            Self::FilesWriteCreate { request_id, .. } => request_id,
            Self::ServicesReadStatus { request_id, .. } => request_id,
        }
    }

    pub fn tool_name(&self) -> &'static str {
        match self {
            Self::SystemUname { .. } => "system.uname",
            Self::SystemOsIdentity { .. } => "system.os.identity",
            Self::SystemUptime { .. } => "system.uptime",
            Self::SystemMemorySummary { .. } => "system.memory.summary",
            Self::SystemStorageSummary { .. } => "system.storage.summary",
            Self::ProcessSelf { .. } => "process.self",
            Self::ProcessList { .. } => "process.list",
            Self::FilesReadContent { .. } => "files.read.content",
            Self::FilesWriteCreate { .. } => "files.write.create",
            Self::ServicesReadStatus { .. } => "services.read.status",
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RequestEnvelope {
    request_id: String,
    tool: String,
    arguments: Value,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct NoArguments {}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileReadArguments {
    selection: FileSelection,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkspaceCreateArguments {
    selection: WorkspaceCreateSelection,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ServiceStatusArguments {
    selection: ServiceSelection,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RequestError {
    MalformedJson { message: String },
    InvalidRequestId,
    UnknownTool { tool: String },
    InvalidArguments { message: String },
}

impl fmt::Display for RequestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MalformedJson { .. } => formatter.write_str("malformed request JSON"),
            Self::InvalidRequestId => formatter.write_str("invalid request identifier"),
            Self::UnknownTool { tool } => write!(formatter, "unknown tool: {tool}"),
            Self::InvalidArguments { .. } => formatter.write_str("invalid tool arguments"),
        }
    }
}

impl std::error::Error for RequestError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn file_request_json(path: &str) -> String {
        serde_json::json!({
            "request_id": "req-file", "tool": "files.read.content",
            "arguments": { "selection": { "absolute_path": path, "identity": {
                "device": 1, "inode": 2, "size": 3, "modified_seconds": 4,
                "modified_nanoseconds": 5, "changed_seconds": 6, "changed_nanoseconds": 7
            }}}
        })
        .to_string()
    }

    #[test]
    fn parses_exact_file_selection_and_rejects_scope_expansion() {
        let request = ToolRequest::parse_json(&file_request_json("/home/user/note.txt"))
            .expect("exact selection");
        let ToolRequest::FilesReadContent { selection, .. } = request else {
            panic!("file request")
        };
        assert_eq!(selection.absolute_path, "/home/user/note.txt");
        assert_eq!(selection.identity.inode, 2);
        assert!(matches!(
            ToolRequest::parse_json(&file_request_json("relative")),
            Err(RequestError::InvalidArguments { .. })
        ));
        assert!(matches!(
            ToolRequest::parse_json(
                &file_request_json("/home/user/note.txt")
                    .replace("\"inode\":2", "\"inode\":2,\"mounts\":[]")
            ),
            Err(RequestError::InvalidArguments { .. })
        ));
    }

    #[test]
    fn parses_workspace_create_and_rejects_mode_or_digest_expansion() {
        use crate::workspace_create::{
            DirectoryIdentity, WORKSPACE_FILE_MODE, WorkspaceCreateSelection, digest,
        };
        let selection = WorkspaceCreateSelection {
            workspace_root: "/home/user/workspace".into(),
            root_identity: DirectoryIdentity {
                device: 1,
                inode: 2,
            },
            parent_identity: DirectoryIdentity {
                device: 1,
                inode: 3,
            },
            relative_destination: "docs/new.txt".into(),
            content: "hello".into(),
            content_sha256: digest(b"hello"),
            mode: WORKSPACE_FILE_MODE,
        };
        let value = serde_json::json!({
            "request_id": "req-create", "tool": "files.write.create", "arguments": { "selection": selection }
        });
        let request = ToolRequest::parse_json(&value.to_string()).expect("workspace create");
        assert_eq!(request.tool_name(), "files.write.create");
        let mut wrong_mode = value.clone();
        wrong_mode["arguments"]["selection"]["mode"] = serde_json::json!(0o644);
        assert!(matches!(
            ToolRequest::parse_json(&wrong_mode.to_string()),
            Err(RequestError::InvalidArguments { .. })
        ));
        let mut wrong_digest = value;
        wrong_digest["arguments"]["selection"]["content_sha256"] =
            serde_json::json!("0".repeat(64));
        assert!(matches!(
            ToolRequest::parse_json(&wrong_digest.to_string()),
            Err(RequestError::InvalidArguments { .. })
        ));
    }

    #[test]
    fn parses_exact_service_status_and_rejects_scope_expansion() {
        let valid = serde_json::json!({
            "request_id": "req-service", "tool": "services.read.status",
            "arguments": { "selection": { "unit": "sshd.service" } }
        });
        let request = ToolRequest::parse_json(&valid.to_string()).expect("exact service");
        let ToolRequest::ServicesReadStatus { selection, .. } = request else {
            panic!("service request")
        };
        assert_eq!(selection.unit, "sshd.service");
        for unit in ["*.service", "sshd.socket", "../sshd.service"] {
            let mut invalid = valid.clone();
            invalid["arguments"]["selection"]["unit"] = serde_json::json!(unit);
            assert!(matches!(
                ToolRequest::parse_json(&invalid.to_string()),
                Err(RequestError::InvalidArguments { .. })
            ));
        }
        let mut expanded = valid;
        expanded["arguments"]["selection"]["destination"] = serde_json::json!("org.example.Other");
        assert!(matches!(
            ToolRequest::parse_json(&expanded.to_string()),
            Err(RequestError::InvalidArguments { .. })
        ));
    }

    #[test]
    fn parses_the_registered_tool() {
        let request = ToolRequest::parse_json(
            r#"{"request_id":"req-1","tool":"system.uname","arguments":{}}"#,
        )
        .expect("registered request should parse");
        assert_eq!(request.tool_name(), "system.uname");
        assert_eq!(request.request_id().as_str(), "req-1");
    }

    #[test]
    fn parses_os_identity_without_arguments() {
        let request = ToolRequest::parse_json(
            r#"{"request_id":"req-2","tool":"system.os.identity","arguments":{}}"#,
        )
        .expect("OS identity request should parse");
        assert_eq!(request.tool_name(), "system.os.identity");
    }

    #[test]
    fn parses_uptime_without_arguments() {
        let request = ToolRequest::parse_json(
            r#"{"request_id":"req-3","tool":"system.uptime","arguments":{}}"#,
        )
        .expect("uptime request should parse");
        assert_eq!(request.tool_name(), "system.uptime");
    }

    #[test]
    fn parses_memory_summary_without_arguments() {
        let request = ToolRequest::parse_json(
            r#"{"request_id":"req-4","tool":"system.memory.summary","arguments":{}}"#,
        )
        .expect("memory summary request should parse");
        assert_eq!(request.tool_name(), "system.memory.summary");
    }

    #[test]
    fn parses_storage_summary_without_arguments() {
        let request = ToolRequest::parse_json(
            r#"{"request_id":"req-5","tool":"system.storage.summary","arguments":{}}"#,
        )
        .expect("storage summary request should parse");
        assert_eq!(request.tool_name(), "system.storage.summary");
    }

    #[test]
    fn parses_process_self_without_arguments() {
        let request = ToolRequest::parse_json(
            r#"{"request_id":"req-6","tool":"process.self","arguments":{}}"#,
        )
        .expect("process self request should parse");
        assert_eq!(request.tool_name(), "process.self");
    }

    #[test]
    fn parses_process_list_without_arguments() {
        let request = ToolRequest::parse_json(
            r#"{"request_id":"req-7","tool":"process.list","arguments":{}}"#,
        )
        .expect("process list request should parse");
        assert_eq!(request.tool_name(), "process.list");
    }

    #[test]
    fn rejects_unknown_fields_and_tools() {
        assert!(matches!(
            ToolRequest::parse_json(
                r#"{"request_id":"req-1","tool":"system.uname","arguments":{},"extra":true}"#
            ),
            Err(RequestError::MalformedJson { .. })
        ));
        assert!(matches!(
            ToolRequest::parse_json(
                r#"{"request_id":"req-3","tool":"system.uptime","arguments":{"path":"/tmp/fake"}}"#
            ),
            Err(RequestError::InvalidArguments { .. })
        ));
        assert!(matches!(
            ToolRequest::parse_json(
                r#"{"request_id":"req-1","tool":"shell.execute","arguments":{}}"#
            ),
            Err(RequestError::UnknownTool { .. })
        ));
    }

    #[test]
    fn rejects_arguments_for_argument_free_tool() {
        assert!(matches!(
            ToolRequest::parse_json(
                r#"{"request_id":"req-1","tool":"system.uname","arguments":{"flag":"-a"}}"#
            ),
            Err(RequestError::InvalidArguments { .. })
        ));
        assert!(matches!(
            ToolRequest::parse_json(
                r#"{"request_id":"req-2","tool":"system.os.identity","arguments":{"path":"/tmp/fake"}}"#
            ),
            Err(RequestError::InvalidArguments { .. })
        ));
        assert!(matches!(
            ToolRequest::parse_json(
                r#"{"request_id":"req-4","tool":"system.memory.summary","arguments":{"field":"Cached"}}"#
            ),
            Err(RequestError::InvalidArguments { .. })
        ));
        assert!(matches!(
            ToolRequest::parse_json(
                r#"{"request_id":"req-5","tool":"system.storage.summary","arguments":{"path":"/home"}}"#
            ),
            Err(RequestError::InvalidArguments { .. })
        ));
        assert!(matches!(
            ToolRequest::parse_json(
                r#"{"request_id":"req-6","tool":"process.self","arguments":{"pid":1}}"#
            ),
            Err(RequestError::InvalidArguments { .. })
        ));
    }
}
