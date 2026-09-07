use crate::context::{
    CONTEXT_PROTOCOL_VERSION, ContextObservation, ContextSource, ContextValue,
    MAX_CONTEXT_RESPONSE_BYTES,
};
use serde::Serialize;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BatteryState {
    Charging,
    Discharging,
    Full,
    NotCharging,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct BatterySummary {
    pub percentage: u8,
    pub state: BatteryState,
}

pub type BatteryObservation = ContextObservation<BatterySummary>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BatteryObservationError {
    WrongSource,
    UnsupportedSchema,
    InvalidLifetime,
    FutureTimestamp,
    Stale,
    InvalidPercentage,
    ResponseTooLarge,
}

impl fmt::Display for BatteryObservationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::WrongSource => "battery observation has the wrong source",
            Self::UnsupportedSchema => "battery observation schema is unsupported",
            Self::InvalidLifetime => "battery observation lifetime is not code-owned",
            Self::FutureTimestamp => "battery observation timestamp is in the future",
            Self::Stale => "battery observation is stale",
            Self::InvalidPercentage => "battery percentage is outside the supported range",
            Self::ResponseTooLarge => "battery observation exceeds the response limit",
        })
    }
}

impl std::error::Error for BatteryObservationError {}

pub fn validate_battery_observation(
    observation: &BatteryObservation,
    now_unix_ms: u64,
) -> Result<(), BatteryObservationError> {
    let source = ContextSource::SystemBatterySummary;
    if observation.source != source {
        return Err(BatteryObservationError::WrongSource);
    }
    if observation.schema_version != CONTEXT_PROTOCOL_VERSION {
        return Err(BatteryObservationError::UnsupportedSchema);
    }
    if observation.max_age_ms != source.max_age_ms() {
        return Err(BatteryObservationError::InvalidLifetime);
    }
    let age_ms = now_unix_ms
        .checked_sub(observation.observed_at_unix_ms)
        .ok_or(BatteryObservationError::FutureTimestamp)?;
    if age_ms > observation.max_age_ms {
        return Err(BatteryObservationError::Stale);
    }
    if let ContextValue::Present(summary) = observation.observation
        && summary.percentage > 100
    {
        return Err(BatteryObservationError::InvalidPercentage);
    }
    let encoded =
        serde_json::to_vec(observation).map_err(|_| BatteryObservationError::ResponseTooLarge)?;
    if encoded.len() > MAX_CONTEXT_RESPONSE_BYTES {
        return Err(BatteryObservationError::ResponseTooLarge);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW_MS: u64 = 10_000;

    fn present(percentage: u8, state: BatteryState) -> BatteryObservation {
        ContextObservation::new(
            ContextSource::SystemBatterySummary,
            NOW_MS,
            ContextValue::Present(BatterySummary { percentage, state }),
        )
    }

    #[test]
    fn validates_present_and_absent_observations() {
        for state in [
            BatteryState::Charging,
            BatteryState::Discharging,
            BatteryState::Full,
            BatteryState::NotCharging,
            BatteryState::Unknown,
        ] {
            assert_eq!(
                validate_battery_observation(&present(100, state), NOW_MS),
                Ok(())
            );
        }
        let absent = ContextObservation::new(
            ContextSource::SystemBatterySummary,
            NOW_MS,
            ContextValue::<BatterySummary>::Absent,
        );
        assert_eq!(validate_battery_observation(&absent, NOW_MS), Ok(()));
    }

    #[test]
    fn rejects_invalid_percentage_and_registry_metadata() {
        assert_eq!(
            validate_battery_observation(&present(101, BatteryState::Unknown), NOW_MS),
            Err(BatteryObservationError::InvalidPercentage)
        );

        let mut wrong_schema = present(50, BatteryState::Charging);
        wrong_schema.schema_version += 1;
        assert_eq!(
            validate_battery_observation(&wrong_schema, NOW_MS),
            Err(BatteryObservationError::UnsupportedSchema)
        );

        let mut wrong_lifetime = present(50, BatteryState::Charging);
        wrong_lifetime.max_age_ms += 1;
        assert_eq!(
            validate_battery_observation(&wrong_lifetime, NOW_MS),
            Err(BatteryObservationError::InvalidLifetime)
        );
    }

    #[test]
    fn rejects_future_and_stale_observations() {
        let mut future = present(50, BatteryState::Discharging);
        future.observed_at_unix_ms = NOW_MS + 1;
        assert_eq!(
            validate_battery_observation(&future, NOW_MS),
            Err(BatteryObservationError::FutureTimestamp)
        );

        let mut stale = present(50, BatteryState::Discharging);
        stale.observed_at_unix_ms = NOW_MS - stale.max_age_ms - 1;
        assert_eq!(
            validate_battery_observation(&stale, NOW_MS),
            Err(BatteryObservationError::Stale)
        );

        let mut boundary = present(50, BatteryState::Discharging);
        boundary.observed_at_unix_ms = NOW_MS - boundary.max_age_ms;
        assert_eq!(validate_battery_observation(&boundary, NOW_MS), Ok(()));
    }

    #[test]
    fn serialized_projection_contains_no_device_identity_or_history() {
        let encoded = serde_json::to_string(&present(42, BatteryState::Charging))
            .expect("battery observation serializes");
        for forbidden in [
            "manufacturer",
            "model",
            "serial",
            "device_path",
            "history",
            "temperature",
            "voltage",
            "time_to_empty",
        ] {
            assert!(!encoded.contains(forbidden));
        }
        assert!(encoded.len() <= MAX_CONTEXT_RESPONSE_BYTES);
    }
}
