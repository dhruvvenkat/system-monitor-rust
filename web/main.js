const invoke = window.__TAURI__?.core?.invoke;
const MAX_HISTORY_POINTS = 60;
const GRAPH_WIDTH = 960;
const GRAPH_HEIGHT = 260;
const GRAPH_PADDING = { top: 18, right: 22, bottom: 28, left: 18 };

const GRAPH_METRICS = {
  cpu: {
    label: "CPU usage",
    scaleLabel: "0-100% scale",
    color: "#79d7ff",
    extract(snapshot) {
      return snapshot.summary.global_cpu_percent;
    },
    format(value) {
      return formatPercent(value);
    },
    maxValue() {
      return 100;
    },
  },
  memory: {
    label: "Memory usage",
    scaleLabel: "0-100% scale",
    color: "#8ef0b4",
    extract(snapshot) {
      return percentage(snapshot.summary.used_memory_bytes, snapshot.summary.total_memory_bytes);
    },
    format(value) {
      return formatPercent(value);
    },
    maxValue() {
      return 100;
    },
  },
  swap: {
    label: "Swap usage",
    scaleLabel: "0-100% scale",
    color: "#ffd479",
    extract(snapshot) {
      return percentage(snapshot.summary.used_swap_bytes, snapshot.summary.total_swap_bytes);
    },
    format(value, sample) {
      if (sample.swapTotalBytes === 0) {
        return "No swap";
      }

      return formatPercent(value);
    },
    maxValue() {
      return 100;
    },
  },
  processes: {
    label: "Process count",
    scaleLabel: "Dynamic scale",
    color: "#c0a3ff",
    extract(snapshot) {
      return snapshot.summary.process_count;
    },
    format(value) {
      return `${Math.round(value)}`;
    },
    maxValue(values) {
      return Math.max(...values, 1);
    },
  },
};

const state = {
  sortBy: "Cpu",
  ascending: false,
  filter: "",
  limit: 25,
  graphMetric: "cpu",
  history: [],
  timer: null,
  busy: false,
};

const elements = {
  refresh: document.getElementById("refresh-btn"),
  direction: document.getElementById("direction-btn"),
  sortField: document.getElementById("sort-field"),
  filter: document.getElementById("filter-input"),
  limit: document.getElementById("limit-input"),
  graphMetric: document.getElementById("graph-metric"),
  graph: document.getElementById("usage-graph"),
  graphEmpty: document.getElementById("graph-empty"),
  graphLabel: document.getElementById("graph-label"),
  graphCurrentValue: document.getElementById("graph-current-value"),
  graphScaleLabel: document.getElementById("graph-scale-label"),
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

function percentage(part, total) {
  if (!total) {
    return 0;
  }

  return (part / total) * 100;
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

function buildGraphSample(snapshot) {
  return {
    timestamp: snapshot.timestamp_millis,
    cpu: GRAPH_METRICS.cpu.extract(snapshot),
    memory: GRAPH_METRICS.memory.extract(snapshot),
    swap: GRAPH_METRICS.swap.extract(snapshot),
    swapTotalBytes: snapshot.summary.total_swap_bytes,
    processes: GRAPH_METRICS.processes.extract(snapshot),
  };
}

function pushHistory(snapshot) {
  // Store all graph metrics per sample so switching the dropdown can redraw immediately.
  state.history.push(buildGraphSample(snapshot));

  if (state.history.length > MAX_HISTORY_POINTS) {
    state.history = state.history.slice(-MAX_HISTORY_POINTS);
  }
}

function graphCoordinates(values, maxValue) {
  const innerWidth = GRAPH_WIDTH - GRAPH_PADDING.left - GRAPH_PADDING.right;
  const innerHeight = GRAPH_HEIGHT - GRAPH_PADDING.top - GRAPH_PADDING.bottom;

  return values.map((value, index) => {
    const x =
      values.length === 1
        ? GRAPH_PADDING.left + innerWidth
        : GRAPH_PADDING.left + (innerWidth * index) / (values.length - 1);
    const y =
      GRAPH_PADDING.top + innerHeight - (Math.min(value, maxValue) / maxValue) * innerHeight;

    return { x, y };
  });
}

function linePath(points) {
  return points
    .map((point, index) => `${index === 0 ? "M" : "L"} ${point.x.toFixed(2)} ${point.y.toFixed(2)}`)
    .join(" ");
}

function areaPath(points) {
  if (!points.length) {
    return "";
  }

  const baseY = GRAPH_HEIGHT - GRAPH_PADDING.bottom;

  return [
    `M ${points[0].x.toFixed(2)} ${baseY.toFixed(2)}`,
    ...points.map((point) => `L ${point.x.toFixed(2)} ${point.y.toFixed(2)}`),
    `L ${points.at(-1).x.toFixed(2)} ${baseY.toFixed(2)}`,
    "Z",
  ].join(" ");
}

function renderGraph() {
  const metric = GRAPH_METRICS[state.graphMetric];
  const values = state.history.map((sample) => sample[state.graphMetric]);
  const latestSample = state.history.at(-1);

  elements.graphMetric.value = state.graphMetric;
  elements.graphLabel.textContent = metric.label;
  elements.graphScaleLabel.textContent = metric.scaleLabel;
  elements.graph.style.setProperty("--graph-accent", metric.color);

  if (!latestSample) {
    elements.graphCurrentValue.textContent = "--";
    elements.graphEmpty.textContent = "Collecting samples...";
    elements.graphEmpty.hidden = false;
    elements.graph.innerHTML = "";
    return;
  }

  elements.graphCurrentValue.textContent = metric.format(latestSample[state.graphMetric], latestSample);

  if (values.length < 2) {
    elements.graphEmpty.textContent = "Collecting samples...";
    elements.graphEmpty.hidden = false;
    elements.graph.innerHTML = "";
    return;
  }

  const maxValue = metric.maxValue(values);
  const points = graphCoordinates(values, maxValue);
  const baseY = GRAPH_HEIGHT - GRAPH_PADDING.bottom;
  const guideValues = Array.from(
    new Set(
      maxValue === 100
        ? [0, 25, 50, 75, 100]
        : [0, Math.round(maxValue * 0.33), Math.round(maxValue * 0.66), Math.round(maxValue)],
    ),
  );

  // The SVG is rebuilt from the buffered samples on each refresh; this keeps the graph
  // deterministic and avoids incremental DOM drift as points age out of the history window.
  elements.graph.innerHTML = `
    <defs>
      <linearGradient id="graph-fill" x1="0%" y1="0%" x2="0%" y2="100%">
        <stop offset="0%" stop-color="${metric.color}" stop-opacity="0.42" />
        <stop offset="100%" stop-color="${metric.color}" stop-opacity="0.04" />
      </linearGradient>
    </defs>
    ${guideValues
      .map((guide) => {
        const y =
          GRAPH_PADDING.top +
          (GRAPH_HEIGHT - GRAPH_PADDING.top - GRAPH_PADDING.bottom) -
          (guide / maxValue) * (GRAPH_HEIGHT - GRAPH_PADDING.top - GRAPH_PADDING.bottom);

        return `
          <g class="graph-guide">
            <line x1="${GRAPH_PADDING.left}" y1="${y.toFixed(2)}" x2="${GRAPH_WIDTH - GRAPH_PADDING.right}" y2="${y.toFixed(2)}"></line>
            <text x="${GRAPH_WIDTH - GRAPH_PADDING.right}" y="${(y - 6).toFixed(2)}">${guide}${maxValue === 100 ? "%" : ""}</text>
          </g>
        `;
      })
      .join("")}
    <path class="graph-area" d="${areaPath(points)}"></path>
    <path class="graph-line" d="${linePath(points)}"></path>
    <circle class="graph-point" cx="${points.at(-1).x.toFixed(2)}" cy="${points.at(-1).y.toFixed(2)}" r="5"></circle>
    <text class="graph-footer" x="${GRAPH_PADDING.left}" y="${(GRAPH_HEIGHT - 8).toFixed(2)}">Last ${values.length} samples</text>
    <text class="graph-footer graph-footer-end" x="${GRAPH_WIDTH - GRAPH_PADDING.right}" y="${(GRAPH_HEIGHT - 8).toFixed(2)}">${new Date(latestSample.timestamp).toLocaleTimeString()}</text>
    <line class="graph-baseline" x1="${GRAPH_PADDING.left}" y1="${baseY.toFixed(2)}" x2="${GRAPH_WIDTH - GRAPH_PADDING.right}" y2="${baseY.toFixed(2)}"></line>
  `;

  elements.graphEmpty.hidden = true;
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
  elements.graphMetric.value = state.graphMetric;
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

    pushHistory(payload.snapshot);
    renderGraph();
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
elements.graphMetric.addEventListener("change", (event) => {
  state.graphMetric = event.target.value;
  renderGraph();
});

syncControls();
scheduleRefresh();
refresh();
