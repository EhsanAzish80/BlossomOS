use crate::{
    AuthenticatedCaller, Authorizer, BluetoothManager, HelperAudit, IdempotencyJournal,
    PrivilegedHelper,
};
use blossom_core::privileged::{
    BluetoothRestartRequest, BluetoothRestartResult, PRIVILEGED_INTERFACE,
};
use std::sync::Mutex;
use zbus::message::Header;
use zbus::proxy::{Builder as ProxyBuilder, CacheProperties, MethodFlags};
use zbus::{Connection, Proxy};

const DBUS_DESTINATION: &str = "org.freedesktop.DBus";
const DBUS_PATH: &str = "/org/freedesktop/DBus";
const DBUS_INTERFACE: &str = "org.freedesktop.DBus";
const MAX_WIRE_RESULT_BYTES: usize = 4096;

pub trait PrivilegedRequestHandler: Send {
    fn handle(
        &mut self,
        caller: AuthenticatedCaller,
        request: BluetoothRestartRequest,
    ) -> BluetoothRestartResult;
}

impl<A, M, J, L> PrivilegedRequestHandler for PrivilegedHelper<A, M, J, L>
where
    A: Authorizer + Send,
    M: BluetoothManager + Send,
    J: IdempotencyJournal + Send,
    L: HelperAudit + Send,
{
    fn handle(
        &mut self,
        caller: AuthenticatedCaller,
        request: BluetoothRestartRequest,
    ) -> BluetoothRestartResult {
        PrivilegedHelper::handle(self, caller, request)
    }
}

/// The complete exported D-Bus surface: exactly one closed method.
pub struct PrivilegedService {
    handler: Mutex<Box<dyn PrivilegedRequestHandler>>,
}

impl PrivilegedService {
    pub fn new(handler: impl PrivilegedRequestHandler + 'static) -> Self {
        Self {
            handler: Mutex::new(Box::new(handler)),
        }
    }
}

#[zbus::interface(name = "org.blossomos.Privileged1")]
impl PrivilegedService {
    #[zbus(name = "TryRestartBluetooth1")]
    async fn try_restart_bluetooth1(
        &self,
        version: u16,
        correlation_id: String,
        idempotency_key: String,
        interactive: bool,
        #[zbus(header)] header: Header<'_>,
        #[zbus(connection)] connection: &Connection,
    ) -> zbus::fdo::Result<Vec<u8>> {
        if header.interface().map(|name| name.as_str()) != Some(PRIVILEGED_INTERFACE) {
            return Err(zbus::fdo::Error::AccessDenied(
                "invalid privileged interface".into(),
            ));
        }
        let sender = header
            .sender()
            .ok_or_else(|| zbus::fdo::Error::AccessDenied("missing unique sender".into()))?
            .to_string();
        let uid = resolve_uid(connection, &sender).await?;
        if uid == 0 {
            return Err(zbus::fdo::Error::AccessDenied(
                "root callers are not accepted".into(),
            ));
        }
        let caller = AuthenticatedCaller {
            uid,
            system_bus_name: sender,
        };
        caller
            .validate()
            .map_err(|_| zbus::fdo::Error::AccessDenied("invalid unique sender".into()))?;
        let request = BluetoothRestartRequest {
            version,
            correlation_id,
            idempotency_key,
            interactive,
        };
        let result = self
            .handler
            .lock()
            .map_err(|_| zbus::fdo::Error::Failed("helper state unavailable".into()))?
            .handle(caller, request);
        let bytes = serde_json::to_vec(&result)
            .map_err(|_| zbus::fdo::Error::Failed("result encoding failed".into()))?;
        if bytes.len() > MAX_WIRE_RESULT_BYTES {
            return Err(zbus::fdo::Error::Failed("result bound exceeded".into()));
        }
        Ok(bytes)
    }
}

async fn resolve_uid(connection: &Connection, sender: &str) -> zbus::fdo::Result<u32> {
    let bus: Proxy<'_> = ProxyBuilder::new(connection)
        .destination(DBUS_DESTINATION)
        .and_then(|builder| builder.path(DBUS_PATH))
        .and_then(|builder| builder.interface(DBUS_INTERFACE))
        .map_err(|_| zbus::fdo::Error::Failed("system bus proxy unavailable".into()))?
        .cache_properties(CacheProperties::No)
        .build()
        .await
        .map_err(|_| zbus::fdo::Error::Failed("system bus proxy unavailable".into()))?;
    bus.call_with_flags(
        "GetConnectionUnixUser",
        MethodFlags::NoAutoStart.into(),
        &(sender,),
    )
    .await
    .map_err(|_| zbus::fdo::Error::AccessDenied("sender credentials unavailable".into()))?
    .ok_or_else(|| zbus::fdo::Error::AccessDenied("sender credentials unavailable".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use blossom_core::privileged::{
        BluetoothRestartFailure, BluetoothRestartOutcome, PRIVILEGED_BUS_NAME,
        PRIVILEGED_OBJECT_PATH, PRIVILEGED_PROTOCOL_VERSION,
    };
    use std::io::{BufRead, BufReader};
    use std::process::{Child, Command, Stdio};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct TestBus(Child);

    impl Drop for TestBus {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    struct Handler {
        calls: Arc<AtomicUsize>,
        expected_sender: String,
    }

    impl PrivilegedRequestHandler for Handler {
        fn handle(
            &mut self,
            caller: AuthenticatedCaller,
            request: BluetoothRestartRequest,
        ) -> BluetoothRestartResult {
            assert_eq!(caller.system_bus_name, self.expected_sender);
            assert_ne!(caller.uid, 0);
            request.validate().unwrap();
            self.calls.fetch_add(1, Ordering::SeqCst);
            BluetoothRestartResult {
                version: PRIVILEGED_PROTOCOL_VERSION,
                correlation_id: request.correlation_id.clone(),
                authenticated_uid: caller.uid,
                request_sha256: request.normalized_digest(caller.uid).unwrap(),
                replayed: false,
                outcome: BluetoothRestartOutcome::Failed {
                    error: BluetoothRestartFailure::Denied,
                    job_submitted: false,
                },
            }
        }
    }

    fn test_bus() -> (TestBus, String) {
        let mut child = Command::new("dbus-daemon")
            .args(["--session", "--nofork", "--print-address=1"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let mut address = String::new();
        BufReader::new(child.stdout.take().unwrap())
            .read_line(&mut address)
            .unwrap();
        (TestBus(child), address.trim().into())
    }

    #[test]
    fn captures_real_sender_uid_and_exports_only_the_closed_method() {
        let (_bus, address) = test_bus();
        let client = zbus::blocking::connection::Builder::address(address.as_str())
            .unwrap()
            .build()
            .unwrap();
        let sender = client.inner().unique_name().unwrap().to_string();
        let calls = Arc::new(AtomicUsize::new(0));
        let _service = zbus::blocking::connection::Builder::address(address.as_str())
            .unwrap()
            .name(PRIVILEGED_BUS_NAME)
            .unwrap()
            .serve_at(
                PRIVILEGED_OBJECT_PATH,
                PrivilegedService::new(Handler {
                    calls: calls.clone(),
                    expected_sender: sender,
                }),
            )
            .unwrap()
            .build()
            .unwrap();
        let proxy = zbus::blocking::Proxy::new(
            &client,
            PRIVILEGED_BUS_NAME,
            PRIVILEGED_OBJECT_PATH,
            PRIVILEGED_INTERFACE,
        )
        .unwrap();
        let bytes: Vec<u8> = proxy
            .call(
                "TryRestartBluetooth1",
                &(1u16, "request-1", "0".repeat(32), true),
            )
            .unwrap();
        let result: BluetoothRestartResult = serde_json::from_slice(&bytes).unwrap();
        assert!(matches!(
            result.outcome,
            BluetoothRestartOutcome::Failed {
                error: BluetoothRestartFailure::Denied,
                job_submitted: false
            }
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        let unknown: Result<(), _> = proxy.call("Execute", &("/bin/sh",));
        assert!(unknown.is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}
