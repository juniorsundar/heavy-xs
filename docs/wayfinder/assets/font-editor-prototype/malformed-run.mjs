#!/usr/bin/env node
// PROTOTYPE: build a deterministic corpus of malformed fonts from bundled/public fonts and run
// each through malformed-probe.lua on both backends (one process per case, 20 s wall, 30 s CPU,
// 2 GiB address space). A crash, signal or timeout is recorded as a containment failure, never a pass.
// Usage: malformed-run.mjs <stage prefix> <binary> <out dir> [extra font paths...]
import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";

const [prefix, binary, out, ...extra] = process.argv.slice(2);
if (!prefix || !binary || !out) throw new Error("usage: malformed-run.mjs <prefix> <binary> <out> [fonts...]");
const fontsDir = path.join(prefix, "share/lite-xl/fonts");
const sources = [
  ["jetbrains", path.join(fontsDir, "JetBrainsMono-Regular.ttf")],
  ["fira", path.join(fontsDir, "FiraSans-Regular.ttf")],
  ...extra.map((p, i) => [`extra${i}`, p]),
];
const corpus = path.join(out, "corpus");
fs.mkdirSync(corpus, { recursive: true });

// Deterministic PRNG so the corpus is reproducible.
let seed = 0x9e3779b9;
const rand = () => ((seed = (Math.imul(seed ^ (seed >>> 15), 0x2c1b3c6d) + 0x6d2b79f5) >>> 0) / 2 ** 32);

const be32 = (b, o) => b.readUInt32BE(o);
function tables(b) {
  const n = b.readUInt16BE(4), t = [];
  for (let i = 0; i < n && 12 + 16 * i + 16 <= b.length; i++) {
    const o = 12 + 16 * i;
    t.push({ tag: b.toString("latin1", o, o + 4), dir: o, off: be32(b, o + 8), len: be32(b, o + 12) });
  }
  return t;
}

const cases = [];
const add = (id, buf) => { const p = path.join(corpus, `${id}.ttf`); fs.writeFileSync(p, buf); cases.push([id, p]); };
add("empty", Buffer.alloc(0));
add("zeros-4k", Buffer.alloc(4096));
add("random-64k", Buffer.from(Array.from({ length: 65536 }, () => (rand() * 256) | 0)));
add("text-file", Buffer.from("this is not a font\n".repeat(100)));
for (const [name, src] of sources) {
  const b = fs.readFileSync(src);
  const isTtc = b.toString("latin1", 0, 4) === "ttcf";
  for (const frac of [0.001, 0.05, 0.3, 0.7, 0.99]) add(`${name}-trunc-${frac}`, b.subarray(0, Math.floor(b.length * frac)));
  for (const n of [16, 256, 4096]) {
    const m = Buffer.from(b);
    for (let i = 0; i < n; i++) m[(rand() * m.length) | 0] = (rand() * 256) | 0;
    add(`${name}-flip-${n}`, m);
  }
  if (isTtc) continue;
  for (const t of tables(b)) {
    if (!["head", "hhea", "maxp", "cmap", "loca", "glyf", "hmtx", "CFF ", "CFF2", "OS/2", "post", "fvar", "gvar"].includes(t.tag)) continue;
    const tag = t.tag.trim().replace("/", "");
    const big = Buffer.from(b); big.writeUInt32BE(0x7ffffff0, t.dir + 12); add(`${name}-${tag}-len-huge`, big);
    const off = Buffer.from(b); off.writeUInt32BE(b.length - 2, t.dir + 8); add(`${name}-${tag}-off-end`, off);
    if (t.off + t.len <= b.length && t.len > 0) {
      const z = Buffer.from(b); z.fill(0xff, t.off, t.off + t.len); add(`${name}-${tag}-ff`, z);
    }
  }
}

const results = [];
for (const backend of ["freetype", "candidate"]) {
  for (const [id, p] of cases) {
    const r = spawnSync("/bin/sh", ["-c", 'ulimit -t 30; ulimit -v 2097152; exec "$0"', binary], {
      timeout: 20_000, encoding: "utf8",
      env: { LITE_FONT_BACKEND: backend, LITE_PREFIX: prefix, LITE_XL_RUNTIME: "malformed-probe", MAL_FONT: p, MAL_ID: id,
        SDL_VIDEODRIVER: "offscreen", SDL_FRAMEBUFFER_ACCELERATION: "0", HOME: out, PATH: "/usr/bin:/bin" },
    });
    const line = (r.stdout || "").split("\n").find((l) => l.startsWith("RESULT "));
    let outcome = line ? line.split(" ")[2] : "no-result";
    if (r.error?.code === "ETIMEDOUT" || r.signal === "SIGTERM") outcome = "TIMEOUT";
    else if (r.signal) outcome = `SIGNAL-${r.signal}`;
    else if (r.status !== 0 && outcome !== "lua-error") outcome = `EXIT-${r.status}`;
    const detail = line ? line.split(" ").slice(3).join(" ") : (r.stderr || "").split("\n").filter(Boolean).slice(-1)[0] || "";
    results.push([backend, id, outcome, detail.replace(/\t/g, " ").replace(/\/[^ ]*\//g, "<path>/")]);
  }
}
fs.writeFileSync(path.join(out, "results.tsv"), "backend\tcase\toutcome\tdetail\n" + results.map((r) => r.join("\t")).join("\n") + "\n");
const corpusHash = createHash("sha256");
for (const [id, p] of cases) corpusHash.update(id).update(fs.readFileSync(p));
const summary = {};
for (const [b, , o] of results) summary[`${b} ${o}`] = (summary[`${b} ${o}`] || 0) + 1;
console.log(JSON.stringify({ cases: cases.length, corpus_sha256: corpusHash.digest("hex"), summary }, null, 1));
