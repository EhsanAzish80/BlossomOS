use serde::{Deserialize, Serialize};
use std::fmt;

pub const SYSTEMD_DESTINATION: &str = "org.freedesktop.systemd1";
pub const SYSTEMD_MANAGER_PATH: &str = "/org/freedesktop/systemd1";
pub const SYSTEMD_MANAGER_INTERFACE: &str = "org.freedesktop.systemd1.Manager";
pub const SYSTEMD_UNIT_INTERFACE: &str = "org.freedesktop.systemd1.Unit";
pub const SYSTEM_BUS_ADDRESS: &str = "unix:path=/run/dbus/system_bus_socket";
pub const MAX_SERVICE_UNIT_BYTES: usize = 256;
pub const MAX_SERVICE_STATE_BYTES: usize = 64;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceSelection {
    pub unit: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ServiceStatus {
    pub requested_unit: String,
    pub scope: String,
    pub canonical_unit: String,
    pub load_state: String,
    pub active_state: String,
    pub sub_state: String,
    pub destination: String,
    pub manager_interface: String,
    pub unit_interface: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceStatusError {
    UnsupportedPlatform,
    InvalidUnit,
    ConnectionFailed,
    UnitUnavailable,
    LookupFailed,
    PropertyFailed,
    Timeout,
    ProtocolViolation,
}

impl fmt::Display for ServiceStatusError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::UnsupportedPlatform => "service status requires Linux systemd D-Bus",
            Self::InvalidUnit => "invalid exact systemd service unit",
            Self::ConnectionFailed => "the local system bus is unavailable",
            Self::UnitUnavailable => "the exact service unit is not currently loaded",
            Self::LookupFailed => "the exact service unit lookup failed",
            Self::PropertyFailed => "an approved service status property could not be read",
            Self::Timeout => "the fixed service status deadline expired",
            Self::ProtocolViolation => "systemd returned an invalid bounded status result",
        })
    }
}

impl std::error::Error for ServiceStatusError {}

pub trait ServiceStatusProvider {
    fn read_status(&mut self, unit: &str) -> Result<ServiceStatus, ServiceStatusError>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct UnavailableServiceStatusProvider;

impl ServiceStatusProvider for UnavailableServiceStatusProvider {
    fn read_status(&mut self, _: &str) -> Result<ServiceStatus, ServiceStatusError> {
        Err(ServiceStatusError::UnsupportedPlatform)
    }
}

pub fn validate_service_unit(unit: &str) -> Result<(), ServiceStatusError> {
    let Some(stem) = unit.strip_suffix(".service") else {
        return Err(ServiceStatusError::InvalidUnit);
    };
    let valid = !stem.is_empty()
        && !stem.starts_with('.')
        && !stem.ends_with('.')
        && !stem.contains("..")
        && unit.len() <= MAX_SERVICE_UNIT_BYTES
        && stem.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b':' | b'_' | b'.' | b'@' | b'-')
        });
    if valid {
        Ok(())
    } else {
        Err(ServiceStatusError::InvalidUnit)
    }
}

pub fn validate_service_status(
    status: &ServiceStatus,
    expected_unit: &str,
) -> Result<(), ServiceStatusError> {
    validate_service_unit(expected_unit)?;
    validate_service_unit(&status.requested_unit)?;
    validate_service_unit(&status.canonical_unit)?;
    if status.requested_unit != expected_unit
        || status.scope != "system"
        || status.destination != SYSTEMD_DESTINATION
        || status.manager_interface != SYSTEMD_MANAGER_INTERFACE
        || status.unit_interface != SYSTEMD_UNIT_INTERFACE
        || !valid_state(&status.load_state)
        || !valid_state(&status.active_state)
        || !valid_state(&status.sub_state)
    {
        return Err(ServiceStatusError::ProtocolViolation);
    }
    Ok(())
}

fn valid_state(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_SERVICE_STATE_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemdServiceStatusProvider;

#[cfg(not(all(target_os = "linux", target_env = "gnu")))]
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemdServiceStatusProvider;

#[cfg(not(all(target_os = "linux", target_env = "gnu")))]
impl ServiceStatusProvider for SystemdServiceStatusProvider {
    fn read_status(&mut self, _: &str) -> Result<ServiceStatus, ServiceStatusError> {
        Err(ServiceStatusError::UnsupportedPlatform)
    }
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
impl ServiceStatusProvider for SystemdServiceStatusProvider {
    fn read_status(&mut self, unit: &str) -> Result<ServiceStatus, ServiceStatusError> {
        read_systemd_status_at(SYSTEM_BUS_ADDRESS, unit)
    }
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
fn read_systemd_status_at(address: &str, unit: &str) -> Result<ServiceStatus, ServiceStatusError> {
    read_systemd_status_at_with_timeout(address, unit, std::time::Duration::from_secs(3))
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
fn read_systemd_status_at_with_timeout(
    address: &str,
    unit: &str,
    timeout: std::time::Duration,
) -> Result<ServiceStatus, ServiceStatusError> {
    use futures_lite::future::race;

    async_io::block_on(race(read_systemd_status(address, unit), async move {
        async_io::Timer::after(timeout).await;
        Err(ServiceStatusError::Timeout)
    }))
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
async fn read_systemd_status(
    address: &str,
    unit: &str,
) -> Result<ServiceStatus, ServiceStatusError> {
    use zbus::proxy::{CacheProperties, MethodFlags};
    use zbus::zvariant::{OwnedObjectPath, OwnedValue};
    use zbus::{Proxy, connection, proxy::Builder as ProxyBuilder};

    validate_service_unit(unit)?;
    let connection = connection::Builder::address(address)
        .map_err(|_| ServiceStatusError::ConnectionFailed)?
        .max_queued(8)
        .build()
        .await
        .map_err(map_connection_error)?;
    let manager: Proxy<'_> = ProxyBuilder::new(&connection)
        .destination(SYSTEMD_DESTINATION)
        .map_err(|_| ServiceStatusError::ProtocolViolation)?
        .path(SYSTEMD_MANAGER_PATH)
        .map_err(|_| ServiceStatusError::ProtocolViolation)?
        .interface(SYSTEMD_MANAGER_INTERFACE)
        .map_err(|_| ServiceStatusError::ProtocolViolation)?
        .cache_properties(CacheProperties::No)
        .build()
        .await
        .map_err(map_connection_error)?;
    let path: OwnedObjectPath = manager
        .call_with_flags("GetUnit", MethodFlags::NoAutoStart.into(), &(unit,))
        .await
        .map_err(map_lookup_error)?
        .ok_or(ServiceStatusError::ProtocolViolation)?;
    let properties: Proxy<'_> = ProxyBuilder::new(&connection)
        .destination(SYSTEMD_DESTINATION)
        .map_err(|_| ServiceStatusError::ProtocolViolation)?
        .path(path)
        .map_err(|_| ServiceStatusError::ProtocolViolation)?
        .interface("org.freedesktop.DBus.Properties")
        .map_err(|_| ServiceStatusError::ProtocolViolation)?
        .cache_properties(CacheProperties::No)
        .build()
        .await
        .map_err(map_connection_error)?;
    async fn read(properties: &Proxy<'_>, name: &str) -> Result<String, ServiceStatusError> {
        let value: OwnedValue = properties
            .call_with_flags(
                "Get",
                MethodFlags::NoAutoStart.into(),
                &(SYSTEMD_UNIT_INTERFACE, name),
            )
            .await
            .map_err(map_property_error)?
            .ok_or(ServiceStatusError::ProtocolViolation)?;
        String::try_from(value).map_err(|_| ServiceStatusError::ProtocolViolation)
    }
    let status = ServiceStatus {
        requested_unit: unit.into(),
        scope: "system".into(),
        canonical_unit: read(&properties, "Id").await?,
        load_state: read(&properties, "LoadState").await?,
        active_state: read(&properties, "ActiveState").await?,
        sub_state: read(&properties, "SubState").await?,
        destination: SYSTEMD_DESTINATION.into(),
        manager_interface: SYSTEMD_MANAGER_INTERFACE.into(),
        unit_interface: SYSTEMD_UNIT_INTERFACE.into(),
    };
    validate_service_status(&status, unit)?;
    Ok(status)
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
fn map_connection_error(_: zbus::Error) -> ServiceStatusError {
    ServiceStatusError::ConnectionFailed
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
fn map_lookup_error(error: zbus::Error) -> ServiceStatusError {
    if error.to_string().contains("NoSuchUnit") {
        ServiceStatusError::UnitUnavailable
    } else {
        ServiceStatusError::LookupFailed
    }
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
fn map_property_error(_: zbus::Error) -> ServiceStatusError {
    ServiceStatusError::PropertyFailed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status() -> ServiceStatus {
        ServiceStatus {
            requested_unit: "sshd.service".into(),
            scope: "system".into(),
            canonical_unit: "sshd.service".into(),
            load_state: "loaded".into(),
            active_state: "active".into(),
            sub_state: "running".into(),
            destination: SYSTEMD_DESTINATION.into(),
            manager_interface: SYSTEMD_MANAGER_INTERFACE.into(),
            unit_interface: SYSTEMD_UNIT_INTERFACE.into(),
        }
    }

    #[test]
    fn accepts_only_conservative_exact_service_units() {
        for valid in [
            "sshd.service",
            "dbus-org.example.service",
            "worker@1.service",
        ] {
            assert_eq!(validate_service_unit(valid), Ok(()));
        }
        for invalid in [
            "",
            "service",
            ".service",
            "foo.socket",
            "../foo.service",
            "foo/bar.service",
            "foo\\x2dbar.service",
            "foo*.service",
            "foo service",
            "foo\0.service",
            ".hidden.service",
            "foo..bar.service",
        ] {
            assert_eq!(
                validate_service_unit(invalid),
                Err(ServiceStatusError::InvalidUnit),
                "{invalid:?}"
            );
        }
        let oversized = format!("{}.service", "a".repeat(MAX_SERVICE_UNIT_BYTES));
        assert_eq!(
            validate_service_unit(&oversized),
            Err(ServiceStatusError::InvalidUnit)
        );
    }

    #[test]
    fn validates_exact_scope_fixed_provenance_and_opaque_bounded_states() {
        assert_eq!(validate_service_status(&status(), "sshd.service"), Ok(()));
        let mut changed = status();
        changed.active_state = "future-state".into();
        assert_eq!(validate_service_status(&changed, "sshd.service"), Ok(()));
        changed.destination = "org.example.Other".into();
        assert_eq!(
            validate_service_status(&changed, "sshd.service"),
            Err(ServiceStatusError::ProtocolViolation)
        );
        let mut control = status();
        control.sub_state = "bad\nstate".into();
        assert_eq!(
            validate_service_status(&control, "sshd.service"),
            Err(ServiceStatusError::ProtocolViolation)
        );
    }

    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    mod linux_dbus {
        use super::*;
        use std::io::{BufRead, BufReader};
        use std::process::{Child, Command, Stdio};
        use zbus::zvariant::OwnedObjectPath;

        const TEST_UNIT_PATH: &str = "/org/freedesktop/systemd1/unit/blossom_2dtest_2eservice";

        struct TestBus(Child);
        impl Drop for TestBus {
            fn drop(&mut self) {
                let _ = self.0.kill();
                let _ = self.0.wait();
            }
        }

        struct Manager;
        #[zbus::interface(name = "org.freedesktop.systemd1.Manager")]
        impl Manager {
            #[zbus(name = "GetUnit")]
            fn get_unit(&self, name: &str) -> zbus::fdo::Result<OwnedObjectPath> {
                if name == "slow.service" {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
                if name != "blossom-test.service" {
                    return Err(zbus::fdo::Error::Failed("unexpected exact unit".into()));
                }
                OwnedObjectPath::try_from(TEST_UNIT_PATH)
                    .map_err(|_| zbus::fdo::Error::Failed("invalid test path".into()))
            }
        }

        struct Unit;
        #[zbus::interface(name = "org.freedesktop.systemd1.Unit")]
        impl Unit {
            #[zbus(property, name = "Id")]
            fn id(&self) -> &str {
                "blossom-test.service"
            }

            #[zbus(property, name = "LoadState")]
            fn load_state(&self) -> &str {
                "loaded"
            }

            #[zbus(property, name = "ActiveState")]
            fn active_state(&self) -> &str {
                "active"
            }

            #[zbus(property, name = "SubState")]
            fn sub_state(&self) -> &str {
                "running"
            }
        }

        #[test]
        fn target_linux_uses_only_the_fixed_mock_systemd_surface() {
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
            let _bus = TestBus(child);

            let _service = zbus::blocking::connection::Builder::address(address.as_str())
                .expect("test address")
                .name(SYSTEMD_DESTINATION)
                .expect("fixed destination")
                .serve_at(SYSTEMD_MANAGER_PATH, Manager)
                .expect("manager object")
                .serve_at(TEST_UNIT_PATH, Unit)
                .expect("unit object")
                .build()
                .expect("mock systemd service");

            let result =
                read_systemd_status_at(&address, "blossom-test.service").expect("fixed D-Bus read");
            assert_eq!(
                result,
                ServiceStatus {
                    requested_unit: "blossom-test.service".into(),
                    scope: "system".into(),
                    canonical_unit: "blossom-test.service".into(),
                    load_state: "loaded".into(),
                    active_state: "active".into(),
                    sub_state: "running".into(),
                    destination: SYSTEMD_DESTINATION.into(),
                    manager_interface: SYSTEMD_MANAGER_INTERFACE.into(),
                    unit_interface: SYSTEMD_UNIT_INTERFACE.into(),
                }
            );
            assert_eq!(
                read_systemd_status_at(&address, "missing.service"),
                Err(ServiceStatusError::LookupFailed)
            );
            assert_eq!(
                read_systemd_status_at_with_timeout(
                    &address,
                    "slow.service",
                    std::time::Duration::from_millis(20),
                ),
                Err(ServiceStatusError::Timeout)
            );
        }

        #[test]
        fn target_linux_observes_real_systemd_only_when_the_runner_has_one() {
            if !std::path::Path::new("/run/systemd/system").is_dir()
                || !std::path::Path::new("/run/dbus/system_bus_socket").exists()
            {
                return;
            }
            let status = SystemdServiceStatusProvider
                .read_status("dbus.service")
                .expect("usable systemd manager exposes its loaded D-Bus service");
            validate_service_status(&status, "dbus.service").expect("bounded real status");
        }
    }
}
