//! Unix-specific process enumeration using sysinfo

use crate::types::{Pid, ProcessInfo};
use std::path::PathBuf;
use sysinfo::{ProcessesToUpdate, System};

/// Get all running processes with their paths
pub fn get_processes() -> Vec<ProcessInfo> {
    let mut system = System::new();
    system.refresh_processes(ProcessesToUpdate::All, true);

    system
        .processes()
        .iter()
        .filter_map(|(pid, process)| {
            let exe_path = process.exe()?;
            let args = process
                .cmd()
                .iter()
                .filter_map(|s| s.to_str())
                .collect::<Vec<_>>()
                .join(" ");
            Some(ProcessInfo {
                pid: pid.as_u32(),
                path: exe_path.to_path_buf(),
                args: if args.is_empty() { None } else { Some(args) },
            })
        })
        .collect()
}
