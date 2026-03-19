use system_monitor::model::{ProcessEntry, ResourceSummary, SystemSnapshot};

#[test]
fn json_output_serializes_snapshot_deterministically() {
    let snapshot = SystemSnapshot {
        timestamp_millis: 1_725_000_000_000,
        summary: ResourceSummary {
            total_memory_bytes: 16 * 1024 * 1024 * 1024,
            used_memory_bytes: 8 * 1024 * 1024 * 1024,
            total_swap_bytes: 4 * 1024 * 1024 * 1024,
            used_swap_bytes: 512 * 1024 * 1024,
            global_cpu_percent: 12.5,
            process_count: 2,
        },
        processes: vec![
            ProcessEntry {
                pid: 1001,
                parent_pid: Some(1),
                name: "alpha".to_string(),
                command: "alpha --serve".to_string(),
                status: "Running".to_string(),
                cpu_percent: 8.5,
                memory_bytes: 64 * 1024 * 1024,
                virtual_memory_bytes: 128 * 1024 * 1024,
            },
            ProcessEntry {
                pid: 1002,
                parent_pid: None,
                name: "beta".to_string(),
                command: "beta --idle".to_string(),
                status: "Sleeping".to_string(),
                cpu_percent: 2.0,
                memory_bytes: 32 * 1024 * 1024,
                virtual_memory_bytes: 64 * 1024 * 1024,
            },
        ],
    };

    let json = system_monitor::ui::render_json(&snapshot).expect("json output");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid json");

    assert_eq!(parsed["timestamp_millis"], 1_725_000_000_000u64);
    assert_eq!(parsed["summary"]["process_count"], 2);
    assert_eq!(parsed["processes"][0]["name"], "alpha");
    assert_eq!(parsed["processes"][1]["pid"], 1002);
}
