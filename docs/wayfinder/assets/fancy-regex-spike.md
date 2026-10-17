# fancy-regex compatibility and workload spike

Evidence for [Spike fancy-regex against the regex acceptance evidence](../issues/09-spike-fancy-regex.md), not a resolution of [Select the Rust-native regex design](../issues/05-select-rust-native-regex-design.md).

**Finding:** this bounded fancy-regex 0.19.2 adapter does not meet the agreed compatibility target. It also does not establish the required general native work-limit mechanism. No engine, compatibility loss, fallback or default limit is approved by this report. This is not a proof that a more substantial compatibility layer or engine modification could never meet the requirements.

## Reproducible source and provenance

- Base/reference: `4c02ffc8e116dc8bfbd3ba14592220a3f1a1a5ac`.
- Disposable local branch: `spike/fancy-regex`, commit `effe6189bdcfb05318bb1139982d2633edcfa49b`, worktree `/tmp/lxl-spike`. **Never merge; not pushed.**
- [Portable source patch](fancy-regex-spike.patch) includes the complete candidate adapter, replacement interpreter/tests, nil-aware Lua helpers, pinned Cargo.lock, raw-engine example, primary API notes, frozen Lua inputs and `SPIKE.md` instructions. It applies to the base revision in another disposable worktree.
- Reference release binary SHA-256: `4381b80219660a91c68c3f79926b6c2dddc616a41cdecadb54fa61a6127c6273`.
- Candidate release binary SHA-256: `826b85739f179f32798b16f864b1869c80b3f2f699f72a142e3b17b20bcb049c`.
- Candidate Cargo.lock SHA-256: `a79cd4091669cfc1f896b013d654166e88148bfa9fef7d6761b70e244ed03272`.
- [Matched workload TSV](fancy-regex-workloads.tsv) SHA-256: `ddd78b38d505c97f0f3b78be0ab00a8712e936bfc0f041d8651f4a1e02c0da1f`.

Collected 2026-10-17 on the same Linux/i9-12900H machine and Nix toolchain as [the release baseline](regex-release-baseline.md), with the same public C source, deterministic synthetic fixtures and unmodified baseline runtime. Both use normal Cargo release optimizations (`panic=abort`, thin LTO), no extra Cargo features, dummy SDL, staged data and disposable user directories. Main-worktree production/build inputs and existing baselines were not changed. The candidate removes the `pcre2-sys` dependency and compiled binding module; symbol inspection finds no `pcre2_*` functions in its binary. SDL, FreeType and Lua remain native C dependencies.

The adapter keeps Rust ownership of compilation, matching, iteration, substitution and limits. Compiled tables contain validated Rust userdata owning an `Rc`; no raw pointer fallback. Lua helpers preserve the newly agreed nil capture holes. No patterns or bundled syntax definitions were rewritten to make the trial pass. The finite replacement interpreter reparses templates per match and is not a completed/optimized PCRE2 replacement implementation.

Experiment settings, **not product defaults**: 1,000,000 backtracks per capture execution; a separate 1,000,000 native match-call quota per composed operation; 8 MiB substitution output; 2 MiB delegated compilation/DFA size settings. Workloads run sequentially with the baseline's 30-second wall watchdog, 20/25-second CPU limits, 2 GiB address-space cap and core files disabled. No private projects/snippets or unsafe-reference crash/GC probes were used.

## Compatibility results

[Candidate corpus output](fancy-regex-corpus.txt): **155/160 checks pass**—90/94 feature/interface/caller cases, 60/61 replacement cases, and all five iterator-correction checks. Candidate mode correctly replaces the old reference's two iterator-bug observations; it is 160 checks, not the 157-check reference mode.

Every corpus failure is listed below. The three JavaScript failures share the actual bundled start expression printed in [dialect evidence](fancy-regex-dialect.txt), `JS_PATTERN 3:1`: `regex_pattern[1]` in [language_js.lua](../../../data/plugins/language_js.lua#L25). The candidate rejects it at pattern position 106 with `Invalid character class`; the tokenizer then attempts to use the missing compiled pattern and raises `string expected`. The latter is an adapter/caller diagnostic consequence, not a second grammar incompatibility.

Root cause, isolated with the raw engine: an **unescaped `[` inside a character class**. PCRE2 reads `[^\\[\/]` as a class containing a literal `[`; fancy-regex (via regex-syntax) reads it as the start of a nested class and rejects `[^\\[\/]` alone. Escaping it (`[^\\\[\/]`) makes the whole bundled expression compile. That would be a pattern rewrite, which needs explicit approval, and it does not cover third-party syntax plugins that use the PCRE2 reading. Tokens produced by the escaped expression were not measured.

| Failing check | Concrete input / current result | Candidate result / affected consumer |
|---|---|---|
| `UT_default_word_class_ascii_only` | `cmatch("\\w", "é")` returns no values | Returns `1,3`; non-ASCII character-class semantics change for all regex consumers. |
| `TOK_js_regex_literal` | Actual JS start expression on `let r = /ab+c/gi;` yields `keyword:let symbol: r operator: = string: /ab+c/gi normal:;` | Raises after start-expression rejection; bundled JavaScript highlighting fails. |
| `TOK_js_regex_class_and_escaped_slash` | Same rule on `let r = /[a-z]+\//g;` yields `keyword:let symbol: r operator: = string: /[a-z]+\//g normal:;` | Same rejection/raise; regex/class highlighting fails. |
| `TOK_js_number_div_not_comment` | Same syntax on `let n = 1.5/x;` yields `keyword:let symbol: n operator: = number: 1.5 operator:/ symbol:x normal:;` | Same rejection/raise; even number/division tokenization is affected. |
| `ext_mark_named` | `gsub("(*MARK:tag)(a)", "a", "${*MARK}")` returns `"tag",1` | Pattern rejected at position 1; current replacement/PCRE verb surface not preserved. |

The standalone [dialect probe](fancy-regex-probe.lua) has eight comparisons: the [PCRE2 control](fancy-regex-reference-dialect.txt) passes all eight; [the candidate](fancy-regex-dialect.txt) fails seven:

| Expression / subject / option | PCRE2 control | Candidate |
|---|---|---|
| `\w` / `é` | No match | `1,3` |
| `\d` / `٣` | No match | `1,3` |
| `\s` / U+00A0 | No match | `1,3` |
| `\h` / ASCII space | `1,2` | No match (the raw-engine probe also matches hexadecimal `a` instead). |
| `\Qa.b\E` / `a.b` | `1,4` | Invalid-escape compile error for `\Q`. |
| `\Aa` / `a` / `NOTBOL` | `1,2` | No match. |
| `a\z` / `a` / `NOTEOL` | `1,2` | No match. |

The last two are **known adapter flag-mapping gaps**, not claims that the engine lacks absolute anchors. Enabling input assertion overrides preserves the corpus's `^`/NOTBOL check but also suppresses absolute anchors. The frozen raw-engine example shows that disabling overrides does not provide the desired `^`/`$` suppression either. A more complete mapping is unimplemented. `gsub("(?:|a)","a","-") = "---",3` is the eighth comparison and passes both implementations.

Raw-engine probes additionally show that `unicode_mode(false)` fixes the tested ASCII classes but rejects UTF-8-capable `.` and `[^a]` on string input, and loses the tested non-ASCII case fold (`(?i)é` on `É`). Thus simply flipping Unicode off is not a demonstrated solution. Generic byte matching would not establish UTF-8 character semantics. No pattern translator, alternate rule spelling, engine patch or fallback was silently introduced. Unlisted constructs remain unclassified, not authorized exclusions.

## Approved corrections and safety checks

[Seventeen candidate-only checks](fancy-regex-safety.txt) pass: invalid-pattern compile failure pair, ignored invalid flag byte, middle capture holes through `cmatch`/`find_offsets`/`find`/`match`/`gmatch`, trailing omission, iterator retaining its pattern across caller GC, malformed/foreign handle rejection, beyond-end/mid-UTF-8 offsets, EOF offset, mixed empty/nonempty retry, whole-subject iterator lookbehind, and nonempty same-position matching.

These ran only after inspection of the safe userdata-backed adapter. The raw-pointer PCRE2 reference did **not** execute the unsafe handle/lifetime/offset cases. Returning nil as the first capture means Lua generic-for can stop on that result; the test consumes the iterator directly to verify the user-approved nil-slot convention. This consequence must remain visible in the final interface documentation, not be silently changed to a different sentinel.

## Matched workload measurements

The [workload TSV](fancy-regex-workloads.tsv) prefixes every original row with `reference` or `candidate`. Original `SAMPLE` fields after that prefix are repetition, action/segment seconds, resume/yield counts, Lua pre/post/clean KiB, RSS pre/post/clean KiB, VmHWM KiB and output counts, as documented in [the baseline runtime](regex-release-baseline.lua). Cohorts are matched by workload/machine/protocol but collected sequentially, not randomized/interleaved; governor/desktop load are uncontrolled. Five observations support descriptive medians, not tight significance or tail estimates.

| Workload | Reference median ms | Candidate median ms | Change | Candidate largest segment ms | Recorded candidate VmHWM MiB |
|---|---:|---:|---:|---:|---:|
| First-hit search | 0.044 | 0.024 | -45.5% | 0.027 | 14.67 |
| Whole-document miss | 2.101 | 1.787 | -14.9% | 1.835 | 14.52 |
| Eight-file project search | 13.448 | 14.417 | +7.2% | 0.407 | 15.90 |
| Dense document replacement | 1.324 | 1.657 | **+25.2%** | 1.798 | 14.54 |
| Small synthetic transform | 0.186 | 0.526 | **+182.8%** | 0.578 | 14.43 |
| Mid synthetic transform | 1.331 | 3.097 | **+132.7%** | 3.436 | 14.77 |
| Large synthetic transform | 5.034 | 10.385 | **+106.3%** | 10.490 | 15.38 |

Each row has five measured actions per implementation plus unmeasured warm-up. All seven candidate cohorts complete with no correctness failures or limit exhaustion at the experiment settings. Their cleanup Lua and RSS last-minus-first deltas are zero at recorded precision. This is a short steady-state observation, not proof of leak freedom. Process VmHWM includes setup/warm-up and is not a per-call allocation peak; real-window/plugin memory is missing.

Five JavaScript workloads—small/mid/large full highlighting, the long-comment case and cached edits—are **blocked by the rejected syntax expression**, with no valid candidate timing samples. Reference medians are respectively 88.951, 1,432.079, 5,812.586, 91.396 and 0.951 ms. The large reference cohort is CPU-censored after two measured actions; the other four have five. Failed candidate compile/actions are not counted as fast benchmark results.

Fresh-handle compilation is recorded separately in the raw `COMPILE` rows. Successful candidate per-case compilation medians range 10–36 microseconds in these final cohorts. This follows module loading, not process/disk-cold startup; the real caller action still includes compilation where its production path does. No compilation speed claim compensates for a pattern that fails to compile.

The exercised ordinary segment/RSS/retention observations fit the 33.3 ms/64 MiB/256 KiB gates, but **highlighting compatibility/performance is not satisfied or certified**. Replacement and all synthetic transform medians exceed the agreed 25% review trigger; replacement is especially near that threshold and needs repeated comparison before treating the percentage as stable. A trigger calls for human review, not automatic rejection. These are prototype-adapter costs, including the finite template interpreter and nil-aware Lua helpers—not isolated engine costs and not evidence for changing Rust/Lua ownership.

## Resource limits and default options

[Thirteen limit assertions](fancy-regex-limits.txt) pass. These include eight timed VM-exhaustion trials:

| Pattern / subject | Backtrack setting | Observed time to distinct work error |
|---|---|---|
| `(a\|aa)+\1b` / 5,000 `a` characters | 1,000 / 10,000 / 100,000 / 1,000,000 | 1.134 / 1.111 / 4.494 / 30.742 ms |
| `((a\|aa)+)+\1b` / 128 `a` characters | Same settings | 0.212 / 0.577 / 3.528 / 31.575 ms |

The `\|` in this Markdown table escapes the column delimiter; the actual patterns contain alternation `|`, as shown in the executable probe. All eight stop within the accepted one second with `REGEX_WORK_LIMIT`. The output-expansion case (`a` ×4,096, replacement `x` ×1,024, 65,536-byte cap) raises `REGEX_OUTPUT_LIMIT` in 0.716 ms with no replacement returned. A separate 1,000-call quota stops a 5,000-match substitution in 0.379 ms. These latter timings include probe preparation/error checking.

[The budget sweep](fancy-regex-budget-sweep.tsv) runs the same seven reachable workloads at four work settings and a 1 MiB output cap:

- 1,000: fails dense replacement, mid/large transforms through the **adapter's match-call quota**, not native VM backtracking.
- 10,000: fails the large transform through that call quota (its second substitution has 16,384 matches).
- 100,000 and 1,000,000: all seven reachable workloads complete, including five checked repeats each; the output cap is not reached.

Thus 100,000 or 1,000,000 and 1 MiB or the original 8 MiB are **experiment options for the reachable fixtures only**, not chosen defaults or proof that all ordinary/stress workloads fit. The lower native backtracking settings may suffice for these simple matches; the coupled outer call counter is what the failing sweeps measure.

**Unresolved enforcement:** a backtrack counter is not a total instruction/time/output/allocation budget. Its public interface exposes no deadline/cancellation hook or shared operation-wide VM counter; delegated scans and compilation are separate paths. A one-MiB miss still runs successfully with work=1 in the probe. The wrapper counts engine calls, not their internal instruction cost, and substitution can repeat many individually bounded executions. Delegate memory settings are approximate per delegated expression, not a process peak guarantee. The observed small pathological set passing does not prove general stoppage within a second; the external watchdog kills a disposable process, not a running native call safely inside the editor.

Pinned primary references: [builder](https://docs.rs/fancy-regex/0.19.2/fancy_regex/struct.RegexBuilder.html), [options/limit definitions](https://docs.rs/fancy-regex/0.19.2/fancy_regex/struct.RegexOptionsBuilder.html), [input controls](https://docs.rs/fancy-regex/0.19.2/fancy_regex/struct.RegexInput.html), [runtime errors](https://docs.rs/fancy-regex/0.19.2/fancy_regex/enum.RuntimeError.html). The patch includes the focused advisory notes; host experiments, not undocumented API assumptions, establish the reported outcomes.

## Reproduce / hand back to design

Create a detached disposable worktree at the local spike commit, or replay the portable patch on the base revision (never on the production worktree):

```sh
root=$PWD
replay=$(mktemp -d)
rmdir "$replay"
git worktree add --detach "$replay" 4c02ffc8e116dc8bfbd3ba14592220a3f1a1a5ac
git -C "$replay" apply "$root/docs/wayfinder/assets/fancy-regex-spike.patch"
cd "$replay"
# Build/stage and per-process runtime setup are in SPIKE.md.
nix develop --command bash -c 'cargo build --release --manifest-path rust/Cargo.toml && cargo test --release --manifest-path rust/Cargo.toml spike_replacement:: && LITE_RELEASE=1 scripts/install-linux.sh rust/target/release/lite-xl rust/target/stage /usr/local >/dev/null'
```

Run the frozen `spike/*.lua` with the commands/environment in `SPIKE.md` and the baseline report. Corpus mode is **candidate**. Additional `FANCY_CASE=dialect|safety|limits` runs reproduce incompatibilities, candidate safety and bounded resource cases. For the sweep, use `FANCY_CASE=workload`, `FANCY_WORK=1000|10000|100000|1000000`, `FANCY_OUTPUT=1048576`, the selected `BASELINE_CASE` and the unchanged baseline fixture environment. Compare completed reference/candidate cohorts only; retain nonzero exits and missing completion markers. Exact timings will vary.

Validation: release build, eight replacement-interpreter unit tests, 17 candidate safety checks, 13 bounded resource assertions, seven completed workload cohorts, candidate/dialect negative findings, Lua syntax, absence of PCRE2 symbols and source-patch replay check. No old contract baseline was rewritten. The interpreter's additional grammar/Unicode casing limits and untested plugin expressions remain gaps; 60 replacement passes are not exhaustive dialect equivalence.

Return to the human with the compatibility examples, explicit performance review triggers and unresolved work enforcement. The next decision may authorize a specific compatibility-layer investigation, engine changes, further candidate investigation or named losses; none is inferred here, and a custom engine is not automatically the only alternative. The spike ends here rather than growing into production implementation or unrequested optimization.
