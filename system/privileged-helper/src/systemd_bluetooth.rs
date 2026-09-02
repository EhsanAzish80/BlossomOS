use crate::{BluetoothManager, ManagerError, RestartCompletion};
use blossom_core::privileged::{BLUETOOTH_UNIT, BluetoothObservation, SYSTEMD_JOB_MODE};
use futures_lite::{StreamExt, future::race};
use std::time::Duration;
use zbus::proxy::{Builder as ProxyBuilder, CacheProperties, MethodFlags};
use zbus::zvariant::{OwnedObjectPath, OwnedValue};
use zbus::{Proxy, connection};

const SYSTEMD_DESTINATION: &str = "org.freedesktop.systemd1";
const SYSTEMD_MANAGER_PATH: &str = "/org/freedesktop/systemd1";
const SYSTEMD_MANAGER_INTERFACE: &str = "org.freedesktop.systemd1.Manager";
const SYSTEMD_UNIT_INTERFACE: &str = "org.freedesktop.systemd1.Unit";
const PROPERTIES_INTERFACE: &str = "org.freedesktop.DBus.Properties";
const SYSTEM_BUS_ADDRESS: &str = "unix:path=/run/dbus/system_bus_socket";
const COMPLETE_TIMEOUT: Duration = Duration::from_secs(20);
const MAX_JOB_RESULT_BYTES: usize = 64;

/// Native systemd adapter for one code-owned operation and unit.
///
/// It accepts no executable, unit, method, mode, path, or bus address from a
/// caller. The test-only constructor is compiled solely for Linux unit tests.
#[derive(Debug)]
pub struct SystemdBluetoothManager {
    address: String,
    timeout: Duration,
}

impl Default for SystemdBluetoothManager {
    fn default() -> Self {
        Self {
            address: SYSTEM_BUS_ADDRESS.into(),
            timeout: COMPLETE_TIMEOUT,
        }
    }
}

impl BluetoothManager for SystemdBluetoothManager {
    fn observe(&mut self) -> Result<BluetoothObservation, ManagerError> {
        async_io::block_on(with_timeout(self.timeout, observe_at(&self.address)))
    }

    fn try_restart(&mut self) -> Result<RestartCompletion, ManagerError> {
        async_io::block_on(with_timeout(self.timeout, try_restart_at(&self.address)))
    }
}

async fn with_timeout<T>(
    timeout: Duration,
    operation: impl Future<Output = Result<T, ManagerError>>,
) -> Result<T, ManagerError> {
    race(operation, async move {
        async_io::Timer::after(timeout).await;
        Err(ManagerError::Timeout)
    })
    .await
}

async fn connect(address: &str) -> Result<zbus::Connection, ManagerError> {
    connection::Builder::address(address)
        .map_err(|_| ManagerError::ProtocolViolation)?
        .max_queued(16)
        .method_timeout(COMPLETE_TIMEOUT)
        .build()
        .await
        .map_err(|_| ManagerError::Disconnected)
}

async fn manager_proxy(connection: &zbus::Connection) -> Result<Proxy<'_>, ManagerError> {
    ProxyBuilder::new(connection)
        .destination(SYSTEMD_DESTINATION)
        .map_err(|_| ManagerError::ProtocolViolation)?
        .path(SYSTEMD_MANAGER_PATH)
        .map_err(|_| ManagerError::ProtocolViolation)?
        .interface(SYSTEMD_MANAGER_INTERFACE)
        .map_err(|_| ManagerError::ProtocolViolation)?
        .cache_properties(CacheProperties::No)
        .build()
        .await
        .map_err(|_| ManagerError::Disconnected)
}

async fn observe_at(address: &str) -> Result<BluetoothObservation, ManagerError> {
    let connection = connect(address).await?;
    observe_on(&connection).await
}

async fn observe_on(connection: &zbus::Connection) -> Result<BluetoothObservation, ManagerError> {
    let manager = manager_proxy(connection).await?;
    let path: OwnedObjectPath = manager
        .call_with_flags(
            "GetUnit",
            MethodFlags::NoAutoStart.into(),
            &(BLUETOOTH_UNIT,),
        )
        .await
        .map_err(map_get_unit_error)?
        .ok_or(ManagerError::ProtocolViolation)?;
    let properties: Proxy<'_> = ProxyBuilder::new(connection)
        .destination(SYSTEMD_DESTINATION)
        .map_err(|_| ManagerError::ProtocolViolation)?
        .path(path)
        .map_err(|_| ManagerError::ProtocolViolation)?
        .interface(PROPERTIES_INTERFACE)
        .map_err(|_| ManagerError::ProtocolViolation)?
        .cache_properties(CacheProperties::No)
        .build()
        .await
        .map_err(|_| ManagerError::Disconnected)?;
    let observation = BluetoothObservation {
        canonical_unit: read_string(&properties, "Id").await?,
        load_state: read_string(&properties, "LoadState").await?,
        active_state: read_string(&properties, "ActiveState").await?,
        invocation_id: read_invocation_id(&properties).await?,
    };
    observation
        .validate()
        .map_err(|_| ManagerError::ProtocolViolation)?;
    Ok(observation)
}

async fn read_value(properties: &Proxy<'_>, name: &str) -> Result<OwnedValue, ManagerError> {
    properties
        .call_with_flags(
            "Get",
            MethodFlags::NoAutoStart.into(),
            &(SYSTEMD_UNIT_INTERFACE, name),
        )
        .await
        .map_err(|_| ManagerError::ProtocolViolation)?
        .ok_or(ManagerError::ProtocolViolation)
}

async fn read_string(properties: &Proxy<'_>, name: &str) -> Result<String, ManagerError> {
    String::try_from(read_value(properties, name).await?)
        .map_err(|_| ManagerError::ProtocolViolation)
}

async fn read_invocation_id(properties: &Proxy<'_>) -> Result<[u8; 16], ManagerError> {
    let bytes = Vec::<u8>::try_from(read_value(properties, "InvocationID").await?)
        .map_err(|_| ManagerError::ProtocolViolation)?;
    bytes
        .try_into()
        .map_err(|_| ManagerError::ProtocolViolation)
}

async fn try_restart_at(address: &str) -> Result<RestartCompletion, ManagerError> {
    let connection = connect(address).await?;
    let manager = manager_proxy(&connection).await?;
    let mut removed = manager
        .receive_signal_with_args("JobRemoved", &[(2, BLUETOOTH_UNIT)])
        .await
        .map_err(|_| ManagerError::Disconnected)?;
    let job_path: OwnedObjectPath = manager
        .call_with_flags(
            "TryRestartUnit",
            MethodFlags::NoAutoStart.into(),
            &(BLUETOOTH_UNIT, SYSTEMD_JOB_MODE),
        )
        .await
        .map_err(|_| ManagerError::Rejected)?
        .ok_or(ManagerError::ProtocolViolation)?;
    let result = loop {
        let message = removed.next().await.ok_or(ManagerError::Disconnected)?;
        let (_, removed_path, unit, result): (u32, OwnedObjectPath, String, String) = message
            .body()
            .deserialize()
            .map_err(|_| ManagerError::ProtocolViolation)?;
        if removed_path == job_path && unit == BLUETOOTH_UNIT {
            break result;
        }
    };
    if !valid_job_result(&result) {
        return Err(ManagerError::ProtocolViolation);
    }
    let after = observe_on(&connection).await?;
    Ok(RestartCompletion {
        job_result: result,
        after,
    })
}

fn valid_job_result(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_JOB_RESULT_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn map_get_unit_error(error: zbus::Error) -> ManagerError {
    if error.to_string().contains("NoSuchUnit") {
        ManagerError::UnitUnavailable
    } else {
        ManagerError::ProtocolViolation
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader};
    use std::process::{Child, Command, Stdio};
    use std::sync::{Arc, Mutex};
    use zbus::object_server::SignalEmitter;

    const UNIT_PATH: &str = "/org/freedesktop/systemd1/unit/bluetooth_2eservice";
    const JOB_PATH: &str = "/org/freedesktop/systemd1/job/7";

    struct TestBus(Child);

    impl Drop for TestBus {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    #[derive(Default)]
    struct State {
        invocation: u8,
        get_unit_calls: usize,
        restart_calls: usize,
    }

    struct Manager(Arc<Mutex<State>>);

    #[derive(Debug, zbus::DBusError)]
    #[zbus(prefix = "org.freedesktop.systemd1")]
    enum MockError {
        Unexpected(String),
        #[zbus(error)]
        ZBus(zbus::Error),
    }

    #[zbus::interface(name = "org.freedesktop.systemd1.Manager")]
    impl Manager {
        #[zbus(name = "GetUnit")]
        fn get_unit(&self, name: &str) -> Result<OwnedObjectPath, MockError> {
            if name != BLUETOOTH_UNIT {
                return Err(MockError::Unexpected(name.into()));
            }
            self.0.lock().unwrap().get_unit_calls += 1;
            OwnedObjectPath::try_from(UNIT_PATH)
                .map_err(zbus::Error::from)
                .map_err(MockError::ZBus)
        }

        #[zbus(name = "TryRestartUnit")]
        async fn try_restart_unit(
            &self,
            unit: &str,
            mode: &str,
            #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
        ) -> Result<OwnedObjectPath, MockError> {
            if unit != BLUETOOTH_UNIT || mode != SYSTEMD_JOB_MODE {
                return Err(MockError::Unexpected(format!("{unit}:{mode}")));
            }
            {
                let mut state = self.0.lock().unwrap();
                state.restart_calls += 1;
                state.invocation = 2;
            }
            let job = OwnedObjectPath::try_from(JOB_PATH)
                .map_err(zbus::Error::from)
                .map_err(MockError::ZBus)?;
            Self::job_removed(&emitter, 7, job.clone(), unit, "done")
                .await
                .map_err(MockError::ZBus)?;
            Ok(job)
        }

        #[zbus(signal, name = "JobRemoved")]
        async fn job_removed(
            emitter: &SignalEmitter<'_>,
            id: u32,
            job: OwnedObjectPath,
            unit: &str,
            result: &str,
        ) -> zbus::Result<()>;
    }

    struct Unit(Arc<Mutex<State>>);

    #[zbus::interface(name = "org.freedesktop.systemd1.Unit")]
    impl Unit {
        #[zbus(property, name = "Id")]
        fn id(&self) -> &str {
            BLUETOOTH_UNIT
        }

        #[zbus(property, name = "LoadState")]
        fn load_state(&self) -> &str {
            "loaded"
        }

        #[zbus(property, name = "ActiveState")]
        fn active_state(&self) -> &str {
            "active"
        }

        #[zbus(property, name = "InvocationID")]
        fn invocation_id(&self) -> Vec<u8> {
            vec![self.0.lock().unwrap().invocation; 16]
        }
    }

    fn test_bus() -> (TestBus, String) {
        let mut child = Command::new("dbus-daemon")
            .args(["--session", "--nofork", "--print-address=1"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("dbus-daemon is available on the Linux CI image");
        let mut address = String::new();
        BufReader::new(child.stdout.take().unwrap())
            .read_line(&mut address)
            .unwrap();
        (TestBus(child), address.trim().into())
    }

    #[test]
    fn uses_only_fixed_get_try_restart_job_and_properties() {
        let (_bus, address) = test_bus();
        let state = Arc::new(Mutex::new(State {
            invocation: 1,
            ..State::default()
        }));
        let _service = zbus::blocking::connection::Builder::address(address.as_str())
            .unwrap()
            .name(SYSTEMD_DESTINATION)
            .unwrap()
            .serve_at(SYSTEMD_MANAGER_PATH, Manager(state.clone()))
            .unwrap()
            .serve_at(UNIT_PATH, Unit(state.clone()))
            .unwrap()
            .build()
            .unwrap();
        let mut adapter = SystemdBluetoothManager {
            address,
            timeout: Duration::from_secs(2),
        };
        let before = adapter.observe().unwrap();
        assert_eq!(before.invocation_id, [1; 16]);
        let completion = adapter.try_restart().unwrap();
        assert_eq!(completion.job_result, "done");
        assert_eq!(completion.after.invocation_id, [2; 16]);
        let state = state.lock().unwrap();
        assert_eq!(state.get_unit_calls, 2);
        assert_eq!(state.restart_calls, 1);
    }

    #[test]
    fn missing_bus_times_out_or_disconnects_without_subprocess_fallback() {
        let result = async_io::block_on(with_timeout(
            Duration::from_millis(100),
            observe_at("unix:path=/run/blossom-definitely-missing-system-bus"),
        ));
        assert!(matches!(
            result,
            Err(ManagerError::Disconnected | ManagerError::Timeout)
        ));
    }
}
