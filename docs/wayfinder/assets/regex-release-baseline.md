# Release regex workload baseline

Evidence for [Collect the release regex workload baseline](../issues/08-collect-release-regex-baseline.md), to resume [Define regex performance acceptance criteria](../issues/06-define-regex-performance-criteria.md). **These are observations and options, not accepted limits, an engine comparison, or an ownership decision.**

Assets: [bounded runtime](regex-release-baseline.lua), [raw TSV results](regex-release-baseline.tsv), and the earlier [compatibility corpus](regex-acceptance.md). The runtime contains the complete deterministic fixture definitions, assertions and TSV field descriptions. No production code, private configuration/project, or existing contract baseline changed.

## Protocol and provenance

Collected 2026-10-17, one sequential disposable process per case. The predeclared envelope was <=1 MiB input, <=16,384 lines, <=65,536 bytes per line, <=8 project files; one warm-up and at most five measured actions plus five fresh-handle compilation observations. Each process had a 30-second wall timeout with 2-second kill grace, CPU soft/hard limits of 20/25 seconds, 2 GiB address-space cap; final collection also disabled core files. These are **survey containment**, not product criteria. No adversarial backtracking, unsafe offsets/handles/iterator lifetimes or unbounded iterators were exercised.

A preliminary collection exposed missing probe parameters, which were fixed before final collection; none of its timings are included. Review also prompted an explicit JavaScript syntax/regex-token assertion and precise first-hit search naming. The large-file and long-comment cases were separated before the final run. The published TSV is only the final, line-buffered collection, including its censored process.

| Reference | Recorded value |
|---|---|
| Source revision | `4c02ffc8e116dc8bfbd3ba14592220a3f1a1a5ac` |
| Dirty state | Existing modified `docs/rust-port/PLAN.md`; untracked planning docs, old `subprojects/` and `stage-rs/`. No dirty `data/`, `rust/`, Cargo/Nix configuration, build/staging scripts or old baselines. |
| Build | `nix develop --command bash -c 'CARGO_ARGS=--release scripts/rust-port/verify.sh regex,tokenizer'`; both suites pass, no `RECORD=1` |
| Profile | Cargo default release optimizations, `panic="abort"`, `lto="thin"`, no extra Cargo features; normal software renderer configuration |
| Toolchain | Nix development environment: rustc 1.97.1 (`8bab26f4f`), Cargo 1.97.0 (`c980f4866`) |
| Platform | Linux 6.18.55, x86_64; Intel Core i9-12900H, 20 logical CPUs; 32,522,252 KiB RAM, 35,775,216 KiB swap |
| Scheduling | `powersave` governor, no affinity/frequency lock; unrelated desktop load not measured. No concurrent survey jobs. |
| Runtime | SDL dummy driver; temporary user/home directories; current staged data; no `core.init()`/real-window presentation/private plugins |
| Engine | Cargo lock pins `pcre2-sys` 0.2.10; release build archive contains `10.46 2025-08-27`; Cargo forces static PCRE2 |
| JIT | Native compile code requests JIT and ignores its result; release binary contains `pcre2_jit_compile_8`, `pcre2_jit_match_8`, `pcre2_match_8`. Successful JIT compilation/use is **not verified**. |

SHA-256 identities:

```text
release binary     4381b80219660a91c68c3f79926b6c2dddc616a41cdecadb54fa61a6127c6273
rust/Cargo.lock    af3836640e360db27e063f32ca76018d8cd42114de53845ecd4f1994ea1dfdc6
.cargo/config.toml 3c65cdb601fc68c37d91ca9f7b119e61d33acba519e358317dfeed92e7f5140f
flake.lock         04d898ce5182a5b6fa86f59144e4f731e58b4b9291b3278abdbef916a92f1dfc
runtime asset      94370d2d0963c57b0391239dff77255379863843b7ab5bca3f5e1fa8aa0336e2
raw TSV            df2096eaf790db375ca4b667284d10bfb7ac53e0cd1a9d0c04cd699a7bd908f0
public C source    2e51dcc0182de8406d8ed3d391c0150c952af9cc718113b2e72588fbd4cbcd67
```

The public C source is [contractplug.c](../../../tests/contract/plugin/contractplug.c), 2,300 bytes/76 lines. It is input text only, not compiled/loaded as a native plugin by this survey. The runtime's FNV-1a fixture hashes identify deterministic bytes, not security properties.

## Fixtures and actual paths

Classifications below are proposals for human review, not supported-size restrictions. The larger JS document is a plausible real document, even though this survey calls its scaled input stress.

| Case | Bytes / lines / longest line | Shape and actual measured path | Proposed class |
|---|---|---|---|
| `hl_small` | 3,872 / 128 / 31 | Synthetic JS regex literal, numeric expressions, strings/comments, UTF-8; `Doc` + real `Highlighter` full-cache request | Small ordinary |
| `hl_mid` | 61,952 / 2,048 / 31 | Same deterministic unit, full-cache request | Ordinary document-scale |
| `hl_large` | 247,808 / 8,192 / 31 | Same unit, full-cache request; censored cohort | Scaled/stress for survey, not pathological |
| `hl_long` | 36,641 / 129 / 32,768 | Small JS plus one long `//` comment; comment fast path, not a dense long-line regex workload | Long-line stress |
| `hl_edit` | 61,952 / 2,048 / 31 | Prime full JS cache in setup; batch of 12 `Doc:insert` + invalidation/re-highlighting operations; output 2,060 lines | Ordinary cached editing proxy |
| `search_first` | 89,946 / 2,048 / 44 | One real `core.doc.search.find` regex call, first `café` hit on line 1 | Ordinary first-hit search; **not dense enumeration** |
| `search_miss` | Same fixture | One real `search.find` miss (`日本語`) scans all lines | Ordinary whole-document miss |
| `project_regex` | 448,500 / 14,820 / 84, 8 files | C source repeated by factors 1/16/128/1/16/16/1/16; real `Project:files`, file I/O and `projectsearch.search_regex`; 780 result records | Scaled project search |
| `replace_doc` | 21,180 / 512 / 41 | Real `Doc:replace` with the findreplace `regex.compile(...,"m")` / `gsub` closure; 2,048 replacements; 23,228 output bytes | Ordinary dense replacement |
| `syn_small` | 4,224 / 128 / 32 | Synthetic compiled-method transform flow; 128 primary substitutions, two live outputs totaling 8,704 bytes | Small synthetic transform |
| `syn_mid` | 67,584 / 2,048 / 32 | Same flow, 2,048 primary substitutions; 139,264 output bytes | Scaled synthetic transform |
| `syn_large` | 270,336 / 8,192 / 32 | Same flow, 8,192 primary substitutions; 557,056 output bytes | Larger synthetic transform |

The snippet flow compiles/calls `:gsub`, `:match`, and a second case-insensitive `:gsub`, as in the sampled consumer; it does not execute the installed plugin's parser or claim real snippet-expression coverage. Project search substitutes only a probe-local view-attachment target; real project enumeration/matching/yields run unchanged. No command-palette interaction, real `DocView` event, rendering or presentation is measured. Replacement reproduces the production regex closure through `Doc:replace`, not the full interactive command UI.

## Timing results

`action_s` measures the caller action, excluding fixture generation, document loading/setup, forced-GC snapshots and correctness checks. Highlight full-cache time is bulk work, not first-visible-screen latency. Edit time is the whole 12-edit batch, not one keystroke. Source-derived project files are already present; traversal/read cost is included, but filesystem caches are uncontrolled and warmed by repetition.

**Segment proxy:** for highlighter/project work, the largest actual `coroutine.resume` interval, including the synchronous action/edit prefix before its first resume. Other cases use the whole synchronous action. This is a potential cooperative blocking interval, **not measured real-window UI blocking/input-to-display latency**. The headless driver immediately resumes yielded work; it does not model frame scheduling, input polling or rendering. It also does not isolate the cost of a native match from Lua/editor work.

All values below are milliseconds; warm-up sample 0 is excluded. Five observations support descriptive min/median/max, **not p95/p99 or worst-case guarantees**.

| Case | Measured n | Action min / median / max | Largest segment proxy |
|---|---:|---:|---:|
| `hl_small` | 5 | 89.315 / 89.533 / 89.742 | 29.568 |
| `hl_mid` | 5 | 1,425.906 / 1,426.325 / 1,427.281 | 31.103 |
| `hl_large` | 2, censored cohort | 5,848.761 / 5,881.793 / 5,914.826 | 38.815 |
| `hl_long` | 5 | 88.763 / 89.107 / 89.588 | 29.629 |
| `hl_edit` | 5 | 0.920 / 0.948 / 0.958 | 0.156 |
| `search_first` | 5 | 0.025 / 0.027 / 0.030 | 0.030 |
| `search_miss` | 5 | 1.372 / 1.633 / 1.777 | 1.777 |
| `project_regex` | 5 | 12.973 / 13.019 / 13.070 | 0.415 |
| `replace_doc` | 5 | 1.193 / 1.223 / 1.327 | 1.327 |
| `syn_small` | 5 | 0.175 / 0.194 / 0.202 | 0.202 |
| `syn_mid` | 5 | 1.760 / 1.910 / 1.924 | 1.924 |
| `syn_large` | 5 | 4.919 / 5.035 / 5.082 | 5.082 |

Compilation is separate: five independent fresh compiled handles per case, after modules are loaded. This is **fresh-pattern compilation in an allocator-warm process**, not process/disk-cold startup. First calls range 32–174 microseconds; per-case medians of calls 2–5 range 9–47 microseconds, printed at one-microsecond precision. Exact patterns/flags/times are in TSV. For highlighting, the compile probe uses the actual bundled atomic/possessive JS regex. Warm caller actions can still include compilation when their real path does so (document search, replacement and synthetic transforms); it is not silently subtracted.

### Censoring

Eleven processes finished warm-up plus all five measured actions, with zero correctness failures. `hl_large` recorded a completed warm-up and two checked measured actions, then received SIGXCPU (exit 152) under the **aggregate process** CPU budget; it has no completion marker. All 60 compile observations and 57 measured actions are present. The censor does not prove one match/action lasted 20 seconds, nor that the candidate must reject this document size. No partially completed action is included in the table. It is not an implementation-test failure: the experiment deliberately stopped a longer cohort and reports its incomplete sample count.

The TSV preserves the timeout utility's “dumped core” status text for SIGXCPU; final runs had `RLIMIT_CORE=0`. This is a process-containment event, not safe in-process cancellation or evidence of the editor's own work-limit mechanism.

## Memory and retention

Snapshots run two full Lua collections. `pre` is before setup; `post` retains the live document/undo or result strings/records; `clean` is after releasing per-action objects and threads. Base fixtures, loaded modules and warmed syntax caches intentionally remain live. VmHWM is the Linux-reported **whole-process** RSS high-water mark across setup/warm-up/actions/cleanup, not a per-action allocation peak; reading it is not exhaustive allocation accounting. VmRSS is a snapshot, not allocated-byte accounting. Final censored-work allocations may be absent from its last recorded snapshot.

| Case | Maximum recorded process VmHWM (MiB) | Maximum post-GC live Lua heap (KiB) | Last-minus-first measured cleanup RSS (KiB) |
|---|---:|---:|---:|
| `hl_small` | 14.29 | 532.88 | +4 |
| `hl_mid` | 17.49 | 1,583.02 | 0 |
| `hl_large` | 27.62 | 4,943.04 | 0 (two observations only) |
| `hl_long` | 14.54 | 601.18 | +36 |
| `hl_edit` | 17.33 | 1,654.78 | -4 |
| `search_first` | 13.84 | 661.12 | 0 |
| `search_miss` | 13.66 | 660.99 | 0 |
| `project_regex` | 14.87 | 657.98 | 0 |
| `replace_doc` | 13.54 | 547.63 | -84 |
| `syn_small` | 13.27 | 471.02 | 0 |
| `syn_mid` | 13.59 | 660.37 | 0 |
| `syn_large` | 14.58 | 1,266.39 | 0 |

Last-minus-first measured cleanup Lua heap is 0.00 KiB at the TSV's two-decimal precision for every case. This short steady-state check shows no visible continuing Lua growth under this cleanup policy; **it is not proof of no leaks**, release to the OS, bounded arbitrary-pattern caches or long-session memory stability. RSS deltas include allocator/page effects. Live output/undo sizes explain post-action growth; native allocations are included in process measurements but not separately attributed. Real-window/plugin memory is missing.

## Concrete choices for the human — none selected

1. **Workload boundary:** include small/full 2,048-line highlighting, cached edits, UTF-8 miss, eight-file project scan, replacement and small/mid synthetic transforms as ordinary comparisons; treat the 8,192-line full pass and 32 KiB comment as stress observations. Alternatively, count 8,192 lines as ordinary: its multi-second full-cache work then needs explicit bulk-progress expectations, not a claim that a six-second action fits a frame. The large transform is cheap here but only synthetic.
2. **Interactive blocking target:** the existing 60 FPS target suggests concrete 8.3/16.7/33.3 ms half-/one-/two-frame options. The observed 29–39 ms highlighter segments would miss the first two; the 33.3 ms option would still miss the largest observed segment. This reference is not an acceptable-worst-case oracle. Decide whether proxy budgets suffice or require real-window timing first. Bulk completion remains separate (e.g. the mid highlight is about 1.43 s despite yielding).
3. **Isolated-process memory gate:** 48 or 64 MiB RSS are concrete headroom options above the largest *recorded* 27.62 MiB in this headless envelope; neither is justified as a whole-editor ceiling without UI/plugin evidence. Lua live high-water here is about 4.83 MiB; this does not yield a native per-match heap/work quota. A retention gate could require plateau/no sustained trend with an explicitly tolerated measurement band, or require longer-session evidence before assigning a numeric band.
4. **Regression review:** absolute agreed limits stay primary. One option is explicit review above a 25% bulk slowdown after paired, same-machine repeated runs, while not rejecting a smaller slowdown within absolute limits; another is requiring review of any repeatable slowdown outside measured noise. This is a choice to make, not an approved percentage. Five reference samples alone cannot calibrate a tight tail/noise allowance.
5. **Enforceable pathological-work protection:** the OS caps here protect the survey only. In-process work/depth/output/heap mechanisms, their numeric budgets, exhaustion semantics and proof they stop native work remain undecided. No reference observation licenses no-match/partial-success on exhaustion.

No allocation/crossing profiler was added because this task compares no placement/candidate and has no disputed cost to attribute.

## Reproduction

Run from the repository root, with the Nix development environment available. This stages a release binary/data and rechecks the old suites without recording. Output directories contain generated public/synthetic fixtures only.

```sh
nix develop --command bash -c 'CARGO_ARGS=--release scripts/rust-port/verify.sh regex,tokenizer'
root=$PWD
out=$(mktemp -d)
for case in hl_small hl_mid hl_large hl_long hl_edit search_first search_miss \
            project_regex replace_doc syn_small syn_mid syn_large; do
  dir="$out/$case"
  mkdir "$dir"
  cp docs/wayfinder/assets/regex-release-baseline.lua "$dir/"
  SDL_VIDEODRIVER=dummy \
  DBUS_SESSION_BUS_ADDRESS=unix:path=/nonexistent/lxl-contract-bus \
  SDL_FILE_DIALOG_DRIVER= LITE_USERDIR="$dir" LITE_OS_HOME="$dir" \
  LITE_PREFIX="$root/rust/target/stage/usr/local" \
  LITE_XL_RUNTIME=regex-release-baseline BASELINE_ROOT="$root" \
  BASELINE_DIR="$dir" BASELINE_CASE="$case" \
    prlimit --core=0:0 --as=2147483648 --cpu=20:25 -- \
    timeout -k 2 30 "$root/rust/target/release/lite-xl" >"$out/$case.tsv" 2>&1 \
    && rc=0 || rc=$?
  printf 'PROCESS\t%s\texit\t%s\n' "$case" "$rc" >>"$out/$case.tsv"
done
printf 'Results: %s\n' "$out"
```

Inspect every exit/completion record; nonzero exits are not automatically acceptable censoring. On this machine the large cohort had exit 152; on a faster machine it may finish. `SAMPLE` columns are: tag, case, repetition, action seconds, max segment seconds, resumes, yields, Lua pre/post/clean KiB, RSS pre/post/clean KiB, VmHWM KiB, primary matches/count, output measure. Output is bytes for replacement/transforms, line count for highlighting, and zero where result records/spans—not text—are returned. `META`/`FIXTURE` include fixture dimensions/hashes. Medians use sorted non-warm-up action times (average the two central values for the two-sample censored case); memory tables take recorded maxima including warm-up. Cleanup deltas compare the last with first measured snapshot, not initial process startup.

Validation includes Lua syntax, release regex/tokenizer regressions, and disposable negative controls: changing the highlight suffix to `.txt` is rejected by the syntax assertion; changing the expected replacement count to 2,049 produces six failures and exit 1. Neither mutation is in the asset. Independent review of the runtime drove the first-hit/JavaScript-evidence corrections. Every timing, segment, memory and retention value in this report was then recomputed directly from the raw TSV columns and matches.
