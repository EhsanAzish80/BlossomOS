use serde::Serialize;
use std::fmt;

pub const PROC_ROOT: &str = "/proc";
pub const MAX_PROCESS_STATUS_BYTES: usize = 16 * 1024;
pub const MAX_PROCESS_STATUS_LINES: usize = 256;
pub const MAX_PROCESS_NAME_BYTES: usize = 64;
pub const MAX_PROCESS_RESULTS: usize = 256;
pub const MAX_PROCESS_DIRECTORY_ENTRIES: usize = 65_536;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessListSource {
    ProcStatusSameEffectiveUser,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessState {
    Running,
    Sleeping,
    DiskSleep,
    Stopped,
    TracingStop,
    Zombie,
    Dead,
    Idle,
    Other,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ProcessListEntry {
    pub process_id: u32,
    pub name: String,
    pub state: ProcessState,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ProcessList {
    pub source: ProcessListSource,
    pub processes: Vec<ProcessListEntry>,
    pub skipped_entries: u32,
    pub truncated: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessListError {
    UnsupportedPlatform,
    ReadDirectoryFailed,
    TooManyDirectoryEntries,
    InvalidStatus,
    StatusTooLarge,
}

impl fmt::Display for ProcessListError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::UnsupportedPlatform => "process listing is available only on Linux",
            Self::ReadDirectoryFailed => "could not read the Linux process directory",
            Self::TooManyDirectoryEntries => "the Linux process directory exceeded its bound",
            Self::InvalidStatus => "a process status record was malformed",
            Self::StatusTooLarge => "a process status record exceeded its bound",
        })
    }
}

impl std::error::Error for ProcessListError {}

pub trait ProcessListProvider {
    fn read_process_list(&mut self) -> Result<ProcessList, ProcessListError>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct UnavailableProcessListProvider;

impl ProcessListProvider for UnavailableProcessListProvider {
    fn read_process_list(&mut self) -> Result<ProcessList, ProcessListError> {
        Err(ProcessListError::UnsupportedPlatform)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ProcProcessListReader;

impl ProcessListProvider for ProcProcessListReader {
    fn read_process_list(&mut self) -> Result<ProcessList, ProcessListError> {
        read_native_process_list()
    }
}

#[cfg(not(target_os = "linux"))]
fn read_native_process_list() -> Result<ProcessList, ProcessListError> {
    Err(ProcessListError::UnsupportedPlatform)
}

#[cfg(target_os = "linux")]
fn read_native_process_list() -> Result<ProcessList, ProcessListError> {
    read_process_list_at(PROC_ROOT, nix::unistd::geteuid().as_raw())
}

#[cfg(target_os = "linux")]
fn read_process_list_at(
    proc_root: &str,
    effective_uid: u32,
) -> Result<ProcessList, ProcessListError> {
    use nix::fcntl::{OFlag, open, openat};
    use nix::sys::stat::Mode;
    use std::fs::{File, read_dir};
    use std::io::Read;
    use std::os::unix::fs::FileTypeExt;

    let mut pids = Vec::new();
    let mut directory_entries = 0_usize;
    for item in read_dir(proc_root).map_err(|_| ProcessListError::ReadDirectoryFailed)? {
        directory_entries += 1;
        if directory_entries > MAX_PROCESS_DIRECTORY_ENTRIES {
            return Err(ProcessListError::TooManyDirectoryEntries);
        }
        let item = item.map_err(|_| ProcessListError::ReadDirectoryFailed)?;
        let name = item.file_name();
        let Some(name) = name.to_str() else { continue };
        if !name.bytes().all(|byte| byte.is_ascii_digit()) {
            continue;
        }
        if let Ok(pid) = name.parse::<u32>()
            && pid > 0
        {
            pids.push(pid);
        }
    }
    pids.sort_unstable();

    let mut processes = Vec::new();
    let mut skipped_entries = 0_u32;
    let mut truncated = false;
    for pid in pids {
        let pid_path = format!("{proc_root}/{pid}");
        let pid_fd = match open(
            pid_path.as_str(),
            OFlag::O_RDONLY | OFlag::O_DIRECTORY | OFlag::O_NOFOLLOW | OFlag::O_CLOEXEC,
            Mode::empty(),
        ) {
            Ok(fd) => fd,
            Err(_) => {
                skipped_entries = skipped_entries.saturating_add(1);
                continue;
            }
        };
        let status_fd = match openat(
            &pid_fd,
            "status",
            OFlag::O_RDONLY | OFlag::O_NOFOLLOW | OFlag::O_CLOEXEC | OFlag::O_NONBLOCK,
            Mode::empty(),
        ) {
            Ok(fd) => fd,
            Err(_) => {
                skipped_entries = skipped_entries.saturating_add(1);
                continue;
            }
        };
        let mut file = File::from(status_fd);
        let metadata = match file.metadata() {
            Ok(metadata)
                if metadata.file_type().is_file()
                    && !metadata.file_type().is_fifo()
                    && !metadata.file_type().is_socket() =>
            {
                metadata
            }
            _ => {
                skipped_entries = skipped_entries.saturating_add(1);
                continue;
            }
        };
        if metadata.len() > MAX_PROCESS_STATUS_BYTES as u64 {
            skipped_entries = skipped_entries.saturating_add(1);
            continue;
        }
        let mut bytes = Vec::new();
        if file
            .by_ref()
            .take((MAX_PROCESS_STATUS_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .is_err()
            || bytes.len() > MAX_PROCESS_STATUS_BYTES
        {
            skipped_entries = skipped_entries.saturating_add(1);
            continue;
        }
        match parse_process_status(&bytes, pid, effective_uid) {
            Ok(Some(entry)) if processes.len() < MAX_PROCESS_RESULTS => processes.push(entry),
            Ok(Some(_)) => {
                truncated = true;
                break;
            }
            Ok(None) => {}
            Err(_) => skipped_entries = skipped_entries.saturating_add(1),
        }
    }
    Ok(ProcessList {
        source: ProcessListSource::ProcStatusSameEffectiveUser,
        processes,
        skipped_entries,
        truncated,
    })
}

pub fn parse_process_status(
    bytes: &[u8],
    expected_pid: u32,
    effective_uid: u32,
) -> Result<Option<ProcessListEntry>, ProcessListError> {
    if bytes.len() > MAX_PROCESS_STATUS_BYTES {
        return Err(ProcessListError::StatusTooLarge);
    }
    let text = std::str::from_utf8(bytes).map_err(|_| ProcessListError::InvalidStatus)?;
    if text.contains('\0') || text.lines().count() > MAX_PROCESS_STATUS_LINES {
        return Err(ProcessListError::InvalidStatus);
    }
    let mut name = None;
    let mut state = None;
    let mut pid = None;
    let mut uid = None;
    for line in text.lines() {
        if let Some(value) = line.strip_prefix("Name:\t") {
            if value.is_empty()
                || value.len() > MAX_PROCESS_NAME_BYTES
                || value.chars().any(char::is_control)
            {
                return Err(ProcessListError::InvalidStatus);
            }
            name = Some(value.to_owned());
        } else if let Some(value) = line.strip_prefix("State:\t") {
            state = Some(parse_state(value)?);
        } else if let Some(value) = line.strip_prefix("Pid:\t") {
            pid = Some(
                value
                    .trim()
                    .parse::<u32>()
                    .map_err(|_| ProcessListError::InvalidStatus)?,
            );
        } else if let Some(value) = line.strip_prefix("Uid:\t") {
            let ids = value
                .split_ascii_whitespace()
                .map(str::parse::<u32>)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| ProcessListError::InvalidStatus)?;
            if ids.len() != 4 {
                return Err(ProcessListError::InvalidStatus);
            }
            uid = Some(ids[1]);
        }
    }
    let (name, state, pid, uid) = match (name, state, pid, uid) {
        (Some(name), Some(state), Some(pid), Some(uid)) => (name, state, pid, uid),
        _ => return Err(ProcessListError::InvalidStatus),
    };
    if pid != expected_pid || pid == 0 {
        return Err(ProcessListError::InvalidStatus);
    }
    if uid != effective_uid {
        return Ok(None);
    }
    Ok(Some(ProcessListEntry {
        process_id: pid,
        name,
        state,
    }))
}

fn parse_state(value: &str) -> Result<ProcessState, ProcessListError> {
    let code = value
        .bytes()
        .next()
        .ok_or(ProcessListError::InvalidStatus)?;
    Ok(match code {
        b'R' => ProcessState::Running,
        b'S' => ProcessState::Sleeping,
        b'D' => ProcessState::DiskSleep,
        b'T' => ProcessState::Stopped,
        b't' => ProcessState::TracingStop,
        b'Z' => ProcessState::Zombie,
        b'X' | b'x' => ProcessState::Dead,
        b'I' => ProcessState::Idle,
        byte if byte.is_ascii_alphabetic() => ProcessState::Other,
        _ => return Err(ProcessListError::InvalidStatus),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status(name: &str, pid: u32, uid: u32) -> Vec<u8> {
        format!(
            "Name:\t{name}\nState:\tS (sleeping)\nPid:\t{pid}\nUid:\t{uid}\t{uid}\t{uid}\t{uid}\n"
        )
        .into_bytes()
    }

    #[test]
    fn parses_only_same_effective_user_with_allowlisted_fields() {
        let entry = parse_process_status(&status("blossom", 42, 1000), 42, 1000)
            .expect("valid status")
            .expect("same user");
        assert_eq!(entry.process_id, 42);
        assert_eq!(entry.name, "blossom");
        assert_eq!(entry.state, ProcessState::Sleeping);
        assert_eq!(
            parse_process_status(&status("other", 43, 2000), 43, 1000),
            Ok(None)
        );
    }

    #[test]
    fn rejects_pid_substitution_malformed_and_oversized_records() {
        assert_eq!(
            parse_process_status(&status("swap", 99, 1000), 42, 1000),
            Err(ProcessListError::InvalidStatus)
        );
        assert_eq!(
            parse_process_status(b"Name:\tbad\0name\n", 1, 1),
            Err(ProcessListError::InvalidStatus)
        );
        assert_eq!(
            parse_process_status(&vec![b'a'; MAX_PROCESS_STATUS_BYTES + 1], 1, 1),
            Err(ProcessListError::StatusTooLarge)
        );
    }

    #[test]
    fn rejects_control_characters_and_missing_required_fields() {
        assert_eq!(
            parse_process_status(
                b"Name:\tbad\x01name\nState:\tR\nPid:\t1\nUid:\t1\t1\t1\t1\n",
                1,
                1
            ),
            Err(ProcessListError::InvalidStatus)
        );
        assert_eq!(
            parse_process_status(b"Name:\tok\nPid:\t1\nUid:\t1\t1\t1\t1\n", 1, 1),
            Err(ProcessListError::InvalidStatus)
        );
    }

    #[cfg(target_os = "linux")]
    mod linux {
        use super::*;
        use std::fs::{self, File};
        use std::io::Write;
        use std::os::unix::fs::symlink;
        use std::path::PathBuf;

        struct FixtureRoot(PathBuf);
        impl FixtureRoot {
            fn new(label: &str) -> Self {
                let path = std::env::temp_dir().join(format!(
                    "blossom-process-list-{label}-{}-{}",
                    std::process::id(),
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .expect("clock")
                        .as_nanos()
                ));
                fs::create_dir(&path).expect("fixture root");
                Self(path)
            }
            fn path(&self) -> &str {
                self.0.to_str().expect("UTF-8 fixture path")
            }
        }
        impl Drop for FixtureRoot {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.0);
            }
        }

        fn write_status(root: &FixtureRoot, pid: u32, uid: u32) {
            let directory = root.0.join(pid.to_string());
            fs::create_dir(&directory).expect("PID directory");
            File::create(directory.join("status"))
                .expect("status")
                .write_all(&status("blossom", pid, uid))
                .expect("status bytes");
        }

        #[test]
        fn reads_regular_entries_and_rejects_pid_directory_symlinks() {
            let root = FixtureRoot::new("pid-symlink");
            write_status(&root, 42, 1000);
            symlink(root.0.join("42"), root.0.join("43")).expect("PID symlink");
            let list = read_process_list_at(root.path(), 1000).expect("bounded partial list");
            assert_eq!(list.processes.len(), 1);
            assert_eq!(list.processes[0].process_id, 42);
            assert_eq!(list.skipped_entries, 1);
        }

        #[test]
        fn rejects_status_symlinks_and_special_files_without_blocking() {
            let root = FixtureRoot::new("status-types");
            write_status(&root, 42, 1000);
            for pid in [43_u32, 44] {
                fs::create_dir(root.0.join(pid.to_string())).expect("PID directory");
            }
            symlink(root.0.join("42/status"), root.0.join("43/status")).expect("status symlink");
            nix::unistd::mkfifo(&root.0.join("44/status"), nix::sys::stat::Mode::S_IRUSR)
                .expect("status FIFO");
            let list = read_process_list_at(root.path(), 1000).expect("bounded partial list");
            assert_eq!(list.processes.len(), 1);
            assert_eq!(list.skipped_entries, 2);
        }

        #[test]
        fn reports_disappeared_and_pid_substituted_entries_as_skipped() {
            let root = FixtureRoot::new("races");
            write_status(&root, 42, 1000);
            fs::create_dir(root.0.join("43")).expect("disappearing PID shell");
            let directory = root.0.join("44");
            fs::create_dir(&directory).expect("substituted PID directory");
            File::create(directory.join("status"))
                .expect("status")
                .write_all(&status("replacement", 45, 1000))
                .expect("status bytes");
            let list = read_process_list_at(root.path(), 1000).expect("typed partial list");
            assert_eq!(list.processes.len(), 1);
            assert_eq!(list.skipped_entries, 2);
        }
    }
}
