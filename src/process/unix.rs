//! Unix-specific process enumeration using sysinfo

use crate::types::ProcessInfo;
use std::path::PathBuf;
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

/// Get all running processes with their paths
pub fn get_processes() -> Vec<ProcessInfo> {
    let mut system = System::new();
    system.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::new()
            .with_cmd(UpdateKind::Always)
            .with_exe(UpdateKind::Always),
    );

    system
        .processes()
        .iter()
        .filter_map(|(pid, process)| {
            let cmd = process.cmd();

            // Try argv[0] first, use exe symlink as a fallback
            let path = match cmd.first() {
                Some(first) if !first.is_empty() => PathBuf::from(first),
                _ => process.exe()?.to_path_buf(),
            };

            let args = cmd
                .iter()
                .skip(1)
                .filter_map(|s| s.to_str())
                .collect::<Vec<_>>()
                .join(" ");

            Some(ProcessInfo {
                pid: pid.as_u32(),
                path,
                args: (!args.is_empty()).then_some(args), // empty strings represent None
            })
        })
        .collect()
}
