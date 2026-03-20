use serde::{Deserialize, Serialize};

use crate::model::{ProcessEntry, SortField, SystemSnapshot};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Query {
    pub sort_by: SortField,
    pub descending: bool,
    pub filter: Option<String>,
    pub limit: usize,
}

impl Query {
    pub fn from_cli(
        sort_by: SortField,
        ascending: bool,
        filter: Option<String>,
        limit: usize,
    ) -> Self {
        Self {
            sort_by,
            descending: !ascending,
            filter,
            limit,
        }
    }
}

pub fn apply<'a>(snapshot: &'a SystemSnapshot, query: &Query) -> Vec<&'a ProcessEntry> {
    let filter = query
        .filter
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_lowercase());

    let mut rows: Vec<&ProcessEntry> = snapshot
        .processes
        .iter()
        .filter(|process| matches_filter(process, filter.as_deref()))
        .collect();

    rows.sort_by(|left, right| compare_process(left, right, query.sort_by, query.descending));

    if query.limit == 0 {
        return Vec::new();
    }

    rows.truncate(query.limit);
    rows
}

fn matches_filter(process: &ProcessEntry, filter: Option<&str>) -> bool {
    let Some(filter) = filter else {
        return true;
    };

    let pid = process.pid.to_string();
    let parent_pid = process.parent_pid.map(|pid| pid.to_string());

    process.name.to_ascii_lowercase().contains(filter)
        || process.command.to_ascii_lowercase().contains(filter)
        || process.status.to_ascii_lowercase().contains(filter)
        || pid.contains(filter)
        || parent_pid
            .as_deref()
            .is_some_and(|value| value.contains(filter))
}

fn compare_process(
    left: &ProcessEntry,
    right: &ProcessEntry,
    sort_by: SortField,
    descending: bool,
) -> std::cmp::Ordering {
    let ordering = match sort_by {
        SortField::Cpu => left.cpu_percent.total_cmp(&right.cpu_percent),
        SortField::Memory => left.memory_bytes.cmp(&right.memory_bytes),
        SortField::Pid => left.pid.cmp(&right.pid),
        SortField::Name => left
            .name
            .to_ascii_lowercase()
            .cmp(&right.name.to_ascii_lowercase()),
    };

    if descending {
        ordering.reverse()
    } else {
        ordering
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ResourceSummary, SystemSnapshot};

    fn snapshot(processes: Vec<ProcessEntry>) -> SystemSnapshot {
        SystemSnapshot {
            timestamp_millis: 0,
            summary: ResourceSummary {
                total_memory_bytes: 0,
                used_memory_bytes: 0,
                total_swap_bytes: 0,
                used_swap_bytes: 0,
                global_cpu_percent: 0.0,
                process_count: processes.len(),
            },
            processes,
        }
    }

    fn process(
        pid: u32,
        name: &str,
        command: &str,
        status: &str,
        cpu_percent: f32,
        memory_bytes: u64,
    ) -> ProcessEntry {
        ProcessEntry {
            pid,
            parent_pid: Some(1),
            name: name.to_string(),
            command: command.to_string(),
            status: status.to_string(),
            cpu_percent,
            memory_bytes,
            virtual_memory_bytes: memory_bytes * 2,
        }
    }

    #[test]
    fn filters_case_insensitively_across_useful_fields() {
        let snapshot = snapshot(vec![
            process(10, "nginx", "nginx: worker", "sleeping", 12.0, 100),
            process(42, "postgres", "postgres: writer", "running", 8.0, 200),
            process(7, "other", "backup job", "idle", 1.0, 50),
        ]);

        let query = Query {
            sort_by: SortField::Pid,
            descending: false,
            filter: Some("POSTGRES".to_string()),
            limit: 10,
        };

        let rows = apply(&snapshot, &query);

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].pid, 42);
    }

    #[test]
    fn sorts_stably_by_cpu_and_respects_direction() {
        let snapshot = snapshot(vec![
            process(3, "first", "first", "running", 20.0, 100),
            process(1, "second", "second", "running", 40.0, 100),
            process(2, "third", "third", "running", 40.0, 100),
        ]);

        let ascending = Query {
            sort_by: SortField::Cpu,
            descending: false,
            filter: None,
            limit: 10,
        };

        let descending = Query {
            sort_by: SortField::Cpu,
            descending: true,
            filter: None,
            limit: 10,
        };

        let asc_rows = apply(&snapshot, &ascending);
        let desc_rows = apply(&snapshot, &descending);

        assert_eq!(
            asc_rows.iter().map(|row| row.pid).collect::<Vec<_>>(),
            vec![3, 1, 2]
        );
        assert_eq!(
            desc_rows.iter().map(|row| row.pid).collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
    }

    #[test]
    fn sorts_by_name_and_applies_limit() {
        let snapshot = snapshot(vec![
            process(9, "gamma", "gamma", "running", 1.0, 30),
            process(2, "alpha", "alpha", "running", 1.0, 10),
            process(7, "beta", "beta", "running", 1.0, 20),
        ]);

        let query = Query {
            sort_by: SortField::Name,
            descending: false,
            filter: None,
            limit: 2,
        };

        let rows = apply(&snapshot, &query);

        assert_eq!(
            rows.iter().map(|row| row.pid).collect::<Vec<_>>(),
            vec![2, 7]
        );
    }

    #[test]
    fn returns_empty_when_limit_is_zero() {
        let snapshot = snapshot(vec![process(1, "alpha", "alpha", "running", 1.0, 10)]);
        let query = Query {
            sort_by: SortField::Pid,
            descending: false,
            filter: None,
            limit: 0,
        };

        let rows = apply(&snapshot, &query);

        assert!(rows.is_empty());
    }
}
