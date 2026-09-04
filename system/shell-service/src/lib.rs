#![forbid(unsafe_code)]

#[cfg(all(target_os = "linux", target_env = "gnu"))]
mod session_bus;

#[cfg(all(target_os = "linux", target_env = "gnu"))]
pub use session_bus::{ShellBusService, ShellRequestHandler, run_production};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShellProcessError {
    InactiveBuild,
    RandomnessUnavailable,
    SessionBusUnavailable,
}

impl ShellProcessError {
    pub fn exit_code(self) -> i32 {
        match self {
            Self::InactiveBuild => 78,
            Self::RandomnessUnavailable => 70,
            Self::SessionBusUnavailable => 69,
        }
    }
}

impl std::fmt::Display for ShellProcessError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::InactiveBuild => "Blossom shell D-Bus service is inactive in this build",
            Self::RandomnessUnavailable => "Blossom shell service randomness is unavailable",
            Self::SessionBusUnavailable => "Blossom shell session bus is unavailable",
        })
    }
}

impl std::error::Error for ShellProcessError {}

#[cfg(not(all(target_os = "linux", target_env = "gnu")))]
pub fn run_production() -> Result<(), ShellProcessError> {
    Err(ShellProcessError::InactiveBuild)
}
