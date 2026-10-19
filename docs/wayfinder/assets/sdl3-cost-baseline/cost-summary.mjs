#!/usr/bin/env node
// PROTOTYPE: summarise a cost-run.sh output directory into Markdown tables.
// Usage: node cost-summary.mjs OUTDIR
// Per (config, workload): per-round A and B statistics, then the median across rounds, and the
// A-vs-B difference, which measures run-to-run noise when A and B are the same build.
import fs from "node:fs";
import path from "node:path";

const dir = process.argv[2];
if (!dir) { console.error("usage: cost-summary.mjs OUTDIR"); process.exit(2); }
const files = fs.readdirSync(dir).filter(f => /\.tsv$/.test(f));

const median = xs => { const s = [...xs].sort((a, b) => a - b); const n = s.length; return n ? (n % 2 ? s[(n - 1) / 2] : (s[n / 2 - 1] + s[n / 2]) / 2) : NaN; };
const pct = (xs, p) => { const s = [...xs].sort((a, b) => a - b); return s.length ? s[Math.min(s.length - 1, Math.ceil(p / 100 * s.length) - 1)] : NaN; };
const read = f => fs.readFileSync(path.join(dir, f), "utf8").trim().split("\n").map(l => l.split("\t"));
const parse = f => { const m = f.match(/^(\w+)-(\w+)-([AB])-r(\d+)-(\d+)\.tsv$/); return m && { mode: m[1], cfg: m[2], label: m[3], round: +m[4], idx: +m[5], f }; };
const runs = files.map(parse).filter(Boolean);
const meta = f => { try { return fs.readFileSync(path.join(dir, f + ".meta"), "utf8").trim(); } catch { return ""; } };
const fmt = (x, d = 2) => Number.isFinite(x) ? x.toFixed(d) : "–";
const out = [];

// Rows: [name, fn(run rows) -> number]
function table(title, mode, metrics, perRun) {
  out.push(`\n### ${title}\n`);
  for (const cfg of ["bundled", "user"]) {
    const rs = runs.filter(r => r.mode === mode && r.cfg === cfg);
    if (!rs.length) continue;
    out.push(`**${cfg} config**\n`);
    out.push(`| metric | A median | B median | B − A | per-round A | per-round B |`);
    out.push(`|---|---|---|---|---|---|`);
    const rounds = [...new Set(rs.map(r => r.round))].sort();
    for (const [name, fn, d] of metrics) {
      const per = label => rounds.map(round => perRun(rs.filter(r => r.label === label && r.round === round), fn));
      const a = per("A"), b = per("B");
      const ma = median(a.filter(Number.isFinite)), mb = median(b.filter(Number.isFinite));
      out.push(`| ${name} | ${fmt(ma, d)} | ${fmt(mb, d)} | ${fmt(mb - ma, d)} | ${a.map(x => fmt(x, d)).join(" / ")} | ${b.map(x => fmt(x, d)).join(" / ")} |`);
    }
    out.push("");
  }
}

// Startup: discard the first launch of each A or B group (index 0 of the first group in a round).
table("Startup (warm cache, launch to first presented frame)", "startup", [
  ["median startup ms", rows => rows.map(r => +r[0]), 1],
  ["median RSS at first frame (MiB)", rows => rows.map(r => +r[1] / 1024), 1],
], (rs, fn) => {
  const kept = rs.filter(r => r.idx !== 0);
  return median(kept.flatMap(r => fn(read(r.f).slice(1))));
});

// Frames: per run, p95 and median of active and presentation for each phase.
const phaseMetrics = [];
for (const phase of ["scroll", "scroll_page", "type"]) {
  for (const [col, label] of [[1, "active"], [2, "present"], [3, "total"]]) {
    phaseMetrics.push([`${phase} p95 ${label} ms`, rows => rows.filter(r => r[0] === phase).map(r => +r[col]), 2, "p95"]);
    if (label !== "total") phaseMetrics.push([`${phase} median ${label} ms`, rows => rows.filter(r => r[0] === phase).map(r => +r[col]), 2, "median"]);
  }
}
out.push("\n### Frames (fullscreen, per drawn frame; each round's value is the median of its two runs)\n");
for (const cfg of ["bundled", "user"]) {
  const rs = runs.filter(r => r.mode === "frames" && r.cfg === cfg);
  if (!rs.length) continue;
  out.push(`**${cfg} config**\n`);
  out.push(`| metric | A median | B median | B − A | per-round A | per-round B |`);
  out.push(`|---|---|---|---|---|---|`);
  const rounds = [...new Set(rs.map(r => r.round))].sort();
  for (const [name, fn, d, stat] of phaseMetrics) {
    const per = label => rounds.map(round => median(rs.filter(r => r.label === label && r.round === round)
      .map(r => { const v = fn(read(r.f).slice(1)); return stat === "p95" ? pct(v, 95) : median(v); })));
    const a = per("A"), b = per("B");
    const ma = median(a), mb = median(b);
    out.push(`| ${name} | ${fmt(ma, d)} | ${fmt(mb, d)} | ${fmt(mb - ma, d)} | ${a.map(x => fmt(x, d)).join(" / ")} | ${b.map(x => fmt(x, d)).join(" / ")} |`);
  }
  out.push("");
}

const idleMetrics = [
  ["CPU %", rows => +rows[0][2], 2],
  ["drawn frames / s", rows => +rows[0][3] / +rows[0][0], 2],
  ["wait_event calls / s", rows => +rows[0][4] / +rows[0][0], 0],
  ["sub-ms waits / s (non-blocking polls)", rows => +rows[0][6] / +rows[0][0], 0],
  ["voluntary context switches / s", rows => +rows[0][8] / +rows[0][0], 1],
  ["involuntary context switches / s", rows => +rows[0][9] / +rows[0][0], 2],
];
const runMedian = (rs, fn) => median(rs.map(r => fn(read(r.f).slice(1))).filter(Number.isFinite));
table("Idle, focused (caret blinking), 20 s", "idle_focused", idleMetrics, runMedian);
table("Idle, unfocused (minimised), 20 s, sampled from outside the process", "idle_unfocused",
  idleMetrics.filter(([n]) => /CPU|context/.test(n)), runMedian);

// Memory: absolute size, and growth over the last 30 cycles (the accumulation signal).
const memMetrics = [
  ["RSS after first cycle (MiB)", rows => +rows.find(r => r[0] === "1")[1] / 1024, 1],
  ["RSS at end (MiB)", rows => +rows[rows.length - 1][1] / 1024, 1],
  ["peak RSS (MiB)", rows => +rows[rows.length - 1][2] / 1024, 1],
  // Growth over the last k = min(30, cycles-1) cycles; per cycle is the leak signal.
  ["RSS growth, last 30 cycles (KiB)", rows => { const n = +rows[rows.length - 1][0]; const k = Math.min(30, n - 1); return +rows[rows.length - 1][1] - +rows.find(r => +r[0] === n - k)[1]; }, 0],
  ["RSS growth per cycle, last 30 (KiB)", rows => { const n = +rows[rows.length - 1][0]; const k = Math.min(30, n - 1); return (+rows[rows.length - 1][1] - +rows.find(r => +r[0] === n - k)[1]) / k; }, 2],
  ["Lua heap growth, last 30 cycles (KiB)", rows => { const n = +rows[rows.length - 1][0]; const k = Math.min(30, n - 1); return +rows[rows.length - 1][3] - +rows.find(r => +r[0] === n - k)[3]; }, 1],
];
table("Memory (50 cycles: second window open/draw/resize/close, main window normal ↔ fullscreen)", "memory", memMetrics, runMedian);

// Environment and anomalies.
const windows = {};
for (const r of runs) { const m = meta(r.f).match(/window=(\S+)/); if (m) { const k = `${r.mode} ${m[1]}`; windows[k] = (windows[k] || 0) + 1; } }
out.push("\n### Window sizes seen\n");
for (const [k, n] of Object.entries(windows).sort()) out.push(`- ${k}: ${n} runs`);
const failures = fs.existsSync(path.join(dir, "failures.txt")) ? fs.readFileSync(path.join(dir, "failures.txt"), "utf8").trim() : "";
out.push(`\n### Failures\n\n${failures ? "```\n" + failures + "\n```" : "None."}`);
const focus = runs.filter(r => r.mode === "idle_focused").map(r => read(r.f)[1]).filter(row => row[10] !== "true" || row[11] !== "true").length;
out.push(`\nFocused-idle runs that lost focus: ${focus}.`);
console.log(out.join("\n"));
