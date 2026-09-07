use crate::context::{
    CONTEXT_PROTOCOL_VERSION, ContextObservation, ContextSource, ContextValue,
    MAX_CONTEXT_RESPONSE_BYTES,
};
use serde::Serialize;
use std::fmt;

pub const UPOWER_DESTINATION: &str = "org.freedesktop.UPower";
pub const UPOWER_DISPLAY_DEVICE_PATH: &str = "/org/freedesktop/UPower/devices/DisplayDevice";
pub const UPOWER_DEVICE_INTERFACE: &str = "org.freedesktop.UPower.Device";
pub const DBUS_PROPERTIES_INTERFACE: &str = "org.freedesktop.DBus.Properties";
pub const SYSTEM_BUS_ADDRESS: &str = "unix:path=/run/dbus/system_bus_socket";
pub const BATTERY_READ_TIMEOUT_MS: u64 = 2_000;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BatteryReadError {
    UnsupportedPlatform,
    ConnectionFailed,
    OwnerUnavailable,
    UntrustedOwner,
    PropertyFailed,
    Timeout,
    ProtocolViolation,
}

impl fmt::Display for BatteryReadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::UnsupportedPlatform => "battery status requires Linux UPower D-Bus",
            Self::ConnectionFailed => "the local system bus is unavailable",
            Self::OwnerUnavailable => "the fixed UPower service owner is unavailable",
            Self::UntrustedOwner => "the fixed UPower service owner is not trusted",
            Self::PropertyFailed => "a fixed UPower battery property could not be read",
            Self::Timeout => "the fixed battery read deadline expired",
            Self::ProtocolViolation => "UPower returned an invalid battery summary",
        })
    }
}

impl std::error::Error for BatteryReadError {}

pub trait BatterySummaryProvider {
    fn read_battery_summary(&mut self) -> Result<BatteryObservation, BatteryReadError>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct UnavailableBatterySummaryProvider;

impl BatterySummaryProvider for UnavailableBatterySummaryProvider {
    fn read_battery_summary(&mut self) -> Result<BatteryObservation, BatteryReadError> {
        Err(BatteryReadError::UnsupportedPlatform)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct UpowerBatterySummaryProvider;

#[cfg(not(all(target_os = "linux", target_env = "gnu")))]
impl BatterySummaryProvider for UpowerBatterySummaryProvider {
    fn read_battery_summary(&mut self) -> Result<BatteryObservation, BatteryReadError> {
        Err(BatteryReadError::UnsupportedPlatform)
    }
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
impl BatterySummaryProvider for UpowerBatterySummaryProvider {
    fn read_battery_summary(&mut self) -> Result<BatteryObservation, BatteryReadError> {
        read_upower_battery_at(
            SYSTEM_BUS_ADDRESS,
            std::time::Duration::from_millis(BATTERY_READ_TIMEOUT_MS),
            0,
        )
    }
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
fn read_upower_battery_at(
    address: &str,
    timeout: std::time::Duration,
    expected_uid: u32,
) -> Result<BatteryObservation, BatteryReadError> {
    use futures_lite::future::race;

    async_io::block_on(race(
        read_upower_battery(address, expected_uid),
        async move {
            async_io::Timer::after(timeout).await;
            Err(BatteryReadError::Timeout)
        },
    ))
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
async fn read_upower_battery(
    address: &str,
    expected_uid: u32,
) -> Result<BatteryObservation, BatteryReadError> {
    use zbus::proxy::MethodFlags;
    use zbus::zvariant::OwnedValue;
    use zbus::{Proxy, connection};

    let connection = connection::Builder::address(address)
        .map_err(|_| BatteryReadError::ConnectionFailed)?
        .max_queued(8)
        .build()
        .await
        .map_err(|_| BatteryReadError::ConnectionFailed)?;
    let bus = fixed_proxy(
        &connection,
        "org.freedesktop.DBus",
        "/org/freedesktop/DBus",
        "org.freedesktop.DBus",
    )
    .await?;
    let owner_before = resolve_owner(&bus, UPOWER_DESTINATION).await?;
    require_owner_uid(&bus, &owner_before, expected_uid).await?;

    let properties = fixed_proxy(
        &connection,
        UPOWER_DESTINATION,
        UPOWER_DISPLAY_DEVICE_PATH,
        DBUS_PROPERTIES_INTERFACE,
    )
    .await?;
    async fn read(properties: &Proxy<'_>, name: &str) -> Result<OwnedValue, BatteryReadError> {
        properties
            .call_with_flags(
                "Get",
                MethodFlags::NoAutoStart.into(),
                &(UPOWER_DEVICE_INTERFACE, name),
            )
            .await
            .map_err(|_| BatteryReadError::PropertyFailed)?
            .ok_or(BatteryReadError::ProtocolViolation)
    }

    let device_type = u32::try_from(read(&properties, "Type").await?)
        .map_err(|_| BatteryReadError::ProtocolViolation)?;
    let raw_state = u32::try_from(read(&properties, "State").await?)
        .map_err(|_| BatteryReadError::ProtocolViolation)?;
    let percentage = f64::try_from(read(&properties, "Percentage").await?)
        .map_err(|_| BatteryReadError::ProtocolViolation)?;
    let is_present = bool::try_from(read(&properties, "IsPresent").await?)
        .map_err(|_| BatteryReadError::ProtocolViolation)?;

    let owner_after = resolve_owner(&bus, UPOWER_DESTINATION).await?;
    if owner_after != owner_before {
        return Err(BatteryReadError::UntrustedOwner);
    }
    require_owner_uid(&bus, &owner_after, expected_uid).await?;

    build_upower_observation(device_type, raw_state, percentage, is_present, now_ms())
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
async fn fixed_proxy<'a>(
    connection: &'a zbus::Connection,
    destination: &'a str,
    path: &'a str,
    interface: &'a str,
) -> Result<zbus::Proxy<'a>, BatteryReadError> {
    use zbus::proxy::{Builder as ProxyBuilder, CacheProperties};

    ProxyBuilder::new(connection)
        .destination(destination)
        .and_then(|builder| builder.path(path))
        .and_then(|builder| builder.interface(interface))
        .map_err(|_| BatteryReadError::ProtocolViolation)?
        .cache_properties(CacheProperties::No)
        .build()
        .await
        .map_err(|_| BatteryReadError::ConnectionFailed)
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
async fn resolve_owner(
    bus: &zbus::Proxy<'_>,
    destination: &str,
) -> Result<zbus::names::OwnedUniqueName, BatteryReadError> {
    use zbus::proxy::MethodFlags;

    bus.call_with_flags(
        "GetNameOwner",
        MethodFlags::NoAutoStart.into(),
        &(destination,),
    )
    .await
    .map_err(|_| BatteryReadError::OwnerUnavailable)?
    .ok_or(BatteryReadError::OwnerUnavailable)
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
async fn require_owner_uid(
    bus: &zbus::Proxy<'_>,
    owner: &zbus::names::OwnedUniqueName,
    expected_uid: u32,
) -> Result<(), BatteryReadError> {
    use zbus::proxy::MethodFlags;

    let uid: u32 = bus
        .call_with_flags(
            "GetConnectionUnixUser",
            MethodFlags::NoAutoStart.into(),
            &(owner.as_str(),),
        )
        .await
        .map_err(|_| BatteryReadError::UntrustedOwner)?
        .ok_or(BatteryReadError::UntrustedOwner)?;
    if uid == expected_uid {
        Ok(())
    } else {
        Err(BatteryReadError::UntrustedOwner)
    }
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
fn build_upower_observation(
    device_type: u32,
    raw_state: u32,
    percentage: f64,
    is_present: bool,
    observed_at_unix_ms: u64,
) -> Result<BatteryObservation, BatteryReadError> {
    if raw_state > 6 || !percentage.is_finite() || !(0.0..=100.0).contains(&percentage) {
        return Err(BatteryReadError::ProtocolViolation);
    }
    if !is_present {
        if !matches!(device_type, 0 | 2) {
            return Err(BatteryReadError::ProtocolViolation);
        }
        let observation = ContextObservation::new(
            ContextSource::SystemBatterySummary,
            observed_at_unix_ms,
            ContextValue::Absent,
        );
        validate_battery_observation(&observation, observed_at_unix_ms)
            .map_err(|_| BatteryReadError::ProtocolViolation)?;
        return Ok(observation);
    }
    if device_type != 2 {
        return Err(BatteryReadError::ProtocolViolation);
    }
    let state = match raw_state {
        0 => BatteryState::Unknown,
        1 => BatteryState::Charging,
        2 => BatteryState::Discharging,
        3 | 5 | 6 => BatteryState::NotCharging,
        4 => BatteryState::Full,
        _ => return Err(BatteryReadError::ProtocolViolation),
    };
    let observation = ContextObservation::new(
        ContextSource::SystemBatterySummary,
        observed_at_unix_ms,
        ContextValue::Present(BatterySummary {
            percentage: percentage.round() as u8,
            state,
        }),
    );
    validate_battery_observation(&observation, observed_at_unix_ms)
        .map_err(|_| BatteryReadError::ProtocolViolation)?;
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

    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    #[test]
    fn maps_only_the_fixed_upower_display_schema() {
        let present =
            build_upower_observation(2, 2, 72.6, true, NOW_MS).expect("valid display battery");
        assert_eq!(
            present.observation,
            ContextValue::Present(BatterySummary {
                percentage: 73,
                state: BatteryState::Discharging,
            })
        );
        for (raw, expected) in [
            (0, BatteryState::Unknown),
            (1, BatteryState::Charging),
            (3, BatteryState::NotCharging),
            (4, BatteryState::Full),
            (5, BatteryState::NotCharging),
            (6, BatteryState::NotCharging),
        ] {
            let observation =
                build_upower_observation(2, raw, 50.0, true, NOW_MS).expect("known UPower state");
            assert!(matches!(
                observation.observation,
                ContextValue::Present(BatterySummary { state, .. }) if state == expected
            ));
        }
        for invalid in [
            build_upower_observation(1, 1, 50.0, true, NOW_MS),
            build_upower_observation(2, 7, 50.0, true, NOW_MS),
            build_upower_observation(2, 1, -0.1, true, NOW_MS),
            build_upower_observation(2, 1, 100.1, true, NOW_MS),
            build_upower_observation(2, 1, f64::NAN, true, NOW_MS),
        ] {
            assert_eq!(invalid, Err(BatteryReadError::ProtocolViolation));
        }

        let absent = build_upower_observation(0, 0, 0.0, false, NOW_MS)
            .expect("absence ignores non-value fields");
        assert_eq!(absent.observation, ContextValue::Absent);
        assert_eq!(
            build_upower_observation(1, 0, 0.0, false, NOW_MS),
            Err(BatteryReadError::ProtocolViolation)
        );
    }

    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    mod linux_dbus {
        use super::*;
        use std::io::{BufRead, BufReader};
        use std::process::{Child, Command, Stdio};
        use std::sync::{
            Arc, Mutex,
            atomic::{AtomicBool, Ordering},
        };

        struct TestBus(Child);

        impl Drop for TestBus {
            fn drop(&mut self) {
                let _ = self.0.kill();
                let _ = self.0.wait();
            }
        }

        fn start_bus() -> (TestBus, String) {
            let mut child = Command::new("dbus-daemon")
                .args(["--session", "--nofork", "--print-address=1"])
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .expect("dbus-daemon is available on the Linux CI image");
            let stdout = child.stdout.take().expect("captured address");
            let mut reader = BufReader::new(stdout);
            let mut address = String::new();
            reader.read_line(&mut address).expect("bus address");
            let address = address.trim().to_string();
            assert!(!address.is_empty());
            (TestBus(child), address)
        }

        struct DisplayDevice {
            calls: Arc<Mutex<Vec<&'static str>>>,
            present: Arc<AtomicBool>,
            slow: Arc<AtomicBool>,
        }

        impl DisplayDevice {
            fn record(&self, property: &'static str) {
                self.calls.lock().expect("call log").push(property);
            }
        }

        #[zbus::interface(name = "org.freedesktop.UPower.Device")]
        impl DisplayDevice {
            #[zbus(property, name = "Type")]
            fn device_type(&self) -> u32 {
                self.record("Type");
                if self.present.load(Ordering::SeqCst) {
                    2
                } else {
                    0
                }
            }

            #[zbus(property, name = "State")]
            fn state(&self) -> u32 {
                self.record("State");
                if self.slow.load(Ordering::SeqCst) {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
                1
            }

            #[zbus(property, name = "Percentage")]
            fn percentage(&self) -> f64 {
                self.record("Percentage");
                62.4
            }

            #[zbus(property, name = "IsPresent")]
            fn is_present(&self) -> bool {
                self.record("IsPresent");
                self.present.load(Ordering::SeqCst)
            }
        }

        #[test]
        fn uses_only_fixed_properties_and_rejects_wrong_owner_or_timeout() {
            let (_bus, address) = start_bus();
            let calls = Arc::new(Mutex::new(Vec::new()));
            let present = Arc::new(AtomicBool::new(true));
            let slow = Arc::new(AtomicBool::new(false));
            let _service = zbus::blocking::connection::Builder::address(address.as_str())
                .expect("test address")
                .name(UPOWER_DESTINATION)
                .expect("fixed destination")
                .serve_at(
                    UPOWER_DISPLAY_DEVICE_PATH,
                    DisplayDevice {
                        calls: Arc::clone(&calls),
                        present: Arc::clone(&present),
                        slow: Arc::clone(&slow),
                    },
                )
                .expect("fixed display device")
                .build()
                .expect("mock UPower service");

            let uid = nix::unistd::geteuid().as_raw();
            let observation =
                read_upower_battery_at(&address, std::time::Duration::from_secs(2), uid)
                    .expect("fixed battery read");
            assert!(matches!(
                observation.observation,
                ContextValue::Present(BatterySummary {
                    percentage: 62,
                    state: BatteryState::Charging,
                })
            ));
            assert_eq!(
                *calls.lock().expect("call log"),
                ["Type", "State", "Percentage", "IsPresent"]
            );

            assert_eq!(
                read_upower_battery_at(
                    &address,
                    std::time::Duration::from_secs(2),
                    uid.wrapping_add(1),
                ),
                Err(BatteryReadError::UntrustedOwner)
            );

            present.store(false, Ordering::SeqCst);
            let absent = read_upower_battery_at(&address, std::time::Duration::from_secs(2), uid)
                .expect("verified battery absence");
            assert_eq!(absent.observation, ContextValue::Absent);

            slow.store(true, Ordering::SeqCst);
            assert_eq!(
                read_upower_battery_at(&address, std::time::Duration::from_millis(20), uid,),
                Err(BatteryReadError::Timeout)
            );
        }

        #[test]
        fn unavailable_owner_and_bus_fail_closed() {
            let (_bus, address) = start_bus();
            assert_eq!(
                read_upower_battery_at(
                    &address,
                    std::time::Duration::from_millis(100),
                    nix::unistd::geteuid().as_raw(),
                ),
                Err(BatteryReadError::OwnerUnavailable)
            );
            assert_eq!(
                read_upower_battery_at(
                    "unix:path=/run/blossom-missing-system-bus",
                    std::time::Duration::from_millis(100),
                    0,
                ),
                Err(BatteryReadError::ConnectionFailed)
            );
        }
    }
}
