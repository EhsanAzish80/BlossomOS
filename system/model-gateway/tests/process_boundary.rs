#![cfg(unix)]

#[cfg(target_os = "linux")]
use blossom_core::{
    GatewayProfile, NormalizedCompletion, NormalizedStreamKind, SyntheticGatewayClient,
    fixed_synthetic_gateway_request,
};
#[cfg(target_os = "linux")]
use std::fs;
#[cfg(target_os = "linux")]
use std::path::{Path, PathBuf};
#[cfg(target_os = "linux")]
use std::process::Child;
use std::process::{Command, Stdio};
#[cfg(target_os = "linux")]
use std::sync::atomic::{AtomicU64, Ordering};
#[cfg(target_os = "linux")]
use std::thread;
#[cfg(target_os = "linux")]
use std::time::{Duration, Instant};

#[cfg(target_os = "linux")]
const SOCKET_ENV: &str = "BLOSSOM_SYNTHETIC_GATEWAY_SOCKET";
#[cfg(target_os = "linux")]
const UID_ENV: &str = "BLOSSOM_SYNTHETIC_GATEWAY_CLIENT_UID";
#[cfg(target_os = "linux")]
const GID_ENV: &str = "BLOSSOM_SYNTHETIC_GATEWAY_CLIENT_GID";
#[cfg(target_os = "linux")]
const PROFILE_ENV: &str = "BLOSSOM_SYNTHETIC_GATEWAY_PROFILE";
#[cfg(target_os = "linux")]
static NEXT_SOCKET: AtomicU64 = AtomicU64::new(1);

#[cfg(target_os = "linux")]
struct FixtureProcess {
    child: Child,
    socket: PathBuf,
}

#[cfg(target_os = "linux")]
impl FixtureProcess {
    fn spawn(expected_uid: u32, expected_gid: u32) -> Self {
        let id = NEXT_SOCKET.fetch_add(1, Ordering::Relaxed);
        let socket = std::env::temp_dir().join(format!(
            "blossom-model-gateway-process-{}-{id}.sock",
            std::process::id()
        ));
        let child = Command::new(env!("CARGO_BIN_EXE_blossom-model-gateway"))
            .arg("--synthetic-fixture")
            .env(SOCKET_ENV, &socket)
            .env(UID_ENV, expected_uid.to_string())
            .env(GID_ENV, expected_gid.to_string())
            .env(PROFILE_ENV, "llama_cpp_cpu_v1")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        wait_for_socket(&socket);
        Self { child, socket }
    }

    fn wait(mut self) -> std::process::ExitStatus {
        let status = self.child.wait().unwrap();
        let _ = fs::remove_file(&self.socket);
        status
    }
}

#[cfg(target_os = "linux")]
impl Drop for FixtureProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = fs::remove_file(&self.socket);
    }
}

#[cfg(target_os = "linux")]
fn wait_for_socket(path: &Path) {
    let deadline = Instant::now() + Duration::from_secs(2);
    while !path.exists() {
        assert!(Instant::now() < deadline, "fixture socket was not created");
        thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn production_invocation_exits_not_ready_without_a_listener() {
    let output = Command::new(env!("CARGO_BIN_EXE_blossom-model-gateway"))
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(78));
    assert_eq!(output.stdout, b"");
    assert_eq!(output.stderr, b"model gateway is not production-ready\n");
}

#[test]
#[cfg(target_os = "linux")]
fn separate_process_serves_only_the_closed_synthetic_request() {
    let uid = nix::unistd::geteuid().as_raw();
    let gid = nix::unistd::getegid().as_raw();
    let fixture = FixtureProcess::spawn(uid, gid);
    let client = SyntheticGatewayClient::connect_at(
        &fixture.socket,
        uid,
        gid,
        GatewayProfile::LlamaCppCpuV1,
    )
    .unwrap();
    let events = client.infer(&fixed_synthetic_gateway_request()).unwrap();
    assert!(matches!(
        &events.last().unwrap().event,
        NormalizedStreamKind::Finished {
            completion: NormalizedCompletion::Text { content }
        } if content == "synthetic-gateway-ok"
    ));
    assert!(fixture.wait().success());
}

#[test]
#[cfg(target_os = "linux")]
fn wrong_client_identity_fails_before_hello_and_request() {
    let uid = nix::unistd::geteuid().as_raw();
    let gid = nix::unistd::getegid().as_raw();
    let fixture = FixtureProcess::spawn(uid.saturating_add(1), gid);
    assert!(
        SyntheticGatewayClient::connect_at(
            &fixture.socket,
            uid,
            gid,
            GatewayProfile::LlamaCppCpuV1,
        )
        .is_err()
    );
    assert!(!fixture.wait().success());
}
