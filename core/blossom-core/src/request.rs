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
    SystemUname { request_id: RequestId },
    SystemOsIdentity { request_id: RequestId },
    SystemUptime { request_id: RequestId },
    SystemMemorySummary { request_id: RequestId },
    SystemStorageSummary { request_id: RequestId },
    ProcessSelf { request_id: RequestId },
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
            | Self::ProcessSelf { request_id } => request_id,
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
