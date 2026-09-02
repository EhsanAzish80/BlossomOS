use blossom_cli::{
    ApprovalChoice, BluetoothRestartTransport, Clock, Interaction, bluetooth_restart_preview,
    run_bluetooth_restart,
};
use blossom_core::privileged::{
    BLUETOOTH_UNIT, BluetoothObservation, BluetoothRestartOutcome, BluetoothRestartRequest,
    BluetoothRestartResult, PRIVILEGED_PROTOCOL_VERSION,
};

struct TestClock(Vec<u64>);

impl Clock for TestClock {
    fn now_ms(&mut self) -> u64 {
        self.0.remove(0)
    }
}

struct TestInteraction {
    interactive: bool,
    choice: ApprovalChoice,
    previews: Vec<String>,
}

impl Interaction for TestInteraction {
    fn is_interactive(&self) -> bool {
        self.interactive
    }

    fn choose(&mut self, preview: &str) -> ApprovalChoice {
        self.previews.push(preview.into());
        self.choice
    }
}

#[derive(Default)]
struct TestTransport {
    calls: usize,
    tamper: bool,
}

impl BluetoothRestartTransport for TestTransport {
    fn execute(
        &mut self,
        request: &BluetoothRestartRequest,
    ) -> Result<BluetoothRestartResult, String> {
        self.calls += 1;
        let before = BluetoothObservation {
            canonical_unit: BLUETOOTH_UNIT.into(),
            load_state: "loaded".into(),
            active_state: "active".into(),
            invocation_id: [1; 16],
        };
        let after = BluetoothObservation {
            invocation_id: if self.tamper { [1; 16] } else { [2; 16] },
            ..before.clone()
        };
        Ok(BluetoothRestartResult {
            version: PRIVILEGED_PROTOCOL_VERSION,
            correlation_id: request.correlation_id.clone(),
            authenticated_uid: 1000,
            request_sha256: request.normalized_digest(1000).unwrap(),
            replayed: false,
            outcome: BluetoothRestartOutcome::RestartedActive {
                before,
                after,
                job_result: "done".into(),
            },
        })
    }
}

fn interaction(interactive: bool, choice: ApprovalChoice) -> TestInteraction {
    TestInteraction {
        interactive,
        choice,
        previews: Vec::new(),
    }
}

fn run(
    transport: &mut TestTransport,
    interaction: &mut TestInteraction,
    times: Vec<u64>,
) -> blossom_cli::RunOutcome {
    run_bluetooth_restart(
        transport,
        interaction,
        &mut TestClock(times),
        "request-1".into(),
        "0".repeat(32),
    )
}

#[test]
fn exact_preview_and_once_only_approval_gate_the_fixed_helper_call() {
    let mut transport = TestTransport::default();
    let mut interaction = interaction(true, ApprovalChoice::ApproveOnce);
    let outcome = run(&mut transport, &mut interaction, vec![100, 101]);
    assert_eq!(outcome.exit_code, 0);
    assert_eq!(transport.calls, 1);
    assert_eq!(interaction.previews.len(), 1);
    let request = BluetoothRestartRequest {
        version: 1,
        correlation_id: "request-1".into(),
        idempotency_key: "0".repeat(32),
        interactive: true,
    };
    assert_eq!(interaction.previews[0], bluetooth_restart_preview(&request));
    assert!(interaction.previews[0].contains("bluetooth.service"));
    assert!(interaction.previews[0].contains("TryRestartUnit"));
    assert!(outcome.activity.contains("policy: ask"));
    assert!(outcome.activity.contains("decision: approved_once"));
    assert!(outcome.activity.contains("verification: verified"));
}

#[test]
fn denial_cancellation_expiry_and_noninteractive_mode_contact_no_helper() {
    for (interactive, choice, times, expected) in [
        (true, ApprovalChoice::Deny, vec![100, 101], 2),
        (true, ApprovalChoice::Cancel, vec![100, 101], 2),
        (true, ApprovalChoice::ApproveOnce, vec![100, 30_101], 3),
        (false, ApprovalChoice::ApproveOnce, vec![100], 2),
    ] {
        let mut transport = TestTransport::default();
        let mut interaction = interaction(interactive, choice);
        let outcome = run(&mut transport, &mut interaction, times);
        assert_eq!(outcome.exit_code, expected);
        assert_eq!(transport.calls, 0);
    }
}

#[test]
fn independently_rejects_a_false_success_result() {
    let mut transport = TestTransport {
        tamper: true,
        ..TestTransport::default()
    };
    let mut interaction = interaction(true, ApprovalChoice::ApproveOnce);
    let outcome = run(&mut transport, &mut interaction, vec![100, 101]);
    assert_eq!(transport.calls, 1);
    assert_eq!(outcome.exit_code, 1);
    assert!(outcome.activity.contains("completed_unverified"));
    assert!(outcome.activity.contains("verification: not_verified"));
}
