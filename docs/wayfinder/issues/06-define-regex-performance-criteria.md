# Define regex performance acceptance criteria

Parent: ../map.md
Labels: wayfinder:grilling
Type: grilling
Mode: HITL
Status: resolved
Assignee: pi-regex-performance-criteria
Blocked by: 02, 03, 08

## Question

Which representative workloads, input sizes, latency and memory criteria, and comparison baseline should decide disputed regex placement and engine-performance claims?

Apply [Define the Rust–Lua ownership rule](02-define-rust-lua-rule.md) to [Choose the regex compatibility envelope](03-choose-regex-compatibility.md). Use the source-cited inventory to choose realistic coverage: interactive highlighting/search, project search and replacement, and relevant sampled plugin behavior. Do not assume every category needs a separate benchmark or a new permanent harness.

Agree on what to measure and what counts as acceptable: interactive stalls/response latency and memory first, with allocation/GC and Rust–Lua crossing measurements where they explain a disputed cost. Distinguish absolute safety/work limits from comparative performance goals, and ordinary workloads from pathological expressions or unusually large inputs. Do not assume a caller's timeout can interrupt an executing native operation.

Choose a reproducible baseline and a comparison method appropriate to those criteria. Do not invent results, hard-code arbitrary budgets, or let microbenchmarks alone decide placement. If facts needed to choose a criterion are missing, create a focused evidence task rather than asking the user to guess them.

This ticket agrees acceptance criteria; it does not execute a benchmark campaign, choose an engine, or assign individual operations to Rust or Lua.

## Comments

### Discussion in progress

The user accepted these first-round constraints:

- Compare the current Cargo-built PCRE2 editor and candidates in release mode on the same machine, fixtures and configuration. Record revision/build settings and separate cold compilation from warm matching. Where attribution matters, hold the engine constant for ownership comparisons or composition constant for engine comparisons. Approved correctness corrections use their intended outcomes, not an old-bug oracle.
- Use a small representative set of real caller paths: interactive highlighting/editing and document search, project search, replacement and snippet transforms. Include regex-heavy syntax, UTF-8, misses, dense matches and long lines; distinguish ordinary from stress cases. Shared fixtures are preferred over a new permanent framework. Bulk completion need not fit a frame, but UI-blocking intervals and memory still matter.
- Judge absolute user-facing responsiveness/memory limits ahead of percentage slowdowns alone. Measurable slowdowns may be acceptable within agreed limits, with substantial bulk-throughput regressions reviewed explicitly. Calibrate concrete limits from evidence rather than inventing numbers. Lua customization remains preferred when both designs satisfy the criteria.
- Require enforceable resource limits and explicit safe failure for pathological/oversized work even if the reference stalls. Verify that limits stop the work; coroutine yields and caller-side timeouts alone do not prove native interruption. Exhaustion must not appear as no match or successful partial replacement.

The user then accepted the measurement scope: end-to-end actions, longest observed blocking intervals and latency distributions where sampling supports them; bulk completion time separately; process peak RSS plus Lua heap; and retained growth after repeated operations and cleanup. Record input/output sizes so legitimate live-result growth is not confused with a leak. Profile allocations/GC and Rust–Lua crossings only when they explain a disputed cost. Observed maxima are not mathematical worst-case guarantees.

### Waiting for baseline evidence — not resolved

The user explicitly chose to calibrate before resolving this ticket rather than approve invented thresholds or split off a separate budget decision. [Assemble the regex compatibility acceptance corpus](07-assemble-regex-compatibility-corpus.md) precedes [Collect the release regex workload baseline](08-collect-release-regex-baseline.md), a bounded reference survey rather than a candidate benchmark campaign.

The claim is released and the baseline task is a blocker so another session follows the evidence frontier rather than skips an indefinitely claimed decision. Resume this HITL discussion after the evidence is available; bring concrete workload sizes, latency/memory budgets and acceptable regression options to the user. Do not resolve this ticket or unblock ownership merely because measurement rules are agreed. Absolute budgets and practical resource-limit/failure criteria remain to be made concrete; no numeric values, benchmark results, engine choice or operation placement are claimed.

### Resolution

The user accepted all six recommendations after reviewing [the release baseline](../assets/regex-release-baseline.md). Measurement method is as recorded above; criteria:

1. **Workload classes.** Ordinary: highlighting up to 2,048 lines, cached edits, whole-document UTF-8 search miss, the eight-file (~450 KB) project search, 512-line dense replacement and synthetic transforms up to 2,048 units. Stress: 8,192-line full highlight, 32 KiB single line and 8,192-unit transform. These are survey classes, not supported-size limits.
2. **Blocking.** Ordinary workloads must keep the longest observed cooperative segment at or below 33.3 ms (two frames at 60 FPS); the reference meets this at 29–31 ms. Stress workloads have no absolute segment limit but regressions are reviewed. One-frame/16.7 ms is not required: meeting it needs highlighter scheduling redesign, outside this effort.
3. **Measurement gate.** Same-machine, identically run headless proxies are sufficient to accept or reject a candidate. Real-window input-to-display latency is a recorded, unmeasured limitation, not a requirement.
4. **Regressions.** Bulk completion has no absolute limit. A median slowdown above 25% on any workload, from paired same-machine runs with at least five measured actions each, requires explicit user review (not automatic rejection). Smaller slowdowns within absolute limits are acceptable.
5. **Memory.** Each workload process stays at or below 64 MiB VmHWM (reference 27.62 MiB). Retention: across the five measured repetitions, post-cleanup Lua heap shows no sustained increase, and post-cleanup RSS grows by no more than 256 KiB from first to last.
6. **Pathological work.** Every native match, iteration and substitution has a native work limit, configurable from Lua. Exhaustion raises a distinct error, never no match or partial replacement. No ordinary or stress workload reaches the default limit. Each case of a pathological set, to be assembled during design, stops within 1 second on the reference machine. Substitution output also has a byte limit.

The enforcement mechanism, default numeric work/output limits and the pathological set belong to [Assign ownership of regex execution and composition](04-assign-regex-operation-ownership.md) and [Select the Rust-native regex design](05-select-rust-native-regex-design.md). No engine or operation placement is chosen here.

### Later lean decision and measured review

The user subsequently dropped aggregate-work/deadline enforcement for the [selected lean contract](../assets/regex-plugin-audit.md): native per-execution backtrack limit plus capped output, with no total-operation time guarantee. The other measurement/segment/memory criteria remain. The user explicitly accepted the four >25% relative regressions in [the final paired measurements](../assets/regex-lean-validation.md) and requested a [separate slowdown handoff](../../regex-slowdown-handoff.md). That approval does not waive review of future regressions.
