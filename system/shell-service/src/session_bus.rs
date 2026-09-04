use blossom_core::executor::bubblewrap::BubblewrapExecutor;
use blossom_core::{
    SHELL_BUS_NAME, SHELL_INTERFACE, SHELL_OBJECT_PATH, SHELL_PROTOCOL_VERSION, ShellClientRequest,
    ShellDiagnosticService, ShellPeerId, decode_shell_client_request,
};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use zbus::message::Header;
use zbus::proxy::{Builder as ProxyBuilder, CacheProperties, MethodFlags};
use zbus::{Connection, Proxy};

use crate::ShellProcessError;

const DBUS_DESTINATION: &str = "org.freedesktop.DBus";
const DBUS_PATH: &str = "/org/freedesktop/DBus";
const DBUS_INTERFACE: &str = "org.freedesktop.DBus";
const MAX_WIRE_RESULT_BYTES: usize = 32 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HandlerError;

pub trait ShellRequestHandler: Send {
    fn start(&mut self, peer: ShellPeerId, now_ms: u64) -> Result<Vec<u8>, HandlerError>;
    fn decide(
        &mut self,
        peer: &ShellPeerId,
        input: &[u8],
        now_ms: u64,
    ) -> Result<Vec<u8>, HandlerError>;
    fn cancel(
        &mut self,
        peer: &ShellPeerId,
        input: &[u8],
        now_ms: u64,
    ) -> Result<Vec<u8>, HandlerError>;
    fn activity(&mut self, after: Option<u64>, limit: u16) -> Result<Vec<u8>, HandlerError>;
    fn disconnect(&mut self, peer: &ShellPeerId, now_ms: u64);
}

impl ShellRequestHandler for ShellDiagnosticService<BubblewrapExecutor> {
    fn start(&mut self, peer: ShellPeerId, now_ms: u64) -> Result<Vec<u8>, HandlerError> {
        encode(
            &self
                .begin_system_uname(peer, now_ms)
                .map_err(|_| HandlerError)?,
        )
    }

    fn decide(
        &mut self,
        peer: &ShellPeerId,
        input: &[u8],
        now_ms: u64,
    ) -> Result<Vec<u8>, HandlerError> {
        let request = decode_shell_client_request(input).map_err(|_| HandlerError)?;
        if !matches!(request, ShellClientRequest::SubmitDecision { .. }) {
            return Err(HandlerError);
        }
        encode(
            &self
                .handle_client_request(peer, request, now_ms)
                .map_err(|_| HandlerError)?,
        )
    }

    fn cancel(
        &mut self,
        peer: &ShellPeerId,
        input: &[u8],
        now_ms: u64,
    ) -> Result<Vec<u8>, HandlerError> {
        let request = decode_shell_client_request(input).map_err(|_| HandlerError)?;
        if !matches!(request, ShellClientRequest::CancelPending { .. }) {
            return Err(HandlerError);
        }
        encode(
            &self
                .handle_client_request(peer, request, now_ms)
                .map_err(|_| HandlerError)?,
        )
    }

    fn activity(&mut self, after: Option<u64>, limit: u16) -> Result<Vec<u8>, HandlerError> {
        encode(&self.read_activity(after, limit).map_err(|_| HandlerError)?)
    }

    fn disconnect(&mut self, peer: &ShellPeerId, now_ms: u64) {
        let _ = self.disconnect(peer, now_ms);
    }
}

fn encode(value: &impl serde::Serialize) -> Result<Vec<u8>, HandlerError> {
    let bytes = serde_json::to_vec(value).map_err(|_| HandlerError)?;
    if bytes.len() > MAX_WIRE_RESULT_BYTES {
        return Err(HandlerError);
    }
    Ok(bytes)
}

pub struct ShellBusService {
    handler: Mutex<Box<dyn ShellRequestHandler>>,
}

impl ShellBusService {
    pub fn new(handler: impl ShellRequestHandler + 'static) -> Self {
        Self {
            handler: Mutex::new(Box::new(handler)),
        }
    }
}

#[zbus::interface(name = "org.blossomos.Shell1")]
impl ShellBusService {
    #[zbus(name = "StartSystemUname1")]
    async fn start_system_uname1(
        &self,
        version: u16,
        #[zbus(header)] header: Header<'_>,
        #[zbus(connection)] connection: &Connection,
    ) -> zbus::fdo::Result<Vec<u8>> {
        check_version(version)?;
        let peer = authenticated_peer(&header, connection).await?;
        self.handler
            .lock()
            .map_err(|_| failed())?
            .start(peer, now_ms())
            .map_err(|_| denied())
    }

    #[zbus(name = "SubmitDecision1")]
    async fn submit_decision1(
        &self,
        input: Vec<u8>,
        #[zbus(header)] header: Header<'_>,
        #[zbus(connection)] connection: &Connection,
    ) -> zbus::fdo::Result<Vec<u8>> {
        let peer = authenticated_peer(&header, connection).await?;
        self.handler
            .lock()
            .map_err(|_| failed())?
            .decide(&peer, &input, now_ms())
            .map_err(|_| denied())
    }

    #[zbus(name = "CancelPending1")]
    async fn cancel_pending1(
        &self,
        input: Vec<u8>,
        #[zbus(header)] header: Header<'_>,
        #[zbus(connection)] connection: &Connection,
    ) -> zbus::fdo::Result<Vec<u8>> {
        let peer = authenticated_peer(&header, connection).await?;
        self.handler
            .lock()
            .map_err(|_| failed())?
            .cancel(&peer, &input, now_ms())
            .map_err(|_| denied())
    }

    #[zbus(name = "ReadActivity1")]
    async fn read_activity1(
        &self,
        version: u16,
        has_cursor: bool,
        cursor: u64,
        limit: u16,
        #[zbus(header)] header: Header<'_>,
        #[zbus(connection)] connection: &Connection,
    ) -> zbus::fdo::Result<Vec<u8>> {
        check_version(version)?;
        let _peer = authenticated_peer(&header, connection).await?;
        self.handler
            .lock()
            .map_err(|_| failed())?
            .activity(has_cursor.then_some(cursor), limit)
            .map_err(|_| denied())
    }
}

fn check_version(version: u16) -> zbus::fdo::Result<()> {
    if version == SHELL_PROTOCOL_VERSION {
        Ok(())
    } else {
        Err(denied())
    }
}

async fn authenticated_peer(
    header: &Header<'_>,
    connection: &Connection,
) -> zbus::fdo::Result<ShellPeerId> {
    if header.interface().map(|name| name.as_str()) != Some(SHELL_INTERFACE) {
        return Err(denied());
    }
    let sender = header.sender().ok_or_else(denied)?.to_string();
    let peer = ShellPeerId::from_bus_unique_name(&sender).map_err(|_| denied())?;
    let uid = resolve_uid(connection, &sender).await?;
    if uid == 0 || uid != nix::unistd::geteuid().as_raw() {
        return Err(denied());
    }
    Ok(peer)
}

async fn resolve_uid(connection: &Connection, sender: &str) -> zbus::fdo::Result<u32> {
    let bus: Proxy<'_> = ProxyBuilder::new(connection)
        .destination(DBUS_DESTINATION)
        .and_then(|b| b.path(DBUS_PATH))
        .and_then(|b| b.interface(DBUS_INTERFACE))
        .map_err(|_| failed())?
        .cache_properties(CacheProperties::No)
        .build()
        .await
        .map_err(|_| failed())?;
    bus.call_with_flags(
        "GetConnectionUnixUser",
        MethodFlags::NoAutoStart.into(),
        &(sender,),
    )
    .await
    .map_err(|_| denied())?
    .ok_or_else(denied)
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| {
            duration.as_millis().try_into().unwrap_or(u64::MAX)
        })
}

fn denied() -> zbus::fdo::Error {
    zbus::fdo::Error::AccessDenied("shell request rejected".into())
}
fn failed() -> zbus::fdo::Error {
    zbus::fdo::Error::Failed("shell service unavailable".into())
}

#[cfg(feature = "production-dbus-service")]
pub fn run_production() -> Result<(), ShellProcessError> {
    let mut nonce = [0_u8; 8];
    getrandom::fill(&mut nonce).map_err(|_| ShellProcessError::RandomnessUnavailable)?;
    let service = ShellDiagnosticService::new(
        BubblewrapExecutor::phase1_default(),
        u64::from_ne_bytes(nonce),
    );
    let _connection = zbus::blocking::connection::Builder::session()
        .map_err(|_| ShellProcessError::SessionBusUnavailable)?
        .name(SHELL_BUS_NAME)
        .map_err(|_| ShellProcessError::SessionBusUnavailable)?
        .serve_at(SHELL_OBJECT_PATH, ShellBusService::new(service))
        .map_err(|_| ShellProcessError::SessionBusUnavailable)?
        .build()
        .map_err(|_| ShellProcessError::SessionBusUnavailable)?;
    loop {
        std::thread::park();
    }
}

#[cfg(not(feature = "production-dbus-service"))]
pub fn run_production() -> Result<(), ShellProcessError> {
    Err(ShellProcessError::InactiveBuild)
}

#[cfg(test)]
mod tests {
    use super::*;
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
        expected_peer: String,
    }

    impl ShellRequestHandler for Handler {
        fn start(&mut self, peer: ShellPeerId, _: u64) -> Result<Vec<u8>, HandlerError> {
            assert_eq!(peer.as_str(), self.expected_peer);
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(br#"{"status":"awaiting_approval"}"#.to_vec())
        }
        fn decide(&mut self, _: &ShellPeerId, _: &[u8], _: u64) -> Result<Vec<u8>, HandlerError> {
            Err(HandlerError)
        }
        fn cancel(&mut self, _: &ShellPeerId, _: &[u8], _: u64) -> Result<Vec<u8>, HandlerError> {
            Err(HandlerError)
        }
        fn activity(&mut self, _: Option<u64>, _: u16) -> Result<Vec<u8>, HandlerError> {
            Err(HandlerError)
        }
        fn disconnect(&mut self, _: &ShellPeerId, _: u64) {}
    }

    fn test_bus() -> (TestBus, String) {
        let mut child = Command::new("dbus-daemon")
            .args(["--session", "--nofork", "--print-address=1"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("private dbus-daemon");
        let mut address = String::new();
        BufReader::new(child.stdout.take().expect("stdout"))
            .read_line(&mut address)
            .expect("bus address");
        (TestBus(child), address.trim().into())
    }

    #[test]
    fn derives_real_peer_and_exports_only_closed_versioned_surface() {
        let (_bus, address) = test_bus();
        let client = zbus::blocking::connection::Builder::address(address.as_str())
            .expect("client address")
            .build()
            .expect("client");
        let sender = client
            .inner()
            .unique_name()
            .expect("unique name")
            .to_string();
        let calls = Arc::new(AtomicUsize::new(0));
        let _service = zbus::blocking::connection::Builder::address(address.as_str())
            .expect("service address")
            .name(SHELL_BUS_NAME)
            .expect("service name")
            .serve_at(
                SHELL_OBJECT_PATH,
                ShellBusService::new(Handler {
                    calls: calls.clone(),
                    expected_peer: sender,
                }),
            )
            .expect("serve")
            .build()
            .expect("service");
        let proxy =
            zbus::blocking::Proxy::new(&client, SHELL_BUS_NAME, SHELL_OBJECT_PATH, SHELL_INTERFACE)
                .expect("proxy");
        let bytes: Vec<u8> = proxy
            .call("StartSystemUname1", &(SHELL_PROTOCOL_VERSION,))
            .expect("fixed call");
        assert_eq!(bytes, br#"{"status":"awaiting_approval"}"#);
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        let wrong_version: Result<Vec<u8>, _> = proxy.call("StartSystemUname1", &(2_u16,));
        assert!(wrong_version.is_err());
        let unknown: Result<(), _> = proxy.call("Execute", &("/bin/sh",));
        assert!(unknown.is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}
