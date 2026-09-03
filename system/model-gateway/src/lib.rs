#![forbid(unsafe_code)]

//! Fail-closed process boundary for the future local model gateway.
//!
//! Release builds expose no listener until the installed-profile registry and
//! runtime readiness proof exist. Debug builds retain one explicit synthetic
//! fixture mode for separate-process protocol evidence only.

use std::fmt;

pub const PRODUCTION_SOCKET_PATH: &str = "/run/blossom-model-gateway/inference.sock";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GatewayProcessError {
    ProfileRegistryUnavailable,
    InvalidInvocation,
    InvalidFixtureConfiguration,
    FixtureUnavailable,
}

impl GatewayProcessError {
    pub fn exit_code(self) -> i32 {
        match self {
            Self::ProfileRegistryUnavailable => 78,
            Self::InvalidInvocation | Self::InvalidFixtureConfiguration => 64,
            Self::FixtureUnavailable => 69,
        }
    }
}

impl fmt::Display for GatewayProcessError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::ProfileRegistryUnavailable => "model gateway is not production-ready",
            Self::InvalidInvocation => "model gateway invocation is invalid",
            Self::InvalidFixtureConfiguration => "synthetic gateway configuration is invalid",
            Self::FixtureUnavailable => "synthetic gateway fixture is unavailable",
        })
    }
}

impl std::error::Error for GatewayProcessError {}

/// The production entry point deliberately fails before creating or connecting
/// any socket. A later checkpoint must replace this only after closed registry
/// and runtime identity validation are available.
pub fn run_production() -> Result<(), GatewayProcessError> {
    Err(GatewayProcessError::ProfileRegistryUnavailable)
}

#[cfg(all(debug_assertions, unix))]
pub fn run_synthetic_fixture_from_environment() -> Result<(), GatewayProcessError> {
    use blossom_core::{GatewayProfile, serve_synthetic_gateway_once};
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::net::UnixListener;
    use std::path::PathBuf;

    const SOCKET: &str = "BLOSSOM_SYNTHETIC_GATEWAY_SOCKET";
    const CLIENT_UID: &str = "BLOSSOM_SYNTHETIC_GATEWAY_CLIENT_UID";
    const CLIENT_GID: &str = "BLOSSOM_SYNTHETIC_GATEWAY_CLIENT_GID";
    const PROFILE: &str = "BLOSSOM_SYNTHETIC_GATEWAY_PROFILE";

    let socket = std::env::var_os(SOCKET)
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .ok_or(GatewayProcessError::InvalidFixtureConfiguration)?;
    let uid = parse_id(CLIENT_UID)?;
    let gid = parse_id(CLIENT_GID)?;
    let profile = match std::env::var(PROFILE).ok().as_deref() {
        Some("ollama_cpu_v1") => GatewayProfile::OllamaCpuV1,
        Some("llama_cpp_cpu_v1") => GatewayProfile::LlamaCppCpuV1,
        _ => return Err(GatewayProcessError::InvalidFixtureConfiguration),
    };
    if socket == std::path::Path::new(PRODUCTION_SOCKET_PATH) || socket.exists() {
        return Err(GatewayProcessError::InvalidFixtureConfiguration);
    }

    let listener =
        UnixListener::bind(&socket).map_err(|_| GatewayProcessError::FixtureUnavailable)?;
    fs::set_permissions(&socket, fs::Permissions::from_mode(0o600))
        .map_err(|_| GatewayProcessError::FixtureUnavailable)?;
    let result = serve_synthetic_gateway_once(
        &listener,
        uid,
        gid,
        profile,
        &"a".repeat(64),
        "gateway-process-fixture-1",
        "synthetic-gateway-ok",
    )
    .map_err(|_| GatewayProcessError::FixtureUnavailable);
    drop(listener);
    let _ = fs::remove_file(&socket);
    result
}

#[cfg(all(debug_assertions, unix))]
fn parse_id(name: &str) -> Result<u32, GatewayProcessError> {
    let value = std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .filter(|value| *value != 0)
        .ok_or(GatewayProcessError::InvalidFixtureConfiguration)?;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn production_entry_point_is_fail_closed() {
        assert_eq!(
            run_production(),
            Err(GatewayProcessError::ProfileRegistryUnavailable)
        );
        assert_eq!(
            GatewayProcessError::ProfileRegistryUnavailable.exit_code(),
            78
        );
    }

    #[test]
    fn errors_are_content_free() {
        assert!(
            !GatewayProcessError::FixtureUnavailable
                .to_string()
                .contains(PRODUCTION_SOCKET_PATH)
        );
    }
}
