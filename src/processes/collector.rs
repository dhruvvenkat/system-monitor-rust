use std::time::{SystemTime, UNIX_EPOCH};

use sysinfo::System;

use crate::{
    AppResult,
    model::{ProcessEntry, ResourceSummary, SystemSnapshot},
};

#[derive(Debug)]
pub struct ProcessCollector {
    system: System,
}

impl ProcessCollector {
    pub fn new() -> Self {
        let system = System::new_all();
        Self { system }
    }

    pub fn snapshot(&mut self) -> AppResult<SystemSnapshot> {
        self.system.refresh_all();

        let mut processes = self
            .system
            .processes()
            .iter()
            .map(|(pid, process)| ProcessEntry {
                pid: pid.as_u32(),
                parent_pid: process.parent().map(|parent| parent.as_u32()),
                name: process.name().to_string_lossy().into_owned(),
                command: {
                    let name = process.name().to_string_lossy();
                    command_line(process.cmd(), name.as_ref(), pid.as_u32())
                },
                status: process.status().to_string(),
                cpu_percent: process.cpu_usage(),
                memory_bytes: process.memory(),
                virtual_memory_bytes: process.virtual_memory(),
            })
            .collect::<Vec<_>>();

        processes.sort_by_key(|entry| entry.pid);

        Ok(SystemSnapshot {
            timestamp_millis: timestamp_millis(),
            summary: ResourceSummary {
                total_memory_bytes: self.system.total_memory(),
                used_memory_bytes: self.system.used_memory(),
                total_swap_bytes: self.system.total_swap(),
                used_swap_bytes: self.system.used_swap(),
                global_cpu_percent: self.system.global_cpu_usage(),
                process_count: processes.len(),
            },
            processes,
        })
    }
}

fn timestamp_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis())
}

fn command_line(command: &[std::ffi::OsString], fallback_name: &str, pid: u32) -> String {
    if command.is_empty() {
        if fallback_name.is_empty() {
            return pid.to_string();
        }
        return fallback_name.to_owned();
    }

    command
        .iter()
        .map(|part| part.to_string_lossy())
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_line_prefers_command_arguments() {
        let command = vec!["/bin/sh".into(), "-c".into(), "echo hi".into()];

        assert_eq!(command_line(&command, "fallback", 7), "/bin/sh -c echo hi");
    }

    #[test]
    fn command_line_falls_back_to_name_then_pid() {
        let empty: Vec<std::ffi::OsString> = Vec::new();

        assert_eq!(command_line(&empty, "shell", 7), "shell");
        assert_eq!(command_line(&empty, "", 7), "7");
    }
}
