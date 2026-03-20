const invoke = window.__TAURI__?.core?.invoke;

const state = {
  sortBy: "Cpu",
  ascending: false,
  filter: "",
  limit: 25,
  timer: null,
  busy: false,
};

const elements = {
  refresh: document.getElementById("refresh-btn"),
  direction: document.getElementById("direction-btn"),
  sortField: document.getElementById("sort-field"),
  filter: document.getElementById("filter-input"),
  limit: document.getElementById("limit-input"),
  summary: document.getElementById("summary-grid"),
  table: document.getElementById("process-table"),
  status: document.getElementById("status-line"),
};

// Keep formatting local to the frontend so the Rust side only returns raw data.
function formatBytes(value) {
  const units = ["B", "KiB", "MiB", "GiB", "TiB"];
  let size = value;
  let unit = 0;

  while (size >= 1024 && unit < units.length - 1) {
    size /= 1024;
    unit += 1;
  }

  return unit === 0 ? `${size} ${units[unit]}` : `${size.toFixed(1)} ${units[unit]}`;
}

function formatPercent(value) {
  return `${value.toFixed(1)}%`;
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
}

function statCard(label, value) {
  return `
    <article class="stat fade-in">
      <div class="label">${label}</div>
      <div class="value">${value}</div>
    </article>
  `;
}

function renderSummary(snapshot) {
  elements.summary.innerHTML = [
    statCard("Processes", snapshot.summary.process_count),
    statCard("CPU", formatPercent(snapshot.summary.global_cpu_percent)),
    statCard(
      "Memory",
      `${formatBytes(snapshot.summary.used_memory_bytes)} / ${formatBytes(snapshot.summary.total_memory_bytes)}`
    ),
    statCard(
      "Swap",
      `${formatBytes(snapshot.summary.used_swap_bytes)} / ${formatBytes(snapshot.summary.total_swap_bytes)}`
    ),
  ].join("");
}

function rowHtml(process) {
  return `
    <tr class="fade-in">
      <td>${process.pid}</td>
      <td>${escapeHtml(process.name)}</td>
      <td><span class="chip">${formatPercent(process.cpu_percent)}</span></td>
      <td>${formatBytes(process.memory_bytes)}</td>
      <td>${escapeHtml(process.status)}</td>
      <td class="command" title="${escapeHtml(process.command)}">${escapeHtml(process.command)}</td>
    </tr>
  `;
}

function renderRows(processes) {
  if (!processes.length) {
    elements.table.innerHTML = `
      <tr>
        <td colspan="6" class="meta">No processes match the current filter.</td>
      </tr>
    `;
    return;
  }

  elements.table.innerHTML = processes.map(rowHtml).join("");
}

function setBusy(busy) {
  state.busy = busy;
  elements.refresh.disabled = busy;
  elements.refresh.textContent = busy ? "Refreshing..." : "Refresh";
}

function syncControls() {
  elements.direction.textContent = state.ascending ? "Ascending" : "Descending";
  elements.sortField.value = state.sortBy;
  elements.filter.value = state.filter;
  elements.limit.value = String(state.limit);
}

async function refresh() {
  if (!invoke) {
    elements.status.textContent = "Tauri invoke API unavailable.";
    return;
  }

  setBusy(true);
  elements.status.textContent = "Fetching snapshot...";

  try {
    // The query shape mirrors the Rust-side MonitorQuery IPC contract.
    const payload = await invoke("monitor_snapshot", {
      query: {
        sortBy: state.sortBy,
        ascending: state.ascending,
        filter: state.filter || null,
        limit: state.limit,
      },
    });

    renderSummary(payload.snapshot);
    renderRows(payload.processes);
    elements.status.textContent = `Updated ${new Date(payload.snapshot.timestamp_millis).toLocaleTimeString()}`;
  } catch (error) {
    elements.status.textContent = String(error);
    elements.table.innerHTML = `
      <tr>
        <td colspan="6" class="meta">Unable to load snapshot.</td>
      </tr>
    `;
  } finally {
    setBusy(false);
  }
}

function scheduleRefresh() {
  // Centralize the polling interval so manual refreshes and auto-refresh stay in sync.
  clearInterval(state.timer);
  state.timer = setInterval(refresh, 2000);
}

elements.refresh.addEventListener("click", refresh);
elements.direction.addEventListener("click", () => {
  state.ascending = !state.ascending;
  syncControls();
  refresh();
});
elements.sortField.addEventListener("change", (event) => {
  state.sortBy = event.target.value;
  refresh();
});
elements.filter.addEventListener("input", (event) => {
  state.filter = event.target.value;
  refresh();
});
elements.limit.addEventListener("change", (event) => {
  state.limit = Math.max(1, Number(event.target.value) || 25);
  refresh();
});

syncControls();
scheduleRefresh();
refresh();
