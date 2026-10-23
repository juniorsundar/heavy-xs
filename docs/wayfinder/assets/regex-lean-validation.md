# Lean regex integration validation

Implements the user-approved [lean contract and bounded community audit](regex-plugin-audit.md). Current user-facing dialect and caveats: [Regex dialect](../../regex.md). This replaces PCRE2, not the Lua helper API; no translator, fork or custom replacement interpreter is shipped.

## Changes and regression checks

- `rust/src/regex.rs` uses fancy-regex 0.19.2 and native `Expander::write_expansion` through a byte-capped writer. Owning userdata/Rc replaces raw pointers; compiled captures borrow the subject, not the cache. Match errors raise; compile failures return nil/message.
- Remove `pcre2-sys`, its compiled/raw binding module and `PCRE2_SYS_STATIC`; preserve vendored-Lua CFLAGS and other native dependencies. Cargo.lock and shipping binary contain no PCRE2 dependency/symbols. Cargo/Nix builds pass.
- Retain nil capture holes, checked byte offsets, iterator lifetime, UTF-8 empty-match progress, ANCHORED and its historical alias. Removed numeric flag bits raise explicit errors. The output cap includes unchanged text, and can reject regex replace-all above 8 MiB even with no matches; the new document-level regression verifies that a failed whole-document operation does not modify text. Several selections are not one transaction; earlier successful edits can precede a later failure.
- Escape literal `[` in bundled JS; three tokenizer examples retain exactly the original token streams. A failed opening rule no longer gets incorrectly re-enabled by its valid closing rule.
- Multi-part regex rules previously bypassed tokenizer compilation caching. A red test observed **45 recompiles across three warmed JS lines**. Cache per opening/closing index now gives zero recompiles. Using the same shipping executable and changing only staged tokenizer Lua reduced the 128-line sample from ~227 ms / 75 ms segment to ~24 ms / 8 ms. The initial CPU-censored runs are superseded, not counted as valid final results.
- The 13 regex Rust tests cover native replacements/Unicode, removed flags, bounded output (including limited no-match), distinct backtrack-limit errors, bad handles/UTF-8 offsets, handle reuse after configuration changes and iterator survival after pattern GC. Together with the existing hit-grid unit test, **14 Rust tests pass**.
- All **16 contract suites pass in debug and release**. Existing startup/restart/quit/SIGINT and 1,239,848-point hit-test checks pass. The current regex regression snapshot deliberately changes diagnostics, removed flags and literal backslashes; historical PCRE2 corpus/probe evidence is not rewritten.
- One final review with the user-requested `opencode-go/deepseek-v4.1-flash` found no code blocker. Its output-cap documentation/test and archived-plan note have been addressed. No unnecessary stage/subproject cleanup or broad `git add -A` was performed.

## Final matched release measurements

[Raw TSV](https://github.com/juniorsundar/heavy-xs/blob/be39cd26/docs/wayfinder/assets/regex-lean-workloads.tsv): each original record is prefixed `reference` or `lean`. Every cohort has one unmeasured warm-up and five correctness-checked actions, all with successful completion markers. Runs alternate reference/lean per workload on the same host, sequentially (not randomized), under the original 30 s watchdog / 20–25 s CPU / 2 GiB address-space envelope. No compilers run concurrently with measurements. Host load/governor, GUI behavior and tail latency remain uncontrolled/unmeasured.

Reference: original base `4c02ffc8e116dc8bfbd3ba14592220a3f1a1a5ac` and its preserved release SHA-256 `4381b80219660a91c68c3f79926b6c2dddc616a41cdecadb54fa61a6127c6273`.

Lean shipping build (no test_hooks), SHA-256 `d0d59c73b3eaa6ee95bfdcb5b1fbf4d3fad12edbac1e554cfb94a7ac8e916d66`. Cargo.lock SHA-256 `a79cd4091669cfc1f896b013d654166e88148bfa9fef7d6761b70e244ed03272`. TSV SHA-256 `4f2140089608763519feda1d62e37feb14e9064b918d9e107494a560420ba75f`. Data changes (including tokenizer caching) are part of the integration comparison, so improvements are not isolated engine-speed claims.

| Workload | PCRE2 median ms | Lean median ms | Change | Lean longest segment ms | Lean VmHWM MiB |
|---|---:|---:|---:|---:|---:|
| Highlight 128 lines | 42.947 | 23.294 | -45.8% | 7.728 | 15.01 |
| Highlight 2,048 lines | 710.099 | 381.071 | -46.3% | 9.435 | 17.51 |
| Highlight 8,192 lines (stress) | 2,875.781 | 1,559.381 | -45.8% | 15.824 | 25.16 |
| Highlight long line (stress) | 45.724 | 26.437 | -42.2% | 11.847 | 15.34 |
| Cached edits (12 edits) | 0.460 | 0.663 | **+44.1%** | 0.278 | 17.68 |
| Search first hit | 0.012 | 0.009 | -25.0% | 0.011 | 15.17 |
| Search miss | 0.603 | 0.697 | +15.6% | 0.742 | 14.97 |
| Eight-file project search | 6.363 | 7.118 | +11.9% | 0.207 | 16.04 |
| Document replacement | 0.529 | 0.551 | +4.2% | 0.589 | 15.19 |
| Synthetic 128 units | 0.061 | 0.155 | **+154.1%** | 0.189 | 14.96 |
| Synthetic 2,048 units | 0.570 | 0.959 | **+68.2%** | 0.992 | 15.07 |
| Synthetic 8,192 units (stress) | 2.304 | 3.821 | **+65.8%** | 3.911 | 16.05 |

All observed ordinary segments fit 33.3 ms and all process peaks fit 64 MiB. Last-minus-first post-cleanup Lua heap deltas are zero at recorded precision; RSS deltas range -40..+8 KiB, within 256 KiB. These are short observations, not leak proofs. None of these fixtures exhausts the default backtrack/output settings.

**Human review: accepted.** The user explicitly accepted the measured cached-edit and three synthetic-transform trade-offs and requested a separate agent handoff to investigate them (never written). Absolute increases are approximately 0.203 ms for the 12-edit batch and 0.094 / 0.389 / 1.517 ms for the transforms. The general 25% review trigger stays unchanged; this approval covers these observations, not arbitrary future regressions. These are native engine/wrapper costs under the lean dialect, not the removed prototype interpreter. The modest sample size and uncontrolled load do not establish precise stable ratios, though the earlier matched collection also showed the same four regressions.

## Not certified

No real-window latency study, every-plugin runtime exercise, exhaustive pattern equivalence or whole-operation deadline guarantee. Third-party saved PCRE2 searches/replacements/snippets can change meaning. Multi-selection replacement is not atomic. Native plugin coverage remains the existing purpose-built fixtures, and the optional SDL renderer configuration was not separately re-run for this regex change. The retired port plan and the spike remain historical records.
