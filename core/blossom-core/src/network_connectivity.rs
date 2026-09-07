use crate::context::{
    CONTEXT_PROTOCOL_VERSION, ContextObservation, ContextSource, ContextValue,
    MAX_CONTEXT_RESPONSE_BYTES,
};
use serde::Serialize;
use std::fmt;

pub const NETWORK_MANAGER_DESTINATION: &str = "org.freedesktop.NetworkManager";
pub const NETWORK_MANAGER_PATH: &str = "/org/freedesktop/NetworkManager";
pub const NETWORK_MANAGER_INTERFACE: &str = "org.freedesktop.NetworkManager";
pub const DBUS_PROPERTIES_INTERFACE: &str = "org.freedesktop.DBus.Properties";
pub const SYSTEM_BUS_ADDRESS: &str = "unix:path=/run/dbus/system_bus_socket";
pub const NETWORK_CONNECTIVITY_READ_TIMEOUT_MS: u64 = 2_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkConnectivity {
    Offline,
    Local,
    Limited,
    Online,
    Unknown,
}

pub type NetworkConnectivityObservation = ContextObservation<NetworkConnectivity>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkConnectivityObservationError {
    WrongSource,
    UnsupportedSchema,
    InvalidLifetime,
    FutureTimestamp,
    Stale,
    UnexpectedAbsence,
    ResponseTooLarge,
}

impl fmt::Display for NetworkConnectivityObservationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::WrongSource => "network observation has the wrong source",
            Self::UnsupportedSchema => "network observation schema is unsupported",
            Self::InvalidLifetime => "network observation lifetime is not code-owned",
            Self::FutureTimestamp => "network observation timestamp is in the future",
            Self::Stale => "network observation is stale",
            Self::UnexpectedAbsence => "network connectivity cannot be represented as absent",
            Self::ResponseTooLarge => "network observation exceeds the response limit",
        })
    }
}

impl std::error::Error for NetworkConnectivityObservationError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkConnectivityReadError {
    UnsupportedPlatform,
    ConnectionFailed,
    OwnerUnavailable,
    UntrustedOwner,
    PropertyFailed,
    Timeout,
    ProtocolViolation,
}

impl fmt::Display for NetworkConnectivityReadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::UnsupportedPlatform => "network connectivity requires Linux NetworkManager D-Bus",
            Self::ConnectionFailed => "the local system bus is unavailable",
            Self::OwnerUnavailable => "the fixed NetworkManager service owner is unavailable",
            Self::UntrustedOwner => "the fixed NetworkManager service owner is not trusted",
            Self::PropertyFailed => "the fixed NetworkManager property could not be read",
            Self::Timeout => "the fixed network connectivity read deadline expired",
            Self::ProtocolViolation => "NetworkManager returned invalid connectivity data",
        })
    }
}

impl std::error::Error for NetworkConnectivityReadError {}

pub trait NetworkConnectivityProvider {
    fn read_network_connectivity(
        &mut self,
    ) -> Result<NetworkConnectivityObservation, NetworkConnectivityReadError>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct UnavailableNetworkConnectivityProvider;

impl NetworkConnectivityProvider for UnavailableNetworkConnectivityProvider {
    fn read_network_connectivity(
        &mut self,
    ) -> Result<NetworkConnectivityObservation, NetworkConnectivityReadError> {
        Err(NetworkConnectivityReadError::UnsupportedPlatform)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct NetworkManagerConnectivityProvider;

#[cfg(not(all(target_os = "linux", target_env = "gnu")))]
impl NetworkConnectivityProvider for NetworkManagerConnectivityProvider {
    fn read_network_connectivity(
        &mut self,
    ) -> Result<NetworkConnectivityObservation, NetworkConnectivityReadError> {
        Err(NetworkConnectivityReadError::UnsupportedPlatform)
    }
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
impl NetworkConnectivityProvider for NetworkManagerConnectivityProvider {
    fn read_network_connectivity(
        &mut self,
    ) -> Result<NetworkConnectivityObservation, NetworkConnectivityReadError> {
        read_network_manager_connectivity_at(
            SYSTEM_BUS_ADDRESS,
            std::time::Duration::from_millis(NETWORK_CONNECTIVITY_READ_TIMEOUT_MS),
            0,
        )
    }
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
fn read_network_manager_connectivity_at(
    address: &str,
    timeout: std::time::Duration,
    expected_uid: u32,
) -> Result<NetworkConnectivityObservation, NetworkConnectivityReadError> {
    use futures_lite::future::race;

    async_io::block_on(race(
        read_network_manager_connectivity(address, expected_uid),
        async move {
            async_io::Timer::after(timeout).await;
            Err(NetworkConnectivityReadError::Timeout)
        },
    ))
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
async fn read_network_manager_connectivity(
    address: &str,
    expected_uid: u32,
) -> Result<NetworkConnectivityObservation, NetworkConnectivityReadError> {
    use zbus::connection;
    use zbus::proxy::MethodFlags;
    use zbus::zvariant::OwnedValue;

    let connection = connection::Builder::address(address)
        .map_err(|_| NetworkConnectivityReadError::ConnectionFailed)?
        .max_queued(8)
        .build()
        .await
        .map_err(|_| NetworkConnectivityReadError::ConnectionFailed)?;
    let bus = fixed_proxy(
        &connection,
        "org.freedesktop.DBus",
        "/org/freedesktop/DBus",
        "org.freedesktop.DBus",
    )
    .await?;
    let owner_before = resolve_owner(&bus, NETWORK_MANAGER_DESTINATION).await?;
    require_owner_uid(&bus, &owner_before, expected_uid).await?;

    let properties = fixed_proxy(
        &connection,
        NETWORK_MANAGER_DESTINATION,
        NETWORK_MANAGER_PATH,
        DBUS_PROPERTIES_INTERFACE,
    )
    .await?;
    let value: OwnedValue = properties
        .call_with_flags(
            "Get",
            MethodFlags::NoAutoStart.into(),
            &(NETWORK_MANAGER_INTERFACE, "Connectivity"),
        )
        .await
        .map_err(|_| NetworkConnectivityReadError::PropertyFailed)?
        .ok_or(NetworkConnectivityReadError::ProtocolViolation)?;
    let raw = u32::try_from(value).map_err(|_| NetworkConnectivityReadError::ProtocolViolation)?;

    let owner_after = resolve_owner(&bus, NETWORK_MANAGER_DESTINATION).await?;
    if owner_after != owner_before {
        return Err(NetworkConnectivityReadError::UntrustedOwner);
    }
    require_owner_uid(&bus, &owner_after, expected_uid).await?;
    build_network_manager_observation(raw, now_ms())
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
async fn fixed_proxy<'a>(
    connection: &'a zbus::Connection,
    destination: &'a str,
    path: &'a str,
    interface: &'a str,
) -> Result<zbus::Proxy<'a>, NetworkConnectivityReadError> {
    use zbus::proxy::{Builder as ProxyBuilder, CacheProperties};

    ProxyBuilder::new(connection)
        .destination(destination)
        .and_then(|builder| builder.path(path))
        .and_then(|builder| builder.interface(interface))
        .map_err(|_| NetworkConnectivityReadError::ProtocolViolation)?
        .cache_properties(CacheProperties::No)
        .build()
        .await
        .map_err(|_| NetworkConnectivityReadError::ConnectionFailed)
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
async fn resolve_owner(
    bus: &zbus::Proxy<'_>,
    destination: &str,
) -> Result<zbus::names::OwnedUniqueName, NetworkConnectivityReadError> {
    use zbus::proxy::MethodFlags;

    bus.call_with_flags(
        "GetNameOwner",
        MethodFlags::NoAutoStart.into(),
        &(destination,),
    )
    .await
    .map_err(|_| NetworkConnectivityReadError::OwnerUnavailable)?
    .ok_or(NetworkConnectivityReadError::OwnerUnavailable)
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
async fn require_owner_uid(
    bus: &zbus::Proxy<'_>,
    owner: &zbus::names::OwnedUniqueName,
    expected_uid: u32,
) -> Result<(), NetworkConnectivityReadError> {
    use zbus::proxy::MethodFlags;

    let uid: u32 = bus
        .call_with_flags(
            "GetConnectionUnixUser",
            MethodFlags::NoAutoStart.into(),
            &(owner.as_str(),),
        )
        .await
        .map_err(|_| NetworkConnectivityReadError::UntrustedOwner)?
        .ok_or(NetworkConnectivityReadError::UntrustedOwner)?;
    if uid == expected_uid {
        Ok(())
    } else {
        Err(NetworkConnectivityReadError::UntrustedOwner)
    }
}

#[cfg(any(all(target_os = "linux", target_env = "gnu"), test))]
fn build_network_manager_observation(
    raw: u32,
    observed_at_unix_ms: u64,
) -> Result<NetworkConnectivityObservation, NetworkConnectivityReadError> {
    let state = match raw {
        0 => NetworkConnectivity::Unknown,
        1 => NetworkConnectivity::Offline,
        2 => NetworkConnectivity::Limited,
        3 => NetworkConnectivity::Local,
        4 => NetworkConnectivity::Online,
        _ => return Err(NetworkConnectivityReadError::ProtocolViolation),
    };
    let observation = ContextObservation::new(
        ContextSource::SystemNetworkConnectivity,
        observed_at_unix_ms,
        ContextValue::Present(state),
    );
    validate_network_connectivity_observation(&observation, observed_at_unix_ms)
        .map_err(|_| NetworkConnectivityReadError::ProtocolViolation)?;
    Ok(observation)
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| {
            duration.as_millis().try_into().unwrap_or(u64::MAX)
        })
}

pub fn validate_network_connectivity_observation(
    observation: &NetworkConnectivityObservation,
    now_unix_ms: u64,
) -> Result<(), NetworkConnectivityObservationError> {
    let source = ContextSource::SystemNetworkConnectivity;
    if observation.source != source {
        return Err(NetworkConnectivityObservationError::WrongSource);
    }
    if observation.schema_version != CONTEXT_PROTOCOL_VERSION {
        return Err(NetworkConnectivityObservationError::UnsupportedSchema);
    }
    if observation.max_age_ms != source.max_age_ms() {
        return Err(NetworkConnectivityObservationError::InvalidLifetime);
    }
    let age_ms = now_unix_ms
        .checked_sub(observation.observed_at_unix_ms)
        .ok_or(NetworkConnectivityObservationError::FutureTimestamp)?;
    if age_ms > observation.max_age_ms {
        return Err(NetworkConnectivityObservationError::Stale);
    }
    if matches!(observation.observation, ContextValue::Absent) {
        return Err(NetworkConnectivityObservationError::UnexpectedAbsence);
    }
    let encoded = serde_json::to_vec(observation)
        .map_err(|_| NetworkConnectivityObservationError::ResponseTooLarge)?;
    if encoded.len() > MAX_CONTEXT_RESPONSE_BYTES {
        return Err(NetworkConnectivityObservationError::ResponseTooLarge);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW_MS: u64 = 20_000;

    fn observation(state: NetworkConnectivity) -> NetworkConnectivityObservation {
        ContextObservation::new(
            ContextSource::SystemNetworkConnectivity,
            NOW_MS,
            ContextValue::Present(state),
        )
    }

    #[test]
    fn validates_only_the_closed_present_states() {
        for state in [
            NetworkConnectivity::Offline,
            NetworkConnectivity::Local,
            NetworkConnectivity::Limited,
            NetworkConnectivity::Online,
            NetworkConnectivity::Unknown,
        ] {
            assert_eq!(
                validate_network_connectivity_observation(&observation(state), NOW_MS),
                Ok(())
            );
        }

        let absent = ContextObservation::new(
            ContextSource::SystemNetworkConnectivity,
            NOW_MS,
            ContextValue::<NetworkConnectivity>::Absent,
        );
        assert_eq!(
            validate_network_connectivity_observation(&absent, NOW_MS),
            Err(NetworkConnectivityObservationError::UnexpectedAbsence)
        );
    }

    #[test]
    fn rejects_registry_and_freshness_expansion() {
        let mut wrong_source = observation(NetworkConnectivity::Online);
        wrong_source.source = ContextSource::SystemBatterySummary;
        assert_eq!(
            validate_network_connectivity_observation(&wrong_source, NOW_MS),
            Err(NetworkConnectivityObservationError::WrongSource)
        );

        let mut wrong_schema = observation(NetworkConnectivity::Online);
        wrong_schema.schema_version += 1;
        assert_eq!(
            validate_network_connectivity_observation(&wrong_schema, NOW_MS),
            Err(NetworkConnectivityObservationError::UnsupportedSchema)
        );

        let mut wrong_lifetime = observation(NetworkConnectivity::Online);
        wrong_lifetime.max_age_ms += 1;
        assert_eq!(
            validate_network_connectivity_observation(&wrong_lifetime, NOW_MS),
            Err(NetworkConnectivityObservationError::InvalidLifetime)
        );

        let mut future = observation(NetworkConnectivity::Online);
        future.observed_at_unix_ms = NOW_MS + 1;
        assert_eq!(
            validate_network_connectivity_observation(&future, NOW_MS),
            Err(NetworkConnectivityObservationError::FutureTimestamp)
        );

        let mut stale = observation(NetworkConnectivity::Online);
        stale.observed_at_unix_ms = NOW_MS - stale.max_age_ms - 1;
        assert_eq!(
            validate_network_connectivity_observation(&stale, NOW_MS),
            Err(NetworkConnectivityObservationError::Stale)
        );
    }

    #[test]
    fn serialization_contains_no_network_identifiers() {
        let encoded = serde_json::to_string(&observation(NetworkConnectivity::Limited))
            .expect("observation serializes");
        assert!(encoded.contains("system.network.connectivity"));
        assert!(encoded.contains("limited"));
        for forbidden in [
            "ssid",
            "bssid",
            "address",
            "interface",
            "route",
            "dns",
            "traffic",
        ] {
            assert!(!encoded.to_ascii_lowercase().contains(forbidden));
        }
    }

    #[test]
    fn maps_only_documented_network_manager_connectivity_values() {
        let expected = [
            NetworkConnectivity::Unknown,
            NetworkConnectivity::Offline,
            NetworkConnectivity::Limited,
            NetworkConnectivity::Local,
            NetworkConnectivity::Online,
        ];
        for (raw, state) in expected.into_iter().enumerate() {
            let result = build_network_manager_observation(raw as u32, NOW_MS)
                .expect("documented value maps");
            assert_eq!(result.observation, ContextValue::Present(state));
        }
        assert_eq!(
            build_network_manager_observation(5, NOW_MS),
            Err(NetworkConnectivityReadError::ProtocolViolation)
        );
        assert_eq!(
            build_network_manager_observation(u32::MAX, NOW_MS),
            Err(NetworkConnectivityReadError::ProtocolViolation)
        );
    }

    #[test]
    fn provider_contract_and_constants_are_fixed() {
        assert_eq!(
            NETWORK_MANAGER_DESTINATION,
            "org.freedesktop.NetworkManager"
        );
        assert_eq!(NETWORK_MANAGER_PATH, "/org/freedesktop/NetworkManager");
        assert_eq!(NETWORK_MANAGER_INTERFACE, "org.freedesktop.NetworkManager");
        assert_eq!(DBUS_PROPERTIES_INTERFACE, "org.freedesktop.DBus.Properties");
        assert_eq!(NETWORK_CONNECTIVITY_READ_TIMEOUT_MS, 2_000);
        let mut unavailable = UnavailableNetworkConnectivityProvider;
        assert_eq!(
            unavailable.read_network_connectivity(),
            Err(NetworkConnectivityReadError::UnsupportedPlatform)
        );
    }
}
