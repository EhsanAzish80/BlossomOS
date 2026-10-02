use blossom_core::ShellApprovalAuthentication;
use std::collections::{HashMap, VecDeque};
use std::fs;
use std::path::Path;
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
    let Some(session) = active_local_graphical_session(&connection).await else {
        return TrustedApprovalResult::InvalidEnvironment;
    };
    let expected_uid = nix::unistd::geteuid().as_raw();
    let Some(process) = trusted_session_process(&session.id, session.leader, expected_uid) else {
        return TrustedApprovalResult::InvalidEnvironment;
    };
    check_for_session(
        &connection,
        challenge,
        challenged,
        &session.id,
        &process,
        challenge_limiter,
    )
    .await
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TrustedSessionProcess {
    pid: u32,
    uid: i32,
    start_time: u64,
}

#[derive(Clone, Debug)]
struct SessionProcessCandidate {
    pid: u32,
    parent_pid: u32,
    uid: u32,
    cgroup: String,
    start_time: u64,
    stable_start_time: Option<u64>,
}

fn proc_start_time(stat: &str) -> Option<u64> {
    let after_name = stat.rsplit_once(')')?.1.trim_start();
    after_name.split_whitespace().nth(19)?.parse().ok()
}

fn proc_parent_pid(stat: &str) -> Option<u32> {
    let after_name = stat.rsplit_once(')')?.1.trim_start();
    after_name.split_whitespace().nth(1)?.parse().ok()
}

fn proc_real_uid(status: &str) -> Option<u32> {
    status.lines().find_map(|line| {
        line.strip_prefix("Uid:")?
            .split_whitespace()
            .next()?
            .parse()
            .ok()
    })
}

fn cgroup_matches_session(cgroup: &str, session_id: &str) -> bool {
    let suffix = format!("/session-{session_id}.scope");
    cgroup.lines().any(|line| line.ends_with(&suffix))
}

fn select_trusted_session_process(
    session_id: &str,
    session_leader: u32,
    expected_uid: u32,
    candidates: impl IntoIterator<Item = SessionProcessCandidate>,
) -> Option<TrustedSessionProcess> {
    let mut matches = candidates.into_iter().filter_map(|candidate| {
        (candidate.parent_pid == session_leader).then_some(())?;
        (candidate.uid == expected_uid).then_some(())?;
        cgroup_matches_session(&candidate.cgroup, session_id).then_some(())?;
        (candidate.stable_start_time == Some(candidate.start_time)).then_some(())?;
        Some(TrustedSessionProcess {
            pid: candidate.pid,
            uid: expected_uid.try_into().ok()?,
            start_time: candidate.start_time,
        })
    });
    let selected = matches.next()?;
    matches.next().is_none().then_some(selected)
}

fn trusted_session_process(
    session_id: &str,
    session_leader: u32,
    expected_uid: u32,
) -> Option<TrustedSessionProcess> {
    let leader_root = format!("/proc/{session_leader}");
    let leader_status = fs::read_to_string(format!("{leader_root}/status")).ok()?;
    (proc_real_uid(&leader_status)? == 0).then_some(())?;
    let leader_cgroup = fs::read_to_string(format!("{leader_root}/cgroup")).ok()?;
    cgroup_matches_session(&leader_cgroup, session_id).then_some(())?;
    let candidates = fs::read_dir("/proc")
        .ok()?
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().to_str()?.parse::<u32>().ok())
        .filter_map(|pid| {
            let root = Path::new("/proc").join(pid.to_string());
            let first_stat = fs::read_to_string(root.join("stat")).ok()?;
            let start_time = proc_start_time(&first_stat)?;
            let status = fs::read_to_string(root.join("status")).ok()?;
            let cgroup = fs::read_to_string(root.join("cgroup")).ok()?;
            let second_stat = fs::read_to_string(root.join("stat")).ok()?;
            Some(SessionProcessCandidate {
                pid,
                parent_pid: proc_parent_pid(&first_stat)?,
                uid: proc_real_uid(&status)?,
                cgroup,
                start_time,
                stable_start_time: proc_start_time(&second_stat),
            })
        });
    select_trusted_session_process(session_id, session_leader, expected_uid, candidates)
}

struct SessionCandidate {
    id: String,
    leader: u32,
    uid: u32,
    seat: String,
    active: bool,
    remote: bool,
    class: String,
    kind: String,
    state: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ActiveGraphicalSession {
    id: String,
    leader: u32,
}

fn select_active_local_graphical_session(
    expected_uid: u32,
    candidates: impl IntoIterator<Item = SessionCandidate>,
) -> Option<ActiveGraphicalSession> {
    let mut eligible = candidates.into_iter().filter(|candidate| {
        candidate.uid == expected_uid
            && !candidate.seat.is_empty()
            && candidate.active
            && !candidate.remote
            && candidate.class == "user"
            && matches!(candidate.kind.as_str(), "wayland" | "x11")
            && candidate.state == "active"
    });
    let selected = eligible.next()?;
    let selected = ActiveGraphicalSession {
        id: selected.id,
        leader: selected.leader,
    };
    eligible.next().is_none().then_some(selected)
}

async fn active_local_graphical_session(
    connection: &zbus::Connection,
) -> Option<ActiveGraphicalSession> {
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
        let leader: u32 = session.get_property("Leader").await.ok()?;
        candidates.push(SessionCandidate {
            id: session_id,
            leader,
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
    process: &TrustedSessionProcess,
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
    // The D-Bus API accepts unix-session subjects, but polkit's JavaScript
    // authority cannot convert them for rule evaluation. Bind authorization to
    // the uniquely verified compositor process inside the already selected
    // active local graphical session instead.
    let subject_details = HashMap::from([
        ("pid", Value::from(process.pid)),
        ("uid", Value::from(process.uid)),
        ("start-time", Value::from(process.start_time)),
    ]);
    let subject = ("unix-process", subject_details);
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
            let pid = subject
                .1
                .get("pid")
                .and_then(|value| value.try_clone().ok())
                .and_then(|value| u32::try_from(value).ok());
            let uid = subject
                .1
                .get("uid")
                .and_then(|value| value.try_clone().ok())
                .and_then(|value| i32::try_from(value).ok());
            let start_time = subject
                .1
                .get("start-time")
                .and_then(|value| value.try_clone().ok())
                .and_then(|value| u64::try_from(value).ok());
            let (valid, behavior) = {
                let mut seen = self.0.lock().unwrap();
                let valid = subject.0 == "unix-process"
                    && pid == Some(463)
                    && uid == Some(1_000)
                    && start_time == Some(12_345)
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
            leader: 400,
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
            Some(ActiveGraphicalSession {
                id: "session-1".into(),
                leader: 400,
            })
        );
    }

    #[test]
    fn parses_process_identity_fields_used_by_polkit() {
        let stat = "463 (Hyprland) S 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 12345 20";
        assert_eq!(proc_start_time(stat), Some(12_345));
        assert_eq!(proc_parent_pid(stat), Some(1));
        assert_eq!(
            proc_real_uid("Name:\tHyprland\nUid:\t1000\t1000\t1000\t1000\n"),
            Some(1_000)
        );
        assert!(cgroup_matches_session(
            "0::/user.slice/user-1000.slice/session-1.scope\n",
            "1"
        ));
        assert!(!cgroup_matches_session(
            "0::/user.slice/user-1000.slice/user@1000.service\n",
            "1"
        ));
    }

    fn process_candidate() -> SessionProcessCandidate {
        SessionProcessCandidate {
            pid: 449,
            parent_pid: 421,
            uid: 1_000,
            cgroup: "0::/user.slice/user-1000.slice/session-1.scope\n".into(),
            start_time: 12_345,
            stable_start_time: Some(12_345),
        }
    }

    #[test]
    fn session_process_selection_binds_parent_uid_scope_and_start_time() {
        assert_eq!(
            select_trusted_session_process("1", 421, 1_000, [process_candidate()]),
            Some(TrustedSessionProcess {
                pid: 449,
                uid: 1_000,
                start_time: 12_345,
            })
        );

        let mut changed_parent = process_candidate();
        changed_parent.parent_pid = 1;
        assert_eq!(
            select_trusted_session_process("1", 421, 1_000, [changed_parent]),
            None
        );

        let mut changed_uid = process_candidate();
        changed_uid.uid = 1_001;
        assert_eq!(
            select_trusted_session_process("1", 421, 1_000, [changed_uid]),
            None
        );

        let mut changed_scope = process_candidate();
        changed_scope.cgroup = "0::/user.slice/user-1000.slice/user@1000.service\n".into();
        assert_eq!(
            select_trusted_session_process("1", 421, 1_000, [changed_scope]),
            None
        );

        let mut reused_pid = process_candidate();
        reused_pid.stable_start_time = Some(12_346);
        assert_eq!(
            select_trusted_session_process("1", 421, 1_000, [reused_pid]),
            None
        );
    }

    #[test]
    fn session_process_selection_rejects_ambiguous_direct_children() {
        let first = process_candidate();
        let mut second = process_candidate();
        second.pid = 450;
        assert_eq!(
            select_trusted_session_process("1", 421, 1_000, [first, second]),
            None
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
            let process = TrustedSessionProcess {
                pid: 463,
                uid: 1_000,
                start_time: 12_345,
            };
            futures_lite::future::race(
                check_for_session(
                    &connection,
                    &challenge(),
                    observed,
                    "session-1",
                    &process,
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
