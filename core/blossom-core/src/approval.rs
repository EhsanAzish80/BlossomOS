use serde::Serialize;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ApprovalToken(pub(crate) u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum ApprovalError {
    Unknown,
    Expired,
    Replay,
    BindingMismatch,
}

impl fmt::Display for ApprovalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Unknown => "unknown approval token",
            Self::Expired => "approval token expired",
            Self::Replay => "approval token was already consumed",
            Self::BindingMismatch => "approval token does not match the preview",
        })
    }
}

impl std::error::Error for ApprovalError {}
