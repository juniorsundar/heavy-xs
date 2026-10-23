# Survey Rust-native font backends

Parent: ../freetype-map.md
Labels: wayfinder:research
Type: research
Mode: AFK
Status: resolved
Assignee: pi-freetype-research
Blocked by: none

## Question

Which maintained Rust-native libraries or small compositions can replace the editor's current FreeType responsibilities, and where do their verified capabilities or dependencies fall short?

Read the research skill and investigate primary documentation/source. Consider plausible candidates such as skrifa-based stacks, swash and fontdue, but verify actual current APIs, releases and capabilities rather than treating these names as a shortlist or a backend choice. Use the local renderer and API as context while the full inventory proceeds independently.

Compare font parsing/rasterization scope; TrueType/OpenType outlines, collections and other relevant custom-font formats; grayscale/mono/LCD output and filtering; hinting; synthetic styles; metrics and fractional positioning; variable/bitmap/color fonts where relevant; missing-glyph/fallback integration; invalid/untrusted font handling; cache ownership; Rust/native transitive dependencies; licence and maintenance. Mark absent, unsupported, uncertain and unnecessary-for-current-editor features distinctly. Identify which responsibility requires another crate, rather than scoring font parsers alone as complete rasterizers.

Return a source-cited comparison, concrete unknowns, likely integration seams and proposed narrow experiments. Verify native dependency claims end-to-end. Do not infer visual equivalence, benchmark results or approved feature losses from API support. No engine selection, prototype implementation, fork, production patch or C fallback.

Capture one Markdown research report on an isolated throwaway `research/freetype-backends` branch/worktree; push that branch to the existing origin and open no PR. Never merge it. Keep only public research/source references on that branch, preserve the main working tree and link the actual branch/report on resolution. If pushing or primary-source access fails, retain the report locally and state the blocker rather than claiming publication.

Context: [Inventory the observable font contract](12-inventory-font-contract.md) proceeds independently. The survey informs [Define font compatibility and acceptance criteria](15-define-font-acceptance.md) and [Select the Rust-native font backend](16-select-rust-font-backend.md).

## Comments

### Research dispatch

Claimed for isolated primary-source research. Intended context pointer: branch `research/freetype-backends`, report `docs/wayfinder/assets/freetype-backend-survey.md`. These are dispatch destinations, not a claim that findings already exist. The charting session owns recording the actual result or releasing the claim if dispatch returns no usable evidence.

The delegation returned no findings and produced no report. The claim is released so another session can take the ticket. No completed survey or successful publication is claimed.

### Research retry

Credentials were refreshed and a reviewer call succeeded. This working session reclaims the independent survey and retries it in isolation; only verified report/branch results will be recorded as research evidence.

The retry returned only a partial source-cited report: it could not check the local renderer, and its local-contract cross-check, complete transitive audit and several capability/version claims still need validation. No survey is claimed. The claim is released; the survey remains open, independently of the resolved inventory.

### Verification and publication retry

Claimed by `pi-freetype-research` to verify the recovered report against primary sources and the now-validated local contract, audit native transitive dependencies, and publish only the public research report on the isolated research branch. No backend selection or production edits are authorized.

### Resolution

Published the verified [Rust-native font backend survey](https://github.com/juniorsundar/lite-xl/blob/b8b9f40b3db851bdb26f3d135a94e23cc3616ef2/docs/wayfinder/assets/freetype-backend-survey.md). It is a public-source Markdown report only; nothing was merged.

Compared released Swash, Skrifa + Zeno, Fontdue and ab_glyph against the validated editor contract. Stock Swash fixes a hint target and AutoFallback policy rather than exposing the editor's forced-auto/slight/full/mono controls; direct Skrifa exposes the distinctions but needs a separate rasterizer and adapter. Fontdue does provide RGB subpixel rasterization, correcting the earlier draft. Mono rasterization, LCD filtering, synthetic-style equivalence, format behavior and metric/appearance trade-offs require candidate evidence, not conclusions from API names.

The complete Linux/default-feature audit covers 33 external packages, including build/proc-macro/math and Yazi decompression dependencies. No native font library/toolchain dependency was found in that resolved graph; the report records exact versions, licences, feature limitations and the full audit lockfile. This is not an all-feature/all-target, whole-editor C-free or hostile-font safety guarantee. The stock Swash Skrifa cap is real and resolves to 0.44 alongside the separate 0.48 candidate, not one interchangeable API version.

A focused independent review found no blockers; package/count/build/links/licence invariants, embedded lock identity, public-only publication scope and the pushed ref were checked. Production sources/configuration and the shipping lock remain unchanged. No candidate was built or benchmarked and no backend, exclusion or rendering loss was approved.

The findings graduate [Compare candidate font controls and rendering](18-compare-font-controls-and-rendering.md) into a concrete HITL evidence ticket, blocked on the baseline and acceptance discussion and required before backend selection. [Collect the FreeType rendering and cost baseline](14-collect-freetype-baseline.md) remains the next frontier; prototype outcomes may expose further decisions.
