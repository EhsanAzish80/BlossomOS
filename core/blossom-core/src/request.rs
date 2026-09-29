use crate::file_read::FileSelection;
use crate::service_status::ServiceSelection;
use crate::workspace_create::WorkspaceCreateSelection;
use serde::Serialize;
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
    SystemBatterySummary {
        request_id: RequestId,
    },
    SystemNetworkConnectivity {
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
    pub fn request_id(&self) -> &RequestId {
        match self {
            Self::SystemUname { request_id }
            | Self::SystemOsIdentity { request_id }
            | Self::SystemUptime { request_id }
            | Self::SystemMemorySummary { request_id }
            | Self::SystemStorageSummary { request_id }
            | Self::SystemBatterySummary { request_id }
            | Self::SystemNetworkConnectivity { request_id }
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
            Self::SystemBatterySummary { .. } => "system.battery.summary",
            Self::SystemNetworkConnectivity { .. } => "system.network.connectivity",
            Self::ProcessSelf { .. } => "process.self",
            Self::ProcessList { .. } => "process.list",
            Self::FilesReadContent { .. } => "files.read.content",
            Self::FilesWriteCreate { .. } => "files.write.create",
            Self::ServicesReadStatus { .. } => "services.read.status",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RequestError {
    RequestTooLarge,
    MalformedJson { message: String },
    InvalidRequestId,
    InvalidToolName,
    UnknownTool { tool: String },
    InvalidArguments { message: String },
}

impl fmt::Display for RequestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RequestTooLarge => formatter.write_str("request exceeds the fixed size limit"),
            Self::MalformedJson { .. } => formatter.write_str("malformed request JSON"),
            Self::InvalidRequestId => formatter.write_str("invalid request identifier"),
            Self::InvalidToolName => formatter.write_str("invalid tool name"),
            Self::UnknownTool { tool } => write!(formatter, "unknown tool: {tool}"),
            Self::InvalidArguments { .. } => formatter.write_str("invalid tool arguments"),
        }
    }
}

impl std::error::Error for RequestError {}
