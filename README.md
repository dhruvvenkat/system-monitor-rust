# system-monitor

`system-monitor` is a Rust system monitor inspired by `top` and `htop`.
Version 1 starts as a terminal UI and is structured so the same core data model can later back a browser-hosted UI or a Tauri shell.

## v1 scope

- Live process listing with CPU, memory, PID, parent PID, command, and status fields.
- Sorting and filtering over the process table.
- Terminal interaction through `ratatui` and `crossterm`.
- Single-shot output modes for scripting and tests.

## Usage

```bash
cargo run -- --help
cargo run -- --interval 1000 --sort cpu
cargo run -- --once
cargo run -- --json
```

## Controls

The terminal UI keeps the initial control set small:

- `q` exits.
- `r` refreshes immediately.
- `s` cycles the sort field.
- `a` or `d` toggles ascending and descending order.

## CLI options

- `-i, --interval <MS>` refresh interval in milliseconds. Default: `1000`.
- `-s, --sort <FIELD>` sort field. Accepted values: `cpu`, `memory`, `pid`, `name`.
- `--ascending` sort in ascending order instead of descending.
- `-f, --filter <TEXT>` filter processes by text match.
- `-l, --limit <COUNT>` maximum number of rows to display. Default: `25`.
- `--once` render a single snapshot and exit.
- `--json` emit a single snapshot as JSON and exit.

## Notes

The current implementation is intentionally CLI-first. The data model and rendering split are designed so a browser-hosted frontend can be added later without rewriting process collection or sorting logic.
