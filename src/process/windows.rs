//! Windows-specific process enumeration using sysinfo
//!
//! Much faster than arRPC's `wmic` approach - direct API calls.

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_processes() {
        let processes = get_processes();
        // Should at least find some processes
        assert!(!processes.is_empty(), "Should find some running processes");

        // Verify all have valid PIDs
        for p in &processes {
            assert!(p.pid > 0, "Process should have valid PID");
        }
    }
}
