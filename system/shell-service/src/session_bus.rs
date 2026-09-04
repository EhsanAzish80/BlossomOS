use blossom_core::executor::bubblewrap::BubblewrapExecutor;
use blossom_core::{
    Executor, SHELL_BUS_NAME, SHELL_INTERFACE, SHELL_OBJECT_PATH, SHELL_PROTOCOL_VERSION,
    ShellClientRequest, ShellDiagnosticService, ShellPeerId, decode_shell_client_request,
};
use std::sync::{Arc, Mutex};
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
    fn disconnect(&mut self, peer: &ShellPeerId, now_ms: u64) -> Result<(), HandlerError>;
}

impl<E: Executor + Send> ShellRequestHandler for ShellDiagnosticService<E> {
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

    fn disconnect(&mut self, peer: &ShellPeerId, now_ms: u64) -> Result<(), HandlerError> {
        self.disconnect(peer, now_ms)
            .map(|_| ())
            .map_err(|_| HandlerError)
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
    handler: Arc<Mutex<Box<dyn ShellRequestHandler>>>,
}

impl ShellBusService {
    pub fn new(handler: impl ShellRequestHandler + 'static) -> Self {
        Self {
            handler: Arc::new(Mutex::new(Box::new(handler))),
        }
    }

    fn shared_handler(&self) -> Arc<Mutex<Box<dyn ShellRequestHandler>>> {
        Arc::clone(&self.handler)
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

fn lost_unique_owner(
    name: &str,
    old_owner: Option<&str>,
    new_owner: Option<&str>,
) -> Option<ShellPeerId> {
    if !name.starts_with(':') || old_owner != Some(name) || new_owner.is_some() {
        return None;
    }
    ShellPeerId::from_bus_unique_name(name).ok()
}

fn disconnect_match_rule() -> Result<zbus::MatchRule<'static>, ShellProcessError> {
    zbus::MatchRule::builder()
        .msg_type(zbus::message::Type::Signal)
        .sender(DBUS_DESTINATION)
        .and_then(|builder| builder.interface(DBUS_INTERFACE))
        .and_then(|builder| builder.member("NameOwnerChanged"))
        .map(|builder| builder.build().to_owned())
        .map_err(|_| ShellProcessError::SessionBusUnavailable)
}

fn monitor_disconnects(
    mut messages: zbus::blocking::MessageIterator,
    handler: Arc<Mutex<Box<dyn ShellRequestHandler>>>,
) -> Result<(), ShellProcessError> {
    for message in &mut messages {
        let message = message.map_err(|_| ShellProcessError::SessionBusUnavailable)?;
        let signal = zbus::fdo::NameOwnerChanged::from_message(message)
            .ok_or(ShellProcessError::SessionBusUnavailable)?;
        let args = signal
            .args()
            .map_err(|_| ShellProcessError::SessionBusUnavailable)?;
        let old_owner = args.old_owner().as_ref().map(|owner| owner.as_str());
        let new_owner = args.new_owner().as_ref().map(|owner| owner.as_str());
        if let Some(peer) = lost_unique_owner(args.name().as_str(), old_owner, new_owner) {
            handler
                .lock()
                .map_err(|_| ShellProcessError::SessionBusUnavailable)?
                .disconnect(&peer, now_ms())
                .map_err(|_| ShellProcessError::SessionBusUnavailable)?;
        }
    }
    Err(ShellProcessError::SessionBusUnavailable)
}

#[cfg(feature = "production-dbus-service")]
pub fn run_production() -> Result<(), ShellProcessError> {
    let mut nonce = [0_u8; 8];
    getrandom::fill(&mut nonce).map_err(|_| ShellProcessError::RandomnessUnavailable)?;
    let service = ShellDiagnosticService::new(
        BubblewrapExecutor::phase1_default(),
        u64::from_ne_bytes(nonce),
    );
    let connection = zbus::blocking::connection::Builder::session()
        .map_err(|_| ShellProcessError::SessionBusUnavailable)?
        .build()
        .map_err(|_| ShellProcessError::SessionBusUnavailable)?;
    let messages = zbus::blocking::MessageIterator::for_match_rule(
        disconnect_match_rule()?,
        &connection,
        Some(64),
    )
    .map_err(|_| ShellProcessError::SessionBusUnavailable)?;
    let interface = ShellBusService::new(service);
    let handler = interface.shared_handler();
    connection
        .object_server()
        .at(SHELL_OBJECT_PATH, interface)
        .map_err(|_| ShellProcessError::SessionBusUnavailable)?;
    connection
        .request_name(SHELL_BUS_NAME)
        .map_err(|_| ShellProcessError::SessionBusUnavailable)?;
    let (fatal_sender, fatal_receiver) = std::sync::mpsc::sync_channel(1);
    std::thread::Builder::new()
        .name("blossom-shell-owner-monitor".into())
        .spawn(move || {
            let _ = fatal_sender.send(monitor_disconnects(messages, handler));
        })
        .map_err(|_| ShellProcessError::SessionBusUnavailable)?;
    fatal_receiver
        .recv()
        .map_err(|_| ShellProcessError::SessionBusUnavailable)?
}

#[cfg(not(feature = "production-dbus-service"))]
pub fn run_production() -> Result<(), ShellProcessError> {
    Err(ShellProcessError::InactiveBuild)
}

#[cfg(test)]
mod tests {
    use super::*;
    use blossom_core::{CommandSpec, ExecutionResult, ExecutorError};
    use std::io::{BufRead, BufReader};
    use std::process::{Child, Command, Stdio};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{Duration, Instant};

    struct CountingExecutor {
        calls: Arc<AtomicUsize>,
    }

    impl Executor for CountingExecutor {
        fn execute(&mut self, command: &CommandSpec) -> Result<ExecutionResult, ExecutorError> {
            assert_eq!(command, &CommandSpec::system_uname());
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(ExecutionResult {
                exit_code: Some(0),
                stdout: b"Linux\n".to_vec(),
                stderr: Vec::new(),
                timed_out: false,
                output_truncated: false,
            })
        }
    }

    struct TestBus(Child);

    impl Drop for TestBus {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    struct Handler {
        calls: Arc<AtomicUsize>,
        disconnects: Arc<AtomicUsize>,
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
        fn disconnect(&mut self, peer: &ShellPeerId, _: u64) -> Result<(), HandlerError> {
            assert_eq!(peer.as_str(), self.expected_peer);
            self.disconnects.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
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
                    disconnects: Arc::new(AtomicUsize::new(0)),
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

    #[test]
    fn recognizes_only_complete_unique_name_loss() {
        assert!(lost_unique_owner(":1.42", Some(":1.42"), None).is_some());
        assert!(lost_unique_owner(":1.42", None, Some(":1.42")).is_none());
        assert!(lost_unique_owner(":1.42", Some(":1.42"), Some(":1.43")).is_none());
        assert!(lost_unique_owner("org.example.Client", Some(":1.42"), None).is_none());
        assert!(lost_unique_owner(":1.42", Some(":1.41"), None).is_none());
        assert!(lost_unique_owner(":malformed", Some(":malformed"), None).is_none());
    }

    #[test]
    fn dropping_client_notifies_the_shared_handler() {
        let (_bus, address) = test_bus();
        let service_connection = zbus::blocking::connection::Builder::address(address.as_str())
            .expect("service address")
            .build()
            .expect("service connection");
        let messages = zbus::blocking::MessageIterator::for_match_rule(
            disconnect_match_rule().expect("match rule"),
            &service_connection,
            Some(64),
        )
        .expect("disconnect subscription");
        let client = zbus::blocking::connection::Builder::address(address.as_str())
            .expect("client address")
            .build()
            .expect("client");
        let sender = client
            .inner()
            .unique_name()
            .expect("unique name")
            .to_string();
        let disconnects = Arc::new(AtomicUsize::new(0));
        let interface = ShellBusService::new(Handler {
            calls: Arc::new(AtomicUsize::new(0)),
            disconnects: Arc::clone(&disconnects),
            expected_peer: sender,
        });
        let handler = interface.shared_handler();
        let monitor = std::thread::spawn(move || monitor_disconnects(messages, handler));

        drop(client);
        let deadline = Instant::now() + Duration::from_secs(2);
        while disconnects.load(Ordering::SeqCst) == 0 && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(disconnects.load(Ordering::SeqCst), 1);
        drop(service_connection);
        drop(monitor);
    }

    fn decision_bytes(preview: &serde_json::Value, decision: &str) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({
            "kind": "submit_decision",
            "version": SHELL_PROTOCOL_VERSION,
            "request_id": preview["request_id"],
            "preview_sha256": preview["preview_sha256"],
            "decision": decision,
        }))
        .expect("decision")
    }

    #[test]
    fn hostile_peer_mutation_replay_and_cancellation_start_nothing() {
        let (_bus, address) = test_bus();
        let owner = zbus::blocking::connection::Builder::address(address.as_str())
            .expect("owner address")
            .build()
            .expect("owner");
        let attacker = zbus::blocking::connection::Builder::address(address.as_str())
            .expect("attacker address")
            .build()
            .expect("attacker");
        let calls = Arc::new(AtomicUsize::new(0));
        let _service = zbus::blocking::connection::Builder::address(address.as_str())
            .expect("service address")
            .name(SHELL_BUS_NAME)
            .expect("service name")
            .serve_at(
                SHELL_OBJECT_PATH,
                ShellBusService::new(ShellDiagnosticService::new(
                    CountingExecutor {
                        calls: Arc::clone(&calls),
                    },
                    17,
                )),
            )
            .expect("serve")
            .build()
            .expect("service");
        let owner_proxy =
            zbus::blocking::Proxy::new(&owner, SHELL_BUS_NAME, SHELL_OBJECT_PATH, SHELL_INTERFACE)
                .expect("owner proxy");
        let attacker_proxy = zbus::blocking::Proxy::new(
            &attacker,
            SHELL_BUS_NAME,
            SHELL_OBJECT_PATH,
            SHELL_INTERFACE,
        )
        .expect("attacker proxy");

        let awaiting: Vec<u8> = owner_proxy
            .call("StartSystemUname1", &(SHELL_PROTOCOL_VERSION,))
            .expect("start");
        let envelope: serde_json::Value = serde_json::from_slice(&awaiting).expect("envelope");
        let preview = &envelope["preview"];
        let approve = decision_bytes(preview, "approve_once");

        let stolen: Result<Vec<u8>, _> =
            attacker_proxy.call("SubmitDecision1", &(approve.clone(),));
        assert!(stolen.is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 0);

        let mut mutated: serde_json::Value = serde_json::from_slice(&approve).expect("request");
        mutated["preview_sha256"] = serde_json::Value::String("0".repeat(64));
        let mutated = serde_json::to_vec(&mutated).expect("mutated request");
        let changed: Result<Vec<u8>, _> = owner_proxy.call("SubmitDecision1", &(mutated,));
        assert!(changed.is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 0);

        let verified: Vec<u8> = owner_proxy
            .call("SubmitDecision1", &(approve.clone(),))
            .expect("approve once");
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&verified).expect("verified")["status"],
            "verified"
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        let replay: Result<Vec<u8>, _> = owner_proxy.call("SubmitDecision1", &(approve,));
        assert!(replay.is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        let awaiting: Vec<u8> = owner_proxy
            .call("StartSystemUname1", &(SHELL_PROTOCOL_VERSION,))
            .expect("second start");
        let envelope: serde_json::Value = serde_json::from_slice(&awaiting).expect("envelope");
        let preview = &envelope["preview"];
        let cancel = serde_json::to_vec(&serde_json::json!({
            "kind": "cancel_pending",
            "version": SHELL_PROTOCOL_VERSION,
            "request_id": preview["request_id"],
            "preview_sha256": preview["preview_sha256"],
        }))
        .expect("cancel");
        let cancelled: Vec<u8> = owner_proxy
            .call("CancelPending1", &(cancel.clone(),))
            .expect("cancel once");
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&cancelled).expect("cancelled")["status"],
            "cancelled"
        );
        let cancel_replay: Result<Vec<u8>, _> = owner_proxy.call("CancelPending1", &(cancel,));
        assert!(cancel_replay.is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn service_loss_and_restart_invalidate_the_old_preview() {
        let (_bus, address) = test_bus();
        let client = zbus::blocking::connection::Builder::address(address.as_str())
            .expect("client address")
            .build()
            .expect("client");
        let first_calls = Arc::new(AtomicUsize::new(0));
        let first_service = zbus::blocking::connection::Builder::address(address.as_str())
            .expect("first service address")
            .name(SHELL_BUS_NAME)
            .expect("first service name")
            .serve_at(
                SHELL_OBJECT_PATH,
                ShellBusService::new(ShellDiagnosticService::new(
                    CountingExecutor {
                        calls: Arc::clone(&first_calls),
                    },
                    31,
                )),
            )
            .expect("first serve")
            .build()
            .expect("first service");
        let proxy =
            zbus::blocking::Proxy::new(&client, SHELL_BUS_NAME, SHELL_OBJECT_PATH, SHELL_INTERFACE)
                .expect("proxy");
        let awaiting: Vec<u8> = proxy
            .call("StartSystemUname1", &(SHELL_PROTOCOL_VERSION,))
            .expect("start");
        let envelope: serde_json::Value = serde_json::from_slice(&awaiting).expect("envelope");
        let stale_approval = decision_bytes(&envelope["preview"], "approve_once");

        drop(first_service);
        let while_absent: Result<Vec<u8>, _> =
            proxy.call("SubmitDecision1", &(stale_approval.clone(),));
        assert!(while_absent.is_err());
        assert_eq!(first_calls.load(Ordering::SeqCst), 0);

        let second_calls = Arc::new(AtomicUsize::new(0));
        let _second_service = zbus::blocking::connection::Builder::address(address.as_str())
            .expect("second service address")
            .name(SHELL_BUS_NAME)
            .expect("second service name")
            .serve_at(
                SHELL_OBJECT_PATH,
                ShellBusService::new(ShellDiagnosticService::new(
                    CountingExecutor {
                        calls: Arc::clone(&second_calls),
                    },
                    32,
                )),
            )
            .expect("second serve")
            .build()
            .expect("second service");
        let after_restart: Result<Vec<u8>, _> = proxy.call("SubmitDecision1", &(stale_approval,));
        assert!(after_restart.is_err());
        assert_eq!(first_calls.load(Ordering::SeqCst), 0);
        assert_eq!(second_calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn session_bus_loss_starts_nothing() {
        let (bus, address) = test_bus();
        let client = zbus::blocking::connection::Builder::address(address.as_str())
            .expect("client address")
            .build()
            .expect("client");
        let calls = Arc::new(AtomicUsize::new(0));
        let _service = zbus::blocking::connection::Builder::address(address.as_str())
            .expect("service address")
            .name(SHELL_BUS_NAME)
            .expect("service name")
            .serve_at(
                SHELL_OBJECT_PATH,
                ShellBusService::new(ShellDiagnosticService::new(
                    CountingExecutor {
                        calls: Arc::clone(&calls),
                    },
                    41,
                )),
            )
            .expect("serve")
            .build()
            .expect("service");
        let proxy =
            zbus::blocking::Proxy::new(&client, SHELL_BUS_NAME, SHELL_OBJECT_PATH, SHELL_INTERFACE)
                .expect("proxy");
        let awaiting: Vec<u8> = proxy
            .call("StartSystemUname1", &(SHELL_PROTOCOL_VERSION,))
            .expect("start");
        let envelope: serde_json::Value = serde_json::from_slice(&awaiting).expect("envelope");
        let approval = decision_bytes(&envelope["preview"], "approve_once");

        drop(bus);
        let after_bus_loss: Result<Vec<u8>, _> = proxy.call("SubmitDecision1", &(approval,));
        assert!(after_bus_loss.is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }
}
