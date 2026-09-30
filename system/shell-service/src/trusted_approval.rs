use blossom_core::ShellApprovalAuthentication;
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use zbus::proxy::{Builder as ProxyBuilder, CacheProperties};
use zbus::zvariant::{OwnedObjectPath, Value};
use zbus::{Proxy, connection};

pub const MODEL_EFFECT_POLKIT_ACTION: &str = "org.blossomos.shell.approve-model-effect";
const POLKIT_DESTINATION: &str = "org.freedesktop.PolicyKit1";
const POLKIT_PATH: &str = "/org/freedesktop/PolicyKit1/Authority";
const POLKIT_INTERFACE: &str = "org.freedesktop.PolicyKit1.Authority";
const LOGIN1_DESTINATION: &str = "org.freedesktop.login1";
const LOGIN1_PATH: &str = "/org/freedesktop/login1";
const LOGIN1_MANAGER_INTERFACE: &str = "org.freedesktop.login1.Manager";
const LOGIN1_SESSION_INTERFACE: &str = "org.freedesktop.login1.Session";
const SYSTEM_BUS_ADDRESS: &str = "unix:path=/run/dbus/system_bus_socket";
const ALLOW_USER_INTERACTION: u32 = 1;
const CHALLENGE_LIMIT: usize = 3;
const CHALLENGE_WINDOW: Duration = Duration::from_secs(60);
const CHALLENGE_COOLDOWN: Duration = Duration::from_secs(120);

// PolicyKit's CheckAuthorization result has the fixed D-Bus signature
// `(bba{ss})`.  It is never an optional value; modeling it as Option changes
// the expected wire shape and makes a real PolicyKit authority fail decoding.
type AuthorizationResponse = (bool, bool, HashMap<String, String>);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrustedApprovalResult {
    Authorized,
    Denied,
    Expired,
    Unavailable,
    ChallengeExpired,
    ChallengeUnavailable,
    InvalidEnvironment,
    RateLimited,
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
    challenge_limiter: SessionChallengeLimiter,
}

impl Default for PolkitTrustedApproval {
    fn default() -> Self {
        Self {
            address: SYSTEM_BUS_ADDRESS.into(),
            challenge_limiter: SessionChallengeLimiter::default(),
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
            &mut self.challenge_limiter,
        ))
    }
}

#[derive(Default)]
struct SessionChallengeLimiter {
    session_id: Option<String>,
    attempts: VecDeque<Instant>,
    cooldown_until: Option<Instant>,
}

impl SessionChallengeLimiter {
    fn admit(&mut self, session_id: &str, now: Instant) -> bool {
        if self.session_id.as_deref() != Some(session_id) {
            self.session_id = Some(session_id.into());
            self.attempts.clear();
            self.cooldown_until = None;
        }
        if let Some(until) = self.cooldown_until {
            if now < until {
                return false;
            }
            self.attempts.clear();
            self.cooldown_until = None;
        }
        while self
            .attempts
            .front()
            .is_some_and(|attempt| now.duration_since(*attempt) >= CHALLENGE_WINDOW)
        {
            self.attempts.pop_front();
        }
        if self.attempts.len() >= CHALLENGE_LIMIT {
            self.cooldown_until = Some(now + CHALLENGE_COOLDOWN);
            return false;
        }
        self.attempts.push_back(now);
        true
    }
}

async fn check_with_timeout(
    address: &str,
    timeout: Duration,
    challenge: &ShellApprovalAuthentication,
    challenge_limiter: &mut SessionChallengeLimiter,
) -> TrustedApprovalResult {
    use futures_lite::future::race;
    use std::sync::atomic::{AtomicBool, Ordering};

    let challenged = std::sync::Arc::new(AtomicBool::new(false));
    let observed = challenged.clone();
    race(
        check(address, challenge, observed, challenge_limiter),
        async move {
            async_io::Timer::after(timeout).await;
            if challenged.load(Ordering::SeqCst) {
                TrustedApprovalResult::ChallengeExpired
            } else {
                TrustedApprovalResult::Expired
            }
        },
    )
    .await
}

async fn check(
    address: &str,
    challenge: &ShellApprovalAuthentication,
    challenged: std::sync::Arc<std::sync::atomic::AtomicBool>,
    challenge_limiter: &mut SessionChallengeLimiter,
) -> TrustedApprovalResult {
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
    let Some(session_id) = active_local_graphical_session(&connection).await else {
        return TrustedApprovalResult::InvalidEnvironment;
    };
    check_for_session(
        &connection,
        challenge,
        challenged,
        &session_id,
        challenge_limiter,
    )
    .await
}

struct SessionCandidate {
    id: String,
    uid: u32,
    seat: String,
    active: bool,
    remote: bool,
    class: String,
    kind: String,
    state: String,
}

fn select_active_local_graphical_session(
    expected_uid: u32,
    candidates: impl IntoIterator<Item = SessionCandidate>,
) -> Option<String> {
    let mut eligible = candidates.into_iter().filter(|candidate| {
        candidate.uid == expected_uid
            && !candidate.seat.is_empty()
            && candidate.active
            && !candidate.remote
            && candidate.class == "user"
            && matches!(candidate.kind.as_str(), "wayland" | "x11")
            && candidate.state == "active"
    });
    let selected = eligible.next()?.id;
    eligible.next().is_none().then_some(selected)
}

async fn active_local_graphical_session(connection: &zbus::Connection) -> Option<String> {
    let manager: Proxy<'_> = ProxyBuilder::new(connection)
        .destination(LOGIN1_DESTINATION)
        .ok()?
        .path(LOGIN1_PATH)
        .ok()?
        .interface(LOGIN1_MANAGER_INTERFACE)
        .ok()?
        .cache_properties(CacheProperties::No)
        .build()
        .await
        .ok()?;
    let sessions: Vec<(String, u32, String, String, OwnedObjectPath)> =
        manager.call("ListSessions", &()).await.ok()?;
    let expected_uid = nix::unistd::geteuid().as_raw();
    let mut candidates = Vec::new();
    for (session_id, uid, _, seat, path) in sessions {
        if uid != expected_uid || seat.is_empty() {
            continue;
        }
        let session: Proxy<'_> = ProxyBuilder::new(connection)
            .destination(LOGIN1_DESTINATION)
            .ok()?
            .path(path)
            .ok()?
            .interface(LOGIN1_SESSION_INTERFACE)
            .ok()?
            .cache_properties(CacheProperties::No)
            .build()
            .await
            .ok()?;
        let active: bool = session.get_property("Active").await.ok()?;
        let remote: bool = session.get_property("Remote").await.ok()?;
        let class: String = session.get_property("Class").await.ok()?;
        let kind: String = session.get_property("Type").await.ok()?;
        let state: String = session.get_property("State").await.ok()?;
        candidates.push(SessionCandidate {
            id: session_id,
            uid,
            seat,
            active,
            remote,
            class,
            kind,
            state,
        });
    }
    select_active_local_graphical_session(expected_uid, candidates)
}

async fn check_for_session(
    connection: &zbus::Connection,
    challenge: &ShellApprovalAuthentication,
    challenged: std::sync::Arc<std::sync::atomic::AtomicBool>,
    session_id: &str,
    challenge_limiter: &mut SessionChallengeLimiter,
) -> TrustedApprovalResult {
    let authority: Proxy<'_> = match ProxyBuilder::new(connection)
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
    let subject_details = HashMap::from([("session-id", Value::from(session_id))]);
    let subject = ("unix-session", subject_details);
    let details = HashMap::from([
        ("blossom.request_id", challenge.request_id.clone()),
        ("blossom.preview_sha256", challenge.preview_sha256.clone()),
        ("blossom.effect", challenge.effect.into()),
        ("blossom.expires_at_ms", challenge.expires_at_ms.to_string()),
    ]);
    let arguments = |flags| {
        (
            (subject.0, subject.1.clone()),
            MODEL_EFFECT_POLKIT_ACTION,
            details.clone(),
            flags,
            "",
        )
    };
    let preflight: Result<AuthorizationResponse, zbus::Error> =
        authority.call("CheckAuthorization", &arguments(0)).await;
    match preflight {
        Ok((true, _, _)) => return TrustedApprovalResult::Authorized,
        Ok((false, true, _)) => {
            use std::sync::atomic::Ordering;
            challenged.store(true, Ordering::SeqCst);
        }
        Ok((false, false, _)) => return TrustedApprovalResult::InvalidEnvironment,
        Err(_) => return TrustedApprovalResult::Unavailable,
    }
    if !challenge_limiter.admit(session_id, Instant::now()) {
        return TrustedApprovalResult::RateLimited;
    }
    let response: Result<AuthorizationResponse, zbus::Error> = authority
        .call("CheckAuthorization", &arguments(ALLOW_USER_INTERACTION))
        .await;
    match response {
        Ok((true, _, _)) => TrustedApprovalResult::Authorized,
        Ok((false, _, _)) => TrustedApprovalResult::Denied,
        Err(_) => TrustedApprovalResult::ChallengeUnavailable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader};
    use std::process::{Child, Command, Stdio};
    use std::sync::{Arc, Mutex};
    use zbus::zvariant::OwnedValue;

    #[test]
    fn authorization_response_matches_polkit_wire_signature() {
        assert_eq!(
            <AuthorizationResponse as zbus::zvariant::Type>::SIGNATURE.to_string(),
            "(bba{ss})"
        );
    }

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
        Bypass,
        ChallengeAllow,
        ChallengeDeny,
        Inactive,
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
            let session_id = subject
                .1
                .get("session-id")
                .and_then(|value| value.try_clone().ok())
                .and_then(|value| String::try_from(value).ok());
            let (valid, behavior) = {
                let mut seen = self.0.lock().unwrap();
                let valid = subject.0 == "unix-session"
                    && session_id.as_deref() == Some("session-1")
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
                    && cancellation_id.is_empty();
                seen.calls += 1;
                seen.valid = valid;
                (valid, seen.behavior)
            };
            if matches!(behavior, Behavior::Delay) && flags == ALLOW_USER_INTERACTION {
                async_io::Timer::after(Duration::from_millis(100)).await;
            }
            Ok(match (behavior, flags) {
                (Behavior::Bypass, 0) => (valid, false, HashMap::new()),
                (Behavior::ChallengeAllow | Behavior::ChallengeDeny | Behavior::Delay, 0) => {
                    (false, valid, HashMap::new())
                }
                (Behavior::ChallengeAllow | Behavior::Delay, ALLOW_USER_INTERACTION) => {
                    (valid, false, HashMap::new())
                }
                (Behavior::ChallengeDeny, ALLOW_USER_INTERACTION) => (false, false, HashMap::new()),
                (Behavior::Inactive, 0) => (false, false, HashMap::new()),
                _ => (false, false, HashMap::new()),
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

    fn session(id: &str) -> SessionCandidate {
        SessionCandidate {
            id: id.into(),
            uid: 1_000,
            seat: "seat0".into(),
            active: true,
            remote: false,
            class: "user".into(),
            kind: "wayland".into(),
            state: "active".into(),
        }
    }

    #[test]
    fn session_selection_rejects_zero_sessions() {
        assert_eq!(
            select_active_local_graphical_session(1_000, Vec::<SessionCandidate>::new()),
            None
        );
    }

    #[test]
    fn session_selection_rejects_two_graphical_sessions() {
        assert_eq!(
            select_active_local_graphical_session(
                1_000,
                vec![session("session-1"), session("session-2")],
            ),
            None
        );
    }

    #[test]
    fn session_selection_rejects_remote_only_session() {
        let mut remote = session("remote-session");
        remote.remote = true;
        remote.seat.clear();
        assert_eq!(
            select_active_local_graphical_session(1_000, vec![remote]),
            None
        );
    }

    #[test]
    fn session_selection_accepts_one_active_local_graphical_session() {
        let mut remote = session("remote-session");
        remote.remote = true;
        assert_eq!(
            select_active_local_graphical_session(1_000, vec![remote, session("session-1")],),
            Some("session-1".into())
        );
    }

    #[test]
    fn challenge_limiter_allows_three_then_cools_down() {
        let start = Instant::now();
        let mut limiter = SessionChallengeLimiter::default();
        for seconds in 0..CHALLENGE_LIMIT {
            assert!(limiter.admit(
                "session-1",
                start + Duration::from_secs(seconds.try_into().unwrap())
            ));
        }
        assert!(!limiter.admit("session-1", start + Duration::from_secs(3)));
        assert!(!limiter.admit("session-1", start + Duration::from_secs(60)));
        assert!(limiter.admit(
            "session-1",
            start + Duration::from_secs(3) + CHALLENGE_COOLDOWN
        ));
    }

    #[test]
    fn challenge_limiter_is_scoped_to_trusted_session() {
        let start = Instant::now();
        let mut limiter = SessionChallengeLimiter::default();
        for seconds in 0..CHALLENGE_LIMIT {
            assert!(limiter.admit(
                "session-1",
                start + Duration::from_secs(seconds.try_into().unwrap())
            ));
        }
        assert!(!limiter.admit("session-1", start + Duration::from_secs(3)));
        assert!(limiter.admit("session-2", start + Duration::from_secs(4)));
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
        let result = async_io::block_on(async {
            let connection = connection::Builder::address(address.as_str())
                .unwrap()
                .max_queued(8)
                .method_timeout(Duration::from_secs(30))
                .build()
                .await
                .unwrap();
            let challenged = Arc::new(std::sync::atomic::AtomicBool::new(false));
            let observed = challenged.clone();
            let mut challenge_limiter = SessionChallengeLimiter::default();
            futures_lite::future::race(
                check_for_session(
                    &connection,
                    &challenge(),
                    observed,
                    "session-1",
                    &mut challenge_limiter,
                ),
                async move {
                    async_io::Timer::after(timeout).await;
                    if challenged.load(std::sync::atomic::Ordering::SeqCst) {
                        TrustedApprovalResult::ChallengeExpired
                    } else {
                        TrustedApprovalResult::Expired
                    }
                },
            )
            .await
        });
        (result, seen)
    }

    #[test]
    fn binds_authentication_to_exact_preview_over_dbus() {
        let (result, seen) =
            authorize_with_behavior(Behavior::ChallengeAllow, Duration::from_secs(2));
        assert_eq!(result, TrustedApprovalResult::Authorized);
        let seen = seen.lock().unwrap();
        assert_eq!(seen.calls, 2);
        assert!(seen.valid);
    }

    #[test]
    fn bypass_denial_inactive_environment_and_timeout_are_distinct() {
        assert_eq!(
            authorize_with_behavior(Behavior::Bypass, Duration::from_secs(2)).0,
            TrustedApprovalResult::Authorized
        );
        assert_eq!(
            authorize_with_behavior(Behavior::ChallengeDeny, Duration::from_secs(2)).0,
            TrustedApprovalResult::Denied
        );
        assert_eq!(
            authorize_with_behavior(Behavior::Inactive, Duration::from_secs(2)).0,
            TrustedApprovalResult::InvalidEnvironment
        );
        assert_eq!(
            authorize_with_behavior(Behavior::Delay, Duration::from_millis(10)).0,
            TrustedApprovalResult::ChallengeExpired
        );
    }
}
