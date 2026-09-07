use crate::Capability;
use serde::Serialize;

pub const CONTEXT_PROTOCOL_VERSION: u16 = 1;
pub const BATTERY_MAX_AGE_MS: u64 = 5_000;
pub const BATTERY_MIN_POLL_INTERVAL_MS: u64 = 1_000;
pub const MAX_CONTEXT_RESPONSE_BYTES: usize = 4 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum ContextSource {
    #[serde(rename = "system.battery.summary")]
    SystemBatterySummary,
}

impl ContextSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SystemBatterySummary => "system.battery.summary",
        }
    }

    pub fn capability(self) -> Capability {
        match self {
            Self::SystemBatterySummary => Capability::SystemReadBatterySummary,
        }
    }

    pub fn schema_version(self) -> u16 {
        CONTEXT_PROTOCOL_VERSION
    }

    pub fn max_age_ms(self) -> u64 {
        match self {
            Self::SystemBatterySummary => BATTERY_MAX_AGE_MS,
        }
    }

    pub fn min_poll_interval_ms(self) -> u64 {
        match self {
            Self::SystemBatterySummary => BATTERY_MIN_POLL_INTERVAL_MS,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "status", content = "value", rename_all = "snake_case")]
pub enum ContextValue<T> {
    Present(T),
    Absent,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ContextObservation<T> {
    pub source: ContextSource,
    pub schema_version: u16,
    pub observed_at_unix_ms: u64,
    pub max_age_ms: u64,
    pub observation: ContextValue<T>,
}

impl<T> ContextObservation<T> {
    pub fn new(
        source: ContextSource,
        observed_at_unix_ms: u64,
        observation: ContextValue<T>,
    ) -> Self {
        Self {
            source,
            schema_version: source.schema_version(),
            observed_at_unix_ms,
            max_age_ms: source.max_age_ms(),
            observation,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn battery_registry_metadata_is_fixed() {
        let source = ContextSource::SystemBatterySummary;
        assert_eq!(source.as_str(), "system.battery.summary");
        assert_eq!(source.capability().as_str(), "system.read:battery.summary");
        assert_eq!(source.schema_version(), 1);
        assert_eq!(source.max_age_ms(), 5_000);
        assert_eq!(source.min_poll_interval_ms(), 1_000);
        assert_eq!(MAX_CONTEXT_RESPONSE_BYTES, 4_096);
        assert_eq!(
            serde_json::to_string(&source).expect("source serializes"),
            "\"system.battery.summary\""
        );
    }

    #[test]
    fn battery_capability_is_denied_unless_explicitly_allowed() {
        use crate::{PolicyDecision, PolicyEngine, PolicyRule};

        let capability = ContextSource::SystemBatterySummary.capability();
        assert_eq!(
            PolicyEngine::default().evaluate_capability(capability),
            PolicyDecision::Deny
        );

        let policy = PolicyEngine::new(vec![PolicyRule {
            capability,
            decision: PolicyDecision::Allow,
        }]);
        assert_eq!(
            policy.evaluate_capability(capability),
            PolicyDecision::Allow
        );
    }
}
