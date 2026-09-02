#![forbid(unsafe_code)]

#[cfg(all(target_os = "linux", target_env = "gnu"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use blossom_core::privileged::{PRIVILEGED_BUS_NAME, PRIVILEGED_OBJECT_PATH};
    use blossom_privileged_helper::{
        FileAudit, FileJournal, PolkitAuthorizer, PrivilegedHelper, PrivilegedService,
        SystemdBluetoothManager,
    };

    let journal = FileJournal::open_root_owned("/run/blossom-privileged/journal")
        .map_err(|_| "privileged journal unavailable")?;
    let audit = FileAudit::open_root_owned("/run/blossom-privileged/audit")
        .map_err(|_| "privileged audit unavailable")?;
    let helper = PrivilegedHelper::new(
        PolkitAuthorizer::default(),
        SystemdBluetoothManager::default(),
        journal,
        audit,
    );
    let _connection = zbus::blocking::connection::Builder::system()?
        .name(PRIVILEGED_BUS_NAME)?
        .serve_at(PRIVILEGED_OBJECT_PATH, PrivilegedService::new(helper))?
        .build()?;
    loop {
        std::thread::park();
    }
}

#[cfg(not(all(target_os = "linux", target_env = "gnu")))]
fn main() {
    eprintln!("blossom-privileged-helper requires GNU/Linux");
    std::process::exit(1);
}
