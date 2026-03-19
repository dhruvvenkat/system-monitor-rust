use std::fmt::Write as _;

use ratatui::{
    layout::{Constraint, Direction, Layout},
    prelude::*,
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Wrap},
};

use crate::{
    AppResult,
    model::{ProcessEntry, SortField, SystemSnapshot},
    query::Query,
};

pub fn render(
    frame: &mut Frame<'_>,
    snapshot: &SystemSnapshot,
    rows: &[&ProcessEntry],
    query: &Query,
    tick_rate: std::time::Duration,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Length(4),
            Constraint::Min(7),
            Constraint::Length(2),
        ])
        .split(frame.area());

    frame.render_widget(header_block(snapshot, query, rows.len()), chunks[0]);
    frame.render_widget(summary_block(snapshot, tick_rate), chunks[1]);

    if rows.is_empty() {
        frame.render_widget(empty_block(query), chunks[2]);
    } else {
        frame.render_widget(process_table(rows), chunks[2]);
    }

    frame.render_widget(help_block(), chunks[3]);
}

pub fn render_loading(frame: &mut Frame<'_>, query: &Query) {
    let block = Block::default()
        .title("system-monitor")
        .borders(Borders::ALL);
    let text = Paragraph::new(format!(
        "Loading process snapshot...\nSort: {} {} | Filter: {} | Limit: {}",
        sort_label(query.sort_by),
        direction_label(query.descending),
        query.filter.as_deref().unwrap_or("<none>"),
        query.limit
    ))
    .block(block)
    .wrap(Wrap { trim: true });

    frame.render_widget(text, frame.area());
}

pub fn render_once(snapshot: &SystemSnapshot, rows: &[&ProcessEntry], query: &Query) -> String {
    let mut out = String::new();

    let _ = writeln!(out, "system-monitor");
    let _ = writeln!(
        out,
        "timestamp={}  processes={}  cpu={:.1}%  memory={} / {}  swap={} / {}",
        snapshot.timestamp_millis,
        snapshot.summary.process_count,
        snapshot.summary.global_cpu_percent,
        format_bytes(snapshot.summary.used_memory_bytes),
        format_bytes(snapshot.summary.total_memory_bytes),
        format_bytes(snapshot.summary.used_swap_bytes),
        format_bytes(snapshot.summary.total_swap_bytes),
    );
    let _ = writeln!(
        out,
        "sort={} {}  filter={}  limit={}",
        sort_label(query.sort_by),
        direction_label(query.descending),
        query.filter.as_deref().unwrap_or("<none>"),
        query.limit,
    );
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "{:<8} {:<24} {:>8} {:>12} {:>12} {:<12} {}",
        "PID", "NAME", "CPU%", "MEM", "VIRT", "STATUS", "COMMAND"
    );

    if rows.is_empty() {
        let _ = writeln!(out, "No processes matched the current filter.");
        return out;
    }

    for process in rows {
        let _ = writeln!(
            out,
            "{:<8} {:<24} {:>7.1}% {:>12} {:>12} {:<12} {}",
            process.pid,
            truncate(&process.name, 24),
            process.cpu_percent,
            format_bytes(process.memory_bytes),
            format_bytes(process.virtual_memory_bytes),
            truncate(&process.status, 12),
            truncate(&process.command, 48)
        );
    }

    out
}

pub fn render_json(snapshot: &SystemSnapshot) -> AppResult<String> {
    Ok(serde_json::to_string_pretty(snapshot)?)
}

fn header_block(
    snapshot: &SystemSnapshot,
    query: &Query,
    visible_count: usize,
) -> Paragraph<'static> {
    Paragraph::new(format!(
        "system-monitor\nsort={} {}  filter={}  limit={}  visible={}  total={}",
        sort_label(query.sort_by),
        direction_label(query.descending),
        query.filter.as_deref().unwrap_or("<none>"),
        query.limit,
        visible_count,
        snapshot.summary.process_count
    ))
    .block(Block::default().borders(Borders::ALL).title("Overview"))
    .wrap(Wrap { trim: true })
}

fn summary_block(snapshot: &SystemSnapshot, tick_rate: std::time::Duration) -> Paragraph<'static> {
    Paragraph::new(format!(
        "CPU {:>5.1}% | Memory {} / {} | Swap {} / {}\nrefresh every {} ms",
        snapshot.summary.global_cpu_percent,
        format_bytes(snapshot.summary.used_memory_bytes),
        format_bytes(snapshot.summary.total_memory_bytes),
        format_bytes(snapshot.summary.used_swap_bytes),
        format_bytes(snapshot.summary.total_swap_bytes),
        tick_rate.as_millis()
    ))
    .block(Block::default().borders(Borders::ALL).title("Summary"))
    .wrap(Wrap { trim: true })
}

fn empty_block(query: &Query) -> Paragraph<'static> {
    Paragraph::new(format!(
        "No processes matched the current filter.\nSort: {} {}",
        sort_label(query.sort_by),
        direction_label(query.descending)
    ))
    .block(Block::default().borders(Borders::ALL).title("Processes"))
    .wrap(Wrap { trim: true })
}

fn process_table(rows: &[&ProcessEntry]) -> Table<'static> {
    let table_rows = rows.iter().map(|process| {
        Row::new(vec![
            Cell::from(process.pid.to_string()),
            Cell::from(process.name.clone()),
            Cell::from(format!("{:.1}%", process.cpu_percent)),
            Cell::from(format_bytes(process.memory_bytes)),
            Cell::from(format_bytes(process.virtual_memory_bytes)),
            Cell::from(process.status.clone()),
            Cell::from(process.command.clone()),
        ])
    });

    Table::new(
        table_rows,
        [
            Constraint::Length(8),
            Constraint::Length(24),
            Constraint::Length(8),
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Min(20),
        ],
    )
    .header(
        Row::new(vec![
            "PID", "NAME", "CPU%", "MEM", "VIRT", "STATUS", "COMMAND",
        ])
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
    )
    .block(Block::default().borders(Borders::ALL).title("Processes"))
    .column_spacing(1)
}

fn help_block() -> Paragraph<'static> {
    Paragraph::new(
        "q/Esc quit | r refresh | s next sort field | a/d toggle direction | Ctrl-C quit",
    )
    .style(Style::default().fg(Color::Gray))
    .block(Block::default().borders(Borders::ALL).title("Controls"))
}

fn sort_label(sort: SortField) -> &'static str {
    match sort {
        SortField::Cpu => "cpu",
        SortField::Memory => "memory",
        SortField::Pid => "pid",
        SortField::Name => "name",
    }
}

fn direction_label(descending: bool) -> &'static str {
    if descending { "desc" } else { "asc" }
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];

    let mut value = bytes as f64;
    let mut unit = 0;

    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }

    if unit == 0 {
        format!("{} {}", bytes, UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

fn truncate(value: &str, limit: usize) -> String {
    if value.chars().count() <= limit {
        return value.to_string();
    }

    let mut truncated = String::new();
    for (index, ch) in value.chars().enumerate() {
        if index >= limit.saturating_sub(1) {
            break;
        }
        truncated.push(ch);
    }
    truncated.push_str("...");
    truncated
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ResourceSummary, SortField, SystemSnapshot};

    fn sample_snapshot() -> SystemSnapshot {
        SystemSnapshot {
            timestamp_millis: 123,
            summary: ResourceSummary {
                total_memory_bytes: 8 * 1024 * 1024 * 1024,
                used_memory_bytes: 2 * 1024 * 1024 * 1024,
                total_swap_bytes: 1024 * 1024 * 1024,
                used_swap_bytes: 128 * 1024 * 1024,
                global_cpu_percent: 12.5,
                process_count: 2,
            },
            processes: vec![],
        }
    }

    #[test]
    fn render_once_includes_process_rows() {
        let snapshot = sample_snapshot();
        let alpha = ProcessEntry {
            pid: 42,
            parent_pid: None,
            name: "alpha".into(),
            command: "/usr/bin/alpha --flag".into(),
            status: "Running".into(),
            cpu_percent: 7.5,
            memory_bytes: 512,
            virtual_memory_bytes: 1024,
        };
        let beta = ProcessEntry {
            pid: 7,
            parent_pid: Some(1),
            name: "beta".into(),
            command: "/usr/bin/beta".into(),
            status: "Sleep".into(),
            cpu_percent: 1.0,
            memory_bytes: 2048,
            virtual_memory_bytes: 4096,
        };
        let rows = vec![&alpha, &beta];

        let query = Query {
            sort_by: SortField::Cpu,
            descending: true,
            filter: Some("alpha".into()),
            limit: 10,
        };

        let rendered = render_once(&snapshot, &rows, &query);

        assert!(rendered.contains("system-monitor"));
        assert!(rendered.contains("alpha"));
        assert!(rendered.contains("beta"));
        assert!(rendered.contains("sort=cpu desc"));
    }
}
