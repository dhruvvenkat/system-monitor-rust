use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use system_monitor::{
    model::{ProcessEntry, SortField, SystemSnapshot},
    monitor::collect_view,
    processes::ProcessCollector,
    query,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorQuery {
    pub sort_by: SortField,
    pub ascending: bool,
    pub filter: Option<String>,
    pub limit: usize,
}

impl Default for MonitorQuery {
    fn default() -> Self {
        Self {
            sort_by: SortField::Cpu,
            ascending: false,
            filter: None,
            limit: 25,
        }
    }
}

impl MonitorQuery {
    fn into_query(self) -> query::Query {
        query::Query::from_cli(self.sort_by, self.ascending, self.filter, self.limit)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorSnapshot {
    pub query: MonitorQuery,
    pub snapshot: SystemSnapshot,
    pub processes: Vec<ProcessEntry>,
}

pub struct AppState {
    collector: Mutex<ProcessCollector>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            collector: Mutex::new(ProcessCollector::new()),
        }
    }
}

impl AppState {
    fn snapshot(&self, query: MonitorQuery) -> Result<MonitorSnapshot, String> {
        // Keep a single collector so repeated refreshes reuse the same sampling state.
        let mut collector = self
            .collector
            .lock()
            .map_err(|_| "collector lock poisoned")?;
        let core_query = query.clone().into_query();
        let view = collect_view(&mut collector, &core_query).map_err(|error| error.to_string())?;

        Ok(MonitorSnapshot {
            query,
            snapshot: view.snapshot,
            processes: view.rows,
        })
    }
}

#[tauri::command]
pub fn monitor_snapshot(
    state: tauri::State<'_, AppState>,
    query: Option<MonitorQuery>,
) -> Result<MonitorSnapshot, String> {
    state.snapshot(query.unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use system_monitor::model::ResourceSummary;

    fn process(pid: u32, name: &str, cpu: f32, memory: u64) -> ProcessEntry {
        ProcessEntry {
            pid,
            parent_pid: Some(1),
            name: name.to_string(),
            command: format!("{name} --flag"),
            status: "running".to_string(),
            cpu_percent: cpu,
            memory_bytes: memory,
            virtual_memory_bytes: memory * 2,
        }
    }

    fn snapshot() -> SystemSnapshot {
        SystemSnapshot {
            timestamp_millis: 1,
            summary: ResourceSummary {
                total_memory_bytes: 16,
                used_memory_bytes: 8,
                total_swap_bytes: 4,
                used_swap_bytes: 2,
                global_cpu_percent: 12.5,
                process_count: 3,
            },
            processes: vec![
                process(3, "gamma", 1.0, 30),
                process(1, "alpha", 5.0, 10),
                process(2, "beta", 5.0, 20),
            ],
        }
    }

    #[test]
    fn snapshot_helper_applies_query_and_preserves_snapshot() {
        let query = MonitorQuery {
            sort_by: SortField::Cpu,
            ascending: true,
            filter: Some("a".to_string()),
            limit: 2,
        };

        let core_query = query.clone().into_query();
        let processes = query::apply(&snapshot(), &core_query)
            .into_iter()
            .cloned()
            .collect::<Vec<_>>();
        let view = MonitorSnapshot {
            query: query.clone(),
            snapshot: snapshot(),
            processes,
        };

        assert_eq!(view.query.limit, 2);
        assert_eq!(view.snapshot.summary.process_count, 3);
        assert_eq!(
            view.processes
                .iter()
                .map(|process| process.pid)
                .collect::<Vec<_>>(),
            vec![3, 1]
        );
    }
}
