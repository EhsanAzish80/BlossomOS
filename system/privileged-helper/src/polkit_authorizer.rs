use crate::{AuthenticatedCaller, AuthorizationDecision, Authorizer};
use blossom_core::privileged::{
    BLUETOOTH_METHOD, BLUETOOTH_POLKIT_ACTION, BLUETOOTH_UNIT, BluetoothRestartRequest,
};
use std::collections::HashMap;
use std::time::Duration;
use zbus::proxy::{Builder as ProxyBuilder, CacheProperties, MethodFlags};
use zbus::zvariant::Value;
use zbus::{Proxy, connection};

const POLKIT_DESTINATION: &str = "org.freedesktop.PolicyKit1";
const POLKIT_PATH: &str = "/org/freedesktop/PolicyKit1/Authority";
const POLKIT_INTERFACE: &str = "org.freedesktop.PolicyKit1.Authority";
const DBUS_DESTINATION: &str = "org.freedesktop.DBus";
const DBUS_PATH: &str = "/org/freedesktop/DBus";
const DBUS_INTERFACE: &str = "org.freedesktop.DBus";
const SYSTEM_BUS_ADDRESS: &str = "unix:path=/run/dbus/system_bus_socket";
const AUTHORIZATION_TIMEOUT: Duration = Duration::from_secs(120);
const ALLOW_USER_INTERACTION: u32 = 1;

type AuthorizationResponse = Option<(bool, bool, HashMap<String, String>)>;

/// Independent polkit authorization for the fixed Bluetooth action.
///
/// The subject is always the authenticated system-bus sender. The request
/// cannot select a subject, action, unit, operation, flag, or bus destination.
#[derive(Debug)]
pub struct PolkitAuthorizer {
    address: String,
    timeout: Duration,
}

impl Default for PolkitAuthorizer {
    fn default() -> Self {
        Self {
            address: SYSTEM_BUS_ADDRESS.into(),
            timeout: AUTHORIZATION_TIMEOUT,
        }
    }
}

impl Authorizer for PolkitAuthorizer {
    fn authorize(
        &mut self,
        caller: &AuthenticatedCaller,
        request: &BluetoothRestartRequest,
    ) -> AuthorizationDecision {
        if caller.validate().is_err() || request.validate().is_err() || !request.interactive {
            return AuthorizationDecision::Denied;
        }
        async_io::block_on(check_with_timeout(
            &self.address,
            self.timeout,
            caller,
            request,
        ))
    }
}

async fn check_with_timeout(
    address: &str,
    timeout: Duration,
    caller: &AuthenticatedCaller,
    request: &BluetoothRestartRequest,
) -> AuthorizationDecision {
    use futures_lite::future::race;

    race(check(address, caller, request), async move {
        async_io::Timer::after(timeout).await;
        AuthorizationDecision::Expired
    })
    .await
}

async fn check(
    address: &str,
    caller: &AuthenticatedCaller,
    request: &BluetoothRestartRequest,
) -> AuthorizationDecision {
    let connection = match connection::Builder::address(address) {
        Ok(builder) => match builder
            .max_queued(8)
            .method_timeout(AUTHORIZATION_TIMEOUT)
            .build()
            .await
        {
            Ok(connection) => connection,
            Err(_) => return AuthorizationDecision::Unavailable,
        },
        Err(_) => return AuthorizationDecision::Unavailable,
    };
    let authority: Proxy<'_> = match ProxyBuilder::new(&connection)
        .destination(POLKIT_DESTINATION)
        .and_then(|builder| builder.path(POLKIT_PATH))
        .and_then(|builder| builder.interface(POLKIT_INTERFACE))
    {
        Ok(builder) => match builder.cache_properties(CacheProperties::No).build().await {
            Ok(proxy) => proxy,
            Err(_) => return AuthorizationDecision::Unavailable,
        },
        Err(_) => return AuthorizationDecision::Unavailable,
    };
    let subject_details = HashMap::from([("name", Value::from(caller.system_bus_name.as_str()))]);
    let subject = ("system-bus-name", subject_details);
    let details = HashMap::from([
        ("blossom.operation", BLUETOOTH_METHOD),
        ("blossom.unit", BLUETOOTH_UNIT),
        ("blossom.correlation", request.correlation_id.as_str()),
    ]);
    let response: Result<AuthorizationResponse, zbus::Error> = authority
        .call_with_flags(
            "CheckAuthorization",
            MethodFlags::NoAutoStart.into(),
            &(
                subject,
                BLUETOOTH_POLKIT_ACTION,
                details,
                ALLOW_USER_INTERACTION,
                "",
            ),
        )
        .await;
    match response {
        Ok(Some((true, _, _))) if sender_still_owned(&connection, caller).await => {
            AuthorizationDecision::Authorized
        }
        Ok(Some((false, _, _))) => AuthorizationDecision::Denied,
        _ => AuthorizationDecision::Unavailable,
    }
}

async fn sender_still_owned(connection: &zbus::Connection, caller: &AuthenticatedCaller) -> bool {
    let bus: Proxy<'_> = match ProxyBuilder::new(connection)
        .destination(DBUS_DESTINATION)
        .and_then(|builder| builder.path(DBUS_PATH))
        .and_then(|builder| builder.interface(DBUS_INTERFACE))
    {
        Ok(builder) => match builder.cache_properties(CacheProperties::No).build().await {
            Ok(proxy) => proxy,
            Err(_) => return false,
        },
        Err(_) => return false,
    };
    matches!(
        bus.call_with_flags::<_, _, bool>(
            "NameHasOwner",
            MethodFlags::NoAutoStart.into(),
            &(caller.system_bus_name.as_str(),),
        )
        .await,
        Ok(Some(true))
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader};
    use std::process::{Child, Command, Stdio};
    use std::sync::{Arc, Mutex};
    use zbus::zvariant::OwnedValue;

    struct TestBus(Child);

    impl Drop for TestBus {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    #[derive(Clone, Copy, Default)]
    enum Behavior {
        #[default]
        Allow,
        Deny,
        Challenge,
        Delay,
    }

    #[derive(Default)]
    struct Seen {
        valid: bool,
        calls: usize,
        expected_name: String,
        behavior: Behavior,
    }

    struct Authority(Arc<Mutex<Seen>>);

    #[zbus::interface(name = "org.freedesktop.PolicyKit1.Authority")]
    impl Authority {
        #[zbus(name = "CheckAuthorization")]
        async fn check_authorization(
            &self,
            subject: (String, HashMap<String, OwnedValue>),
            action: String,
            details: HashMap<String, String>,
            flags: u32,
            cancellation_id: String,
        ) -> (bool, bool, HashMap<String, String>) {
            let name = subject
                .1
                .get("name")
                .and_then(|value| value.try_clone().ok())
                .and_then(|value| String::try_from(value).ok());
            let (valid, behavior) = {
                let mut seen = self.0.lock().unwrap();
                let valid = subject.0 == "system-bus-name"
                    && name.as_deref() == Some(seen.expected_name.as_str())
                    && action == BLUETOOTH_POLKIT_ACTION
                    && details.get("blossom.operation").map(String::as_str)
                        == Some(BLUETOOTH_METHOD)
                    && details.get("blossom.unit").map(String::as_str) == Some(BLUETOOTH_UNIT)
                    && details.get("blossom.correlation").map(String::as_str) == Some("request-1")
                    && flags == ALLOW_USER_INTERACTION
                    && cancellation_id.is_empty();
                seen.calls += 1;
                seen.valid = valid;
                (valid, seen.behavior)
            };
            if matches!(behavior, Behavior::Delay) {
                async_io::Timer::after(Duration::from_millis(100)).await;
            }
            match behavior {
                Behavior::Allow | Behavior::Delay => (valid, false, HashMap::new()),
                Behavior::Deny => (false, false, HashMap::new()),
                Behavior::Challenge => (false, true, HashMap::new()),
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

    fn caller(name: &str) -> AuthenticatedCaller {
        AuthenticatedCaller {
            uid: 1000,
            system_bus_name: name.into(),
        }
    }

    fn request(interactive: bool) -> BluetoothRestartRequest {
        BluetoothRestartRequest {
            version: 1,
            correlation_id: "request-1".into(),
            idempotency_key: "0".repeat(32),
            interactive,
        }
    }

    #[test]
    fn binds_the_fixed_action_and_details_to_the_system_bus_subject() {
        let (_bus, address) = test_bus();
        let client = zbus::blocking::connection::Builder::address(address.as_str())
            .unwrap()
            .build()
            .unwrap();
        let sender = client.inner().unique_name().unwrap().to_string();
        let seen = Arc::new(Mutex::new(Seen {
            expected_name: sender.clone(),
            ..Seen::default()
        }));
        let _service = zbus::blocking::connection::Builder::address(address.as_str())
            .unwrap()
            .name(POLKIT_DESTINATION)
            .unwrap()
            .serve_at(POLKIT_PATH, Authority(seen.clone()))
            .unwrap()
            .build()
            .unwrap();
        let mut authorizer = PolkitAuthorizer {
            address,
            timeout: Duration::from_secs(2),
        };
        assert_eq!(
            authorizer.authorize(&caller(&sender), &request(true)),
            AuthorizationDecision::Authorized
        );
        let seen = seen.lock().unwrap();
        assert_eq!(seen.calls, 1);
        assert!(seen.valid);
        drop(seen);
        drop(client);
        assert_eq!(
            authorizer.authorize(&caller(&sender), &request(true)),
            AuthorizationDecision::Unavailable
        );
    }

    fn authorize_with_behavior(behavior: Behavior, timeout: Duration) -> AuthorizationDecision {
        let (_bus, address) = test_bus();
        let client = zbus::blocking::connection::Builder::address(address.as_str())
            .unwrap()
            .build()
            .unwrap();
        let sender = client.inner().unique_name().unwrap().to_string();
        let seen = Arc::new(Mutex::new(Seen {
            expected_name: sender.clone(),
            behavior,
            ..Seen::default()
        }));
        let _service = zbus::blocking::connection::Builder::address(address.as_str())
            .unwrap()
            .name(POLKIT_DESTINATION)
            .unwrap()
            .serve_at(POLKIT_PATH, Authority(seen))
            .unwrap()
            .build()
            .unwrap();
        PolkitAuthorizer { address, timeout }.authorize(&caller(&sender), &request(true))
    }

    #[test]
    fn denial_and_challenge_never_authorize() {
        assert_eq!(
            authorize_with_behavior(Behavior::Deny, Duration::from_secs(2)),
            AuthorizationDecision::Denied
        );
        assert_eq!(
            authorize_with_behavior(Behavior::Challenge, Duration::from_secs(2)),
            AuthorizationDecision::Denied
        );
    }

    #[test]
    fn an_unfinished_authorization_expires_closed() {
        assert_eq!(
            authorize_with_behavior(Behavior::Delay, Duration::from_millis(10)),
            AuthorizationDecision::Expired
        );
    }

    #[test]
    fn noninteractive_or_invalid_subject_is_denied_without_bus_contact() {
        let mut authorizer = PolkitAuthorizer {
            address: "unix:path=/run/blossom-missing-polkit-bus".into(),
            timeout: Duration::from_millis(20),
        };
        assert_eq!(
            authorizer.authorize(&caller(":1.42"), &request(false)),
            AuthorizationDecision::Denied
        );
        let mut invalid = caller(":1.42");
        invalid.system_bus_name = "caller-selected-name".into();
        assert_eq!(
            authorizer.authorize(&invalid, &request(true)),
            AuthorizationDecision::Denied
        );
    }
}
