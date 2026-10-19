# Collect the release regex workload baseline

Parent: ../map.md
Labels: wayfinder:task
Type: task
Mode: AFK
Status: resolved
Assignee: pi-regex-release-baseline
Blocked by: 07

## Question

What bounded, reproducible measurements of the current release editor will let the human choose concrete workload sizes and responsiveness/memory budgets?

Unblock [Define regex performance acceptance criteria](06-define-regex-performance-criteria.md), using [Assemble the regex compatibility acceptance corpus](07-assemble-regex-compatibility-corpus.md) and the constraints recorded in the performance ticket's discussion.

Collect reference evidence only, not a candidate-engine benchmark campaign, production optimization or acceptance decision. Use a small set of shared fixtures and existing runtime/test facilities; no new permanent benchmark framework is presumed.

### Reference and coverage

- Use the current Cargo-built PCRE2 editor in release mode. Record revision, relevant dirty source state, build settings, machine, configuration, fixture identities/sizes and exact reproduction commands. Distinguish requested JIT from verified execution mode rather than assuming compilation enabled it successfully.
- Reuse the compatibility corpus and exercise real caller paths for highlighting/editing, document search, project search, replacement and representative snippet transformations. Keep synthetic plugin cases distinct from executed installed-plugin cases. Do not inspect or publish private projects/configuration to manufacture a representative workload.
- Present a small range of actual source-derived and deterministically scaled inputs. Record byte/line counts, longest line, relevant pattern features and match/output sizes. Include regex-heavy syntax, UTF-8, misses, dense matches and long lines. Propose ordinary versus stress classifications for human review; fixture sizes are observations, not approved supported-size limits.
- Separate cold pattern compilation and warmed-up execution, end-to-end action time, observed main-thread blocking and bulk completion time. Do not substitute isolated matching microbenchmarks for caller-path evidence.

### Measurements and interpretation

- Use repeated bounded observations and report sample counts, variation, observed maxima and latency distributions only where sampling supports them. Timing proxies must be named: an offscreen runtime or call duration does not establish real-window input-to-display latency. Record gaps rather than claiming an unmeasured UI guarantee.
- Measure process peak RSS and Lua heap, plus retained growth after repeated operations and cleanup. Include native allocations through process measurements; Lua heap alone is insufficient. Record GC/cleanup policy, cache warm-up and expected live input/output growth. Distinguish measurement/sampling limits from true peaks or proof of a leak.
- Use allocation/GC or Rust–Lua crossing profiling only where needed to explain a disputed cost. Do not turn this bounded survey into broad instrumentation work.
- No candidate comparison or proof of pathological-input safety is produced here. A measured maximum is not a mathematical bound. Do not infer future engine budgets or interruptibility from the reference's behavior.

### Containment and deliverable

Choose and document a finite, conservative survey envelope before running probes. Run suspect/long cases in disposable processes with external watchdog/resource containment, and record censored/time-limited results honestly. These are safeguards for collecting evidence, not evidence that a caller timeout interrupts native regex work. Do not run an unbounded iterator, deliberately execute known-unsafe offset/handle/lifetime paths, grow inputs until the host runs out of memory, or change user documents/settings. Skip unsafe cases and explain the limitation.

Return a reproducible report under `docs/wayfinder/assets/`, with minimal fixture/runtime assets only where necessary. Include a compact results table and concrete options the human can use to choose workload sizes, latency/memory budgets and acceptable regressions. Do not choose those product criteria, fabricate a missing measurement, rewrite production files, or replace existing contract baselines. If measurement needs external library knowledge or an unresolved semantic choice, surface a focused research/HITL blocker.

## Comments

### Survey envelope — before execution

Collect sequential disposable-process observations only. Use repository-public C source plus synthetic JavaScript/UTF-8 inputs derived from the acceptance examples; never private snippets/projects. Cap each fixture at 1 MiB, 16,384 lines and 65,536 bytes per line, with at most eight project files. Use one unmeasured warm-up and five measured actions per workload, and at most five cold compile observations; repeat cleanup checks for those same actions rather than unbounded growth tests. Cover small/scaled/stress cases, dense and missing matches, long lines, incremental highlighting edits, real document/project matching and replacement paths, and explicitly synthetic compiled-method transforms.

Each editor process has a 30-second external timeout (2-second kill grace), 20-second CPU soft/25-second hard limit and 2 GiB address-space cap. These are conservative experiment containment values, not supported-size limits or product acceptance budgets. Skip unsafe/pathological probes; record timeout/limit failures as censored, not successful measurements. Timing is headless synchronous-action time or time between actual coroutine yields; neither proves input-to-display latency or native interruptibility. Read process VmHWM/VmRSS and Lua heap with explicit full-GC snapshots. Do not infer process release from Lua collection alone.

### Resolution

Collected the bounded release reference survey: [report](../assets/regex-release-baseline.md), [runtime](https://github.com/juniorsundar/heavy-xs/blob/be39cd26/docs/wayfinder/assets/regex-release-baseline.lua) and [raw TSV](https://github.com/juniorsundar/heavy-xs/blob/be39cd26/docs/wayfinder/assets/regex-release-baseline.tsv). The report records revision, clean build inputs, release profile, toolchain, machine, linked PCRE2 10.46, fixture dimensions/hashes and exact reproduction commands. JIT is requested and compiled in, but its execution is not verified.

Twelve real or explicitly synthetic caller-path cases ran within the predeclared envelope: JavaScript highlighting (full-cache, long line, cached edits), first-hit and miss document search, eight-file project search, `Doc:replace` with the find-replace regex closure, and synthetic snippet-method transforms. Eleven completed five checked measured actions with zero failures. The 8,192-line full highlight is censored after two measured actions by the survey's aggregate CPU containment; that is not a single-call bound or product failure.

Key observations, all headless proxies: full-cache highlighting takes about 89 ms (128 lines), 1.43 s (2,048) and 5.9 s (8,192) of bulk work, with largest cooperative segments of 29–39 ms. Other actions stay under about 13 ms. Recorded process VmHWM stays at or below 27.62 MiB, and cleanup Lua heap shows no measured growth across repetitions. None of this measures real-window input-to-display latency, a worst case, native interruptibility or the absence of leaks.

Validation: release regex/tokenizer suites pass; runtime syntax check; negative controls reject non-JavaScript syntax and a wrong replacement count; report statistics were independently recomputed from raw columns. No production code, build input or old baseline changed.

The report offers concrete, unselected options for workload boundaries, cooperative blocking targets, memory gates, regression review and enforceable pathological-work limits. This evidence unblocks [Define regex performance acceptance criteria](06-define-regex-performance-criteria.md) for a HITL choice; it chooses no budget, engine or ownership.
