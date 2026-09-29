use blossom_core::ShellApprovalAuthentication;
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use zbus::proxy::{Builder as ProxyBuilder, CacheProperties, MethodFlags};
use zbus::zvariant::Value;
use zbus::{Proxy, connection};

pub const MODEL_EFFECT_POLKIT_ACTION: &str = "org.blossomos.shell.approve-model-effect";
const POLKIT_DESTINATION: &str = "org.freedesktop.PolicyKit1";
const POLKIT_PATH: &str = "/org/freedesktop/PolicyKit1/Authority";
const POLKIT_INTERFACE: &str = "org.freedesktop.PolicyKit1.Authority";
const SYSTEM_BUS_ADDRESS: &str = "unix:path=/run/dbus/system_bus_socket";
const ALLOW_USER_INTERACTION: u32 = 1;

type AuthorizationResponse = Option<(bool, bool, HashMap<String, String>)>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrustedApprovalResult {
    Authorized,
    Denied,
    Expired,
    Unavailable,
}

pub trait TrustedApprovalAuthorizer: Send {
    fn authorize(&mut self, challenge: &ShellApprovalAuthentication) -> TrustedApprovalResult;
}

pub struct DenyTrustedApproval;

impl TrustedApprovalAuthorizer for DenyTrustedApproval {
    fn authorize(&mut self, _: &ShellApprovalAuthentication) -> TrustedApprovalResult {
        TrustedApprovalResult::Unavailable
    }
}

pub struct PolkitTrustedApproval {
    address: String,
}

impl Default for PolkitTrustedApproval {
    fn default() -> Self {
        Self {
            address: SYSTEM_BUS_ADDRESS.into(),
        }
    }
}

impl TrustedApprovalAuthorizer for PolkitTrustedApproval {
    fn authorize(&mut self, challenge: &ShellApprovalAuthentication) -> TrustedApprovalResult {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(u64::MAX, |duration| {
                duration.as_millis().try_into().unwrap_or(u64::MAX)
            });
        let remaining_ms = challenge.expires_at_ms.saturating_sub(now_ms);
        if remaining_ms == 0 {
            return TrustedApprovalResult::Expired;
        }
        async_io::block_on(check_with_timeout(
            &self.address,
            Duration::from_millis(remaining_ms),
            challenge,
        ))
    }
}

async fn check_with_timeout(
    address: &str,
    timeout: Duration,
    challenge: &ShellApprovalAuthentication,
) -> TrustedApprovalResult {
    use futures_lite::future::race;

    race(check(address, challenge), async move {
        async_io::Timer::after(timeout).await;
        TrustedApprovalResult::Expired
    })
    .await
}

async fn check(address: &str, challenge: &ShellApprovalAuthentication) -> TrustedApprovalResult {
    let connection = match connection::Builder::address(address) {
        Ok(builder) => match builder
            .max_queued(8)
            .method_timeout(Duration::from_secs(30))
            .build()
            .await
        {
            Ok(connection) => connection,
            Err(_) => return TrustedApprovalResult::Unavailable,
        },
        Err(_) => return TrustedApprovalResult::Unavailable,
    };
    let Some(unique_name) = connection.unique_name() else {
        return TrustedApprovalResult::Unavailable;
    };
    let authority: Proxy<'_> = match ProxyBuilder::new(&connection)
        .destination(POLKIT_DESTINATION)
        .and_then(|builder| builder.path(POLKIT_PATH))
        .and_then(|builder| builder.interface(POLKIT_INTERFACE))
    {
        Ok(builder) => match builder.cache_properties(CacheProperties::No).build().await {
            Ok(proxy) => proxy,
            Err(_) => return TrustedApprovalResult::Unavailable,
        },
        Err(_) => return TrustedApprovalResult::Unavailable,
    };
    let subject_details = HashMap::from([("name", Value::from(unique_name.as_str()))]);
    let subject = ("system-bus-name", subject_details);
    let details = HashMap::from([
        ("blossom.request_id", challenge.request_id.clone()),
        ("blossom.preview_sha256", challenge.preview_sha256.clone()),
        ("blossom.effect", challenge.effect.into()),
        ("blossom.expires_at_ms", challenge.expires_at_ms.to_string()),
    ]);
    let response: Result<AuthorizationResponse, zbus::Error> = authority
        .call_with_flags(
            "CheckAuthorization",
            MethodFlags::NoAutoStart.into(),
            &(
                subject,
                MODEL_EFFECT_POLKIT_ACTION,
                details,
                ALLOW_USER_INTERACTION,
                "",
            ),
        )
        .await;
    match response {
        Ok(Some((true, _, _))) => TrustedApprovalResult::Authorized,
        Ok(Some((false, _, _))) => TrustedApprovalResult::Denied,
        _ => TrustedApprovalResult::Unavailable,
    }
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
        Delay,
    }

    #[derive(Default)]
    struct Seen {
        calls: usize,
        valid: bool,
        behavior: Behavior,
    }

    struct Authority(Arc<Mutex<Seen>>);

    #[derive(Debug, zbus::DBusError)]
    #[zbus(prefix = "org.freedesktop.PolicyKit1.Error")]
    enum AuthorityError {
        #[zbus(error)]
        ZBus(zbus::Error),
    }

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
        ) -> Result<(bool, bool, HashMap<String, String>), AuthorityError> {
            let sender = subject
                .1
                .get("name")
                .and_then(|value| value.try_clone().ok())
                .and_then(|value| String::try_from(value).ok());
            let (valid, behavior) = {
                let mut seen = self.0.lock().unwrap();
                let valid = subject.0 == "system-bus-name"
                    && sender.as_deref().is_some_and(|name| name.starts_with(':'))
                    && action == MODEL_EFFECT_POLKIT_ACTION
                    && details.get("blossom.request_id").map(String::as_str) == Some("request-1")
                    && details.get("blossom.preview_sha256").map(String::as_str)
                        == Some("preview-digest")
                    && details.get("blossom.effect").map(String::as_str)
                        == Some("files.write:create")
                    && details
                        .get("blossom.expires_at_ms")
                        .and_then(|value| value.parse::<u64>().ok())
                        .is_some_and(|value| value > now_ms())
                    && flags == ALLOW_USER_INTERACTION
                    && cancellation_id.is_empty();
                seen.calls += 1;
                seen.valid = valid;
                (valid, seen.behavior)
            };
            if matches!(behavior, Behavior::Delay) {
                async_io::Timer::after(Duration::from_millis(100)).await;
            }
            Ok(match behavior {
                Behavior::Allow | Behavior::Delay => (valid, false, HashMap::new()),
                Behavior::Deny => (false, false, HashMap::new()),
            })
        }
    }

    fn now_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis()
            .try_into()
            .unwrap()
    }

    fn challenge() -> ShellApprovalAuthentication {
        ShellApprovalAuthentication {
            request_id: "request-1".into(),
            preview_sha256: "preview-digest".into(),
            effect: "files.write:create",
            expires_at_ms: now_ms() + 5_000,
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

    fn authorize_with_behavior(
        behavior: Behavior,
        timeout: Duration,
    ) -> (TrustedApprovalResult, Arc<Mutex<Seen>>) {
        let (_bus, address) = test_bus();
        let seen = Arc::new(Mutex::new(Seen {
            behavior,
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
        let result = async_io::block_on(check_with_timeout(&address, timeout, &challenge()));
        (result, seen)
    }

    #[test]
    fn binds_authentication_to_exact_preview_over_dbus() {
        let (result, seen) = authorize_with_behavior(Behavior::Allow, Duration::from_secs(2));
        assert_eq!(result, TrustedApprovalResult::Authorized);
        let seen = seen.lock().unwrap();
        assert_eq!(seen.calls, 1);
        assert!(seen.valid);
    }

    #[test]
    fn denial_and_timeout_fail_closed() {
        assert_eq!(
            authorize_with_behavior(Behavior::Deny, Duration::from_secs(2)).0,
            TrustedApprovalResult::Denied
        );
        assert_eq!(
            authorize_with_behavior(Behavior::Delay, Duration::from_millis(10)).0,
            TrustedApprovalResult::Expired
        );
    }
}
