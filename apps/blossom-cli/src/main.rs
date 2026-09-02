#![forbid(unsafe_code)]

use blossom_cli::{
    ApprovalChoice, Clock, Interaction, exact_preview, run_fixed_diagnostic, run_memory_summary,
    run_os_identity, run_uptime,
};
use blossom_core::{
    OsReleaseReader, ProcMeminfoReader, ProcUptimeReader, RequestId, ToolRequest,
    executor::bubblewrap::BubblewrapExecutor,
};
use std::io::{self, IsTerminal, Write};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

struct SystemClock;

impl Clock for SystemClock {
    fn now_ms(&mut self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            .try_into()
            .unwrap_or(u64::MAX)
    }
}

struct TerminalInteraction;

impl Interaction for TerminalInteraction {
    fn is_interactive(&self) -> bool {
        io::stdin().is_terminal() && io::stdout().is_terminal()
    }

    fn choose(&mut self, preview: &str) -> ApprovalChoice {
        let (cancel_tx, cancel_rx) = mpsc::channel();
        if ctrlc::set_handler(move || {
            let _ = cancel_tx.send(());
        })
        .is_err()
        {
            return ApprovalChoice::Deny;
        }

        println!("{preview}\n");
        println!("[a] Approve once");
        println!("[d] Deny");
        print!("> ");
        let _ = io::stdout().flush();

        let (input_tx, input_rx) = mpsc::channel();
        thread::spawn(move || {
            let mut input = String::new();
            let result = io::stdin().read_line(&mut input).map(|_| input);
            let _ = input_tx.send(result);
        });
        loop {
            if cancel_rx.try_recv().is_ok() {
                println!("\nCancelled.");
                return ApprovalChoice::Cancel;
            }
            match input_rx.recv_timeout(Duration::from_millis(25)) {
                Ok(Ok(input)) if input.trim().eq_ignore_ascii_case("a") => {
                    return ApprovalChoice::ApproveOnce;
                }
                Ok(Ok(_)) | Ok(Err(_)) => return ApprovalChoice::Deny,
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => return ApprovalChoice::Deny,
            }
        }
    }
}

fn main() {
    let arguments = std::env::args_os().skip(1).collect::<Vec<_>>();
    let os_identity_requested = arguments.as_slice() == ["os-identity"];
    let uptime_requested = arguments.as_slice() == ["uptime"];
    let memory_requested = arguments.as_slice() == ["memory-summary"];
    if !arguments.is_empty() && !os_identity_requested && !uptime_requested && !memory_requested {
        eprintln!(
            "Usage: blossom-cli [os-identity|uptime|memory-summary]\nNo executable or argument input is supported."
        );
        std::process::exit(64);
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let request_id = RequestId::parse(format!("req-{now}-{}", std::process::id()))
        .expect("generated request identifier is valid");
    let mut interaction = TerminalInteraction;
    let mut clock = SystemClock;

    if os_identity_requested {
        let outcome = run_os_identity(
            BubblewrapExecutor::phase1_default(),
            OsReleaseReader::default(),
            &mut clock,
            request_id,
        );
        if let Some(result) = outcome.result {
            println!("{result}");
        }
        print!("{}", outcome.activity);
        std::process::exit(outcome.exit_code);
    }

    if uptime_requested {
        let outcome = run_uptime(
            BubblewrapExecutor::phase1_default(),
            ProcUptimeReader::default(),
            &mut clock,
            request_id,
        );
        if let Some(result) = outcome.result {
            println!("{result}");
        }
        print!("{}", outcome.activity);
        std::process::exit(outcome.exit_code);
    }

    if memory_requested {
        let outcome = run_memory_summary(
            BubblewrapExecutor::phase1_default(),
            ProcMeminfoReader::default(),
            &mut clock,
            request_id,
        );
        if let Some(result) = outcome.result {
            println!("{result}");
        }
        print!("{}", outcome.activity);
        std::process::exit(outcome.exit_code);
    }

    if !interaction.is_interactive() {
        let request = ToolRequest::SystemUname {
            request_id: request_id.clone(),
        };
        println!("{}\n", exact_preview(&request));
        println!("Non-interactive input is denied by default.\n");
    }

    let outcome = run_fixed_diagnostic(
        BubblewrapExecutor::phase1_default(),
        &mut interaction,
        &mut clock,
        request_id,
    );
    print!("{}", outcome.activity);
    std::process::exit(outcome.exit_code);
}
