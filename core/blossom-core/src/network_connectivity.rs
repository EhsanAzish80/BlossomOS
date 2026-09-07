use crate::context::{
    CONTEXT_PROTOCOL_VERSION, ContextObservation, ContextSource, ContextValue,
    MAX_CONTEXT_RESPONSE_BYTES,
};
use serde::Serialize;
use std::fmt;

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
}
