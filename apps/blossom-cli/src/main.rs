#![forbid(unsafe_code)]

use blossom_cli::{
    ApprovalChoice, Clock, Interaction, SystemBluetoothRestartTransport, exact_preview,
    file_read_preview, process_list_preview, run_bluetooth_restart, run_file_read,
    run_fixed_diagnostic, run_memory_summary, run_os_identity, run_process_list, run_process_self,
    run_service_status, run_storage_summary, run_uptime, run_workspace_create,
    service_status_preview, workspace_create_preview,
};
use blossom_core::{
    AtomicWorkspaceFileCreator, NativeProcessSelfReader, Openat2FileReader, OsReleaseReader,
    ProcMeminfoReader, ProcProcessListReader, ProcUptimeReader, RequestId, RootStorageReader,
    SystemdServiceStatusProvider, ToolRequest, executor::bubblewrap::BubblewrapExecutor,
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
    let storage_requested = arguments.as_slice() == ["storage-summary"];
    let process_self_requested = arguments.as_slice() == ["process-self"];
    let process_list_requested = arguments.as_slice() == ["process-list"];
    let bluetooth_restart_requested = arguments.as_slice() == ["bluetooth-try-restart"];
    let service_status_unit = if arguments.len() == 2 && arguments[0] == "service-status" {
        arguments[1].to_str()
    } else {
        None
    };
    let file_read_path = if arguments.len() == 2 && arguments[0] == "file-read" {
        arguments[1].to_str()
    } else {
        None
    };
    let workspace_create_input = if arguments.len() == 4 && arguments[0] == "workspace-create" {
        match (
            arguments[1].to_str(),
            arguments[2].to_str(),
            arguments[3].to_str(),
        ) {
            (Some(root), Some(destination), Some(content)) => Some((root, destination, content)),
            _ => None,
        }
    } else {
        None
    };
    if !arguments.is_empty()
        && !os_identity_requested
        && !uptime_requested
        && !memory_requested
        && !storage_requested
        && !process_self_requested
        && !process_list_requested
        && !bluetooth_restart_requested
        && file_read_path.is_none()
        && workspace_create_input.is_none()
        && service_status_unit.is_none()
    {
        eprintln!(
            "Usage: blossom-cli [os-identity|uptime|memory-summary|storage-summary|process-self|process-list|file-read ABSOLUTE_PATH|workspace-create ABSOLUTE_ROOT RELATIVE_DESTINATION CONTENT|service-status EXACT_SERVICE_UNIT|bluetooth-try-restart]\nNo executable or generic command argument input is supported."
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

    if bluetooth_restart_requested {
        let mut random = [0u8; 16];
        if getrandom::fill(&mut random).is_err() {
            eprintln!("Unable to generate a privileged idempotency key.");
            std::process::exit(1);
        }
        let idempotency_key = random
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let correlation_id = format!("priv-{now}-{}", std::process::id());
        let mut transport = SystemBluetoothRestartTransport;
        if !interaction.is_interactive() {
            let request = blossom_core::privileged::BluetoothRestartRequest {
                version: blossom_core::privileged::PRIVILEGED_PROTOCOL_VERSION,
                correlation_id: correlation_id.clone(),
                idempotency_key: idempotency_key.clone(),
                interactive: true,
            };
            println!("{}\n", blossom_cli::bluetooth_restart_preview(&request));
            println!("Non-interactive input is denied by default.\n");
        }
        let outcome = run_bluetooth_restart(
            &mut transport,
            &mut interaction,
            &mut clock,
            correlation_id,
            idempotency_key,
        );
        if let Some(result) = outcome.result {
            println!("{result}");
        }
        print!("{}", outcome.activity);
        std::process::exit(outcome.exit_code);
    }

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

    if storage_requested {
        let outcome = run_storage_summary(
            BubblewrapExecutor::phase1_default(),
            RootStorageReader,
            &mut clock,
            request_id,
        );
        if let Some(result) = outcome.result {
            println!("{result}");
        }
        print!("{}", outcome.activity);
        std::process::exit(outcome.exit_code);
    }

    if process_self_requested {
        let outcome = run_process_self(
            BubblewrapExecutor::phase1_default(),
            NativeProcessSelfReader,
            &mut clock,
            request_id,
        );
        if let Some(result) = outcome.result {
            println!("{result}");
        }
        print!("{}", outcome.activity);
        std::process::exit(outcome.exit_code);
    }

    if process_list_requested {
        let request = ToolRequest::ProcessList {
            request_id: request_id.clone(),
        };
        if !interaction.is_interactive() {
            println!("{}\n", process_list_preview(&request));
            println!("Non-interactive input is denied by default.\n");
        }
        let outcome = run_process_list(
            BubblewrapExecutor::phase1_default(),
            ProcProcessListReader,
            &mut interaction,
            &mut clock,
            request_id,
        );
        if let Some(result) = outcome.result {
            println!("{result}");
        }
        print!("{}", outcome.activity);
        std::process::exit(outcome.exit_code);
    }

    if let Some(path) = file_read_path {
        let reader = match Openat2FileReader::select(path) {
            Ok(reader) => reader,
            Err(error) => {
                eprintln!("File selection failed: {error}");
                std::process::exit(1);
            }
        };
        if !interaction.is_interactive() {
            let request = ToolRequest::FilesReadContent {
                request_id: request_id.clone(),
                selection: blossom_core::FileContentProvider::selection(&reader).clone(),
            };
            println!("{}\n", file_read_preview(&request));
            println!("Non-interactive input is denied by default.\n");
        }
        let outcome = run_file_read(
            BubblewrapExecutor::phase1_default(),
            reader,
            &mut interaction,
            &mut clock,
            request_id,
        );
        if let Some(result) = outcome.result {
            println!("{result}");
        }
        print!("{}", outcome.activity);
        std::process::exit(outcome.exit_code);
    }

    if let Some((root, destination, content)) = workspace_create_input {
        let creator = match AtomicWorkspaceFileCreator::select(root, destination, content) {
            Ok(creator) => creator,
            Err(error) => {
                eprintln!("Workspace selection failed: {error}");
                std::process::exit(1);
            }
        };
        if !interaction.is_interactive() {
            let request = ToolRequest::FilesWriteCreate {
                request_id: request_id.clone(),
                selection: blossom_core::WorkspaceCreateProvider::selection(&creator).clone(),
            };
            println!("{}\n", workspace_create_preview(&request));
            println!("Non-interactive input is denied by default.\n");
        }
        let outcome = run_workspace_create(
            BubblewrapExecutor::phase1_default(),
            creator,
            &mut interaction,
            &mut clock,
            request_id,
        );
        if let Some(result) = outcome.result {
            println!("{result}");
        }
        print!("{}", outcome.activity);
        std::process::exit(outcome.exit_code);
    }

    if let Some(unit) = service_status_unit {
        let request = ToolRequest::ServicesReadStatus {
            request_id: request_id.clone(),
            selection: blossom_core::ServiceSelection { unit: unit.into() },
        };
        if !interaction.is_interactive() {
            println!("{}\n", service_status_preview(&request));
            println!("Non-interactive input is denied by default.\n");
        }
        let outcome = run_service_status(
            BubblewrapExecutor::phase1_default(),
            SystemdServiceStatusProvider,
            &mut interaction,
            &mut clock,
            request_id,
            unit.into(),
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
