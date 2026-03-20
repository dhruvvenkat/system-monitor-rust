use serde::{Deserialize, Serialize};

use crate::{
    AppResult,
    model::{ProcessEntry, SystemSnapshot},
    processes::ProcessCollector,
    query::{self, Query},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorView {
    pub snapshot: SystemSnapshot,
    pub rows: Vec<ProcessEntry>,
}

impl MonitorView {
    // Keep the frontend payload narrow: summary data stays in the snapshot,
    // while rows contains only the filtered/sorted slice the UI needs to draw.
    pub fn from_snapshot(snapshot: SystemSnapshot, query: &Query) -> Self {
        let rows = query::apply(&snapshot, query)
            .into_iter()
            .cloned()
            .collect::<Vec<_>>();

        Self { snapshot, rows }
    }
}

// Both the terminal UI and the Tauri window should use the same snapshot/query
// pipeline so behavior stays consistent across presentation layers.
pub fn collect_view(collector: &mut ProcessCollector, query: &Query) -> AppResult<MonitorView> {
    let snapshot = collector.snapshot()?;
    Ok(MonitorView::from_snapshot(snapshot, query))
}
