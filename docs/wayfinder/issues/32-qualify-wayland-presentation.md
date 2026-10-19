# Qualify Wayland CPU presentation without unsupported pixel padding

Parent: ../sdl3-map.md
Labels: wayfinder:prototype
Type: prototype
Mode: HITL
Status: resolved
Assignee: pi-wayland-presentation-resume
Blocked by: 26, 29

## Question

What supported CPU-presentation route preserves the editor's RGB pixels on the tested Wayland desktop without relying on softbuffer's undocumented tolerance of nonzero XRGB padding?

Continue from [Prototype window, scale and IME on Wayland and X11](29-prototype-window-scale-ime.md). The standalone probe is preserved on local throwaway branch `prototype/sdl3-window-probe`, commit `f8dacae0`; locate its worktree with `git worktree list`. Its README records the evidence and its `check-visible.sh` provides the visible-pixel/idle regression check. Do not promote the workaround into production.

The native capture was uniformly transparent black with documented zero-padding XRGB buffers despite nonblack CPU samples and successful attach/commit calls. Changing only the high byte to `0xff` restored the image. Disabling IME alone did not restore it. These observations do not identify a compositor, driver or crate as the culprit.

Investigate pinned upstream contracts/source and the actual buffer/compositor path, then test the smallest supported remedy in a throwaway probe. Preserve the original red-capable zero-padding reproduction and compare under matched conditions, with correct active-window captures and the user's live review. Record scale/backend, exact features and dependencies; preserve RGB pixels and CPU-only presentation. Do not infer success from `present()` alone or silently select a new crate, fork or SDL fallback.

Resolve only when a supported remedy is explained and demonstrated on the reproducing desktop, or the user explicitly decides an alternative disposition with its limits. An explanation alone without a supported, demonstrated route does not clear the compatibility gap. Any exception to the agreed acceptance criteria needs explicit approval. No stack selection, production implementation or acceptance-gate waiver is authorized here.

## Comments

### Takeover and matched native Wayland 1× comparison — partial evidence

- The user explicitly authorized taking over the prior claim, followed by two eight-second probe runs with probe-only local captures. No previous claim or production change is inferred from that permission.
- Reused the preserved diagnostic branch `prototype/wayland-presentation`, commit `c4d11679da84c16e7efb8be678afca0f0b69aed0`, in `../heavy-xs-presentation-probe`; no probe source changes. The original workaround snapshot remains at `f8dacae0`. The diagnostic defaults to documented zero padding and exposes an explicit `zero|ff` comparison.
- Offline build, all **9 unit tests**, formatting and launcher syntax checks passed. Probe lock SHA-256: `25d736f989c7f615073a59355bf0f05f968291077217ebf6bac6db48722d2c85`; live executable SHA-256: `d52a547b8fedb37adc72ab6230594796b82bb95f171833c125f31892fa40f3bc`.
- Both cases used the same native KDE Wayland session, KWin `6.7.5`, decorated mode, `PROBE_NIX=1`, `WAYLAND_DEBUG=client`, and eight-second lifetime. A pre-run `kscreen-doctor` query reported both connected outputs at scale 1; [post-run environment context](/tmp/wayfinder-presentation.4htNMi/environment-context.txt) records the KWin version and output scales again. Both runs presented and captured **2545×1340 at scale 1.000**, exiting 0. Per-window output identity and GPU/driver versions were not recorded, so this is not a claim to reproduce the earlier fractional-scale environment.
- Before capture, a temporary read-only KWin script verified the active probe's exact mode-bearing title and executable PID. Identity was checked immediately before/after capture and intervening focus-loss/other-window notifications caused rejection; the script was unloaded afterward. This is a snapshot-and-notification guard, not an atomic pinned-window capture. The zero run contains a focus excursion outside the capture interval, so uninterrupted focus/idle across the entire run is not claimed. Screenshots contain only probe content, were kept local, and were not copied to the clipboard. No display settings, services or editor files were changed.
- **Observed automated result: both zero and ff are visible; their captured pixels are exactly identical** (ImageMagick AE `0`). Both pass dimensions/frame agreement, non-uniform-content and exact green-strip RGB `#244C3A` assertions. Image-reader inspection independently reports the grid, green strip, diagnostic text and text field in both. Its tentative downscaled border difference is not supported by the exact pixel comparison.
- The repeatable pixel assertion is `python3 /tmp/wayfinder-presentation.4htNMi/check-pixels.py`. It passed for both cases; a blank/near-uniform capture or absent strip would fail. **The historical blank-window symptom did not reproduce in this decorated 1× comparison.** No cause or supported remedy is inferred; the original report included fractional scale and remains unresolved.
- Both protocol traces show `set_opaque_region` and a surface commit before the first non-null buffer attach, and buffer format value `1`. This records actual wire ordering, not attribution or a complete protocol/source audit.
- Local-only evidence: [zero metadata](/tmp/wayfinder-presentation.4htNMi/zero/metadata.txt), [ff metadata](/tmp/wayfinder-presentation.4htNMi/ff/metadata.txt), captures and protocol logs in the respective directories, [capture runner](/tmp/wayfinder-presentation.4htNMi/compare.sh), and [pixel assertions](/tmp/wayfinder-presentation.4htNMi/check-pixels.py). The parent directory is private (mode 700); temporary artifacts may expire. No raw screenshots, typed input or protocol dumps are committed.
- **Pending:** the user's visual feedback and a disposition for the non-reproduction, including whether to repeat under the earlier fractional-scale/borderless conditions. This is partial evidence only: the ticket stays claimed and stack selection stays blocked.

### Resumed claim — fractional/borderless rerun planning

The user explicitly authorized taking over the claim and planning a matched rerun. This is not permission to launch windows, capture, or change display settings; those actions await a separate agreement.

Read-only recovery found the original diagnostic logs in `/tmp/lxl-sdl3-window.Idkf2X/`: `fixed.log` and `ime-off.log` record native Wayland, **borderless**, actual **scale 1.200**, and a presented surface **2543×2076**. The later 1.250-scale corrected review is distinct. Output connector, compositor version and GPU/driver at the original failure remain unknown.

The newer comparison's `/tmp/wayfinder-presentation.4htNMi/` evidence directory no longer exists here, so its temporary captures, assertion script and identity-guard runner cannot be replayed. Its written results above remain historical observations, not newly verified evidence. The currently available diagnostic worktree is `/tmp/lxl-wayland-presentation/source`, branch `prototype/wayland-presentation` at `c4d11679`, not the other session's earlier sibling-worktree path.

Current read-only desktop queries report KWin **6.8.0**, native Wayland, DP-8 at **1.20**, DP-7 at **1.00**, and eDP-1 at **1.25**. No scale needs changing to test fractional rendering. This differs from the recorded KWin 6.7.5 / all-1× comparison; the original failing KWin version is unknown. A proposed rerun uses one existing fractional output, borderless mode and identical zero/ff cases, recording actual per-window output, scale and surface dimensions. It must not claim exact historical reproduction if those conditions differ. Any new capture runner must restore probe-title/PID checks; the inherited active-window screenshot helper alone is insufficient. No live actions performed during this planning step.

### Authorized fractional attempt — capture guard refused before screenshot

The user approved existing DP-8 at 1.20 and two eight-second borderless cases with probe-only captures and protocol logs, with no clipboard or display changes. The pushable diagnostic branch gained the guarded runner in `49de673c` (pure guard/parser self-check, nine Rust tests, build/format/syntax checks and independent safety review passed). Its read-only KWin observer uses a random journal marker and does not store unrelated windows' titles or journal lines. Guards are snapshots plus notifications, not atomic pinned-window capture.

The first zero-padding attempt reached **DP-8, scale 1.200, 2543×2076**, matching the recorded original scale/extent. It initially presented at 800×520, then KWin tiled it during the fixed settling interval. The geometry guard rejected it **before any screenshot**; the probe and observer were cleaned up. The ff control did not launch. This is no visibility result and does not reproduce or clear the original blank-window defect.

Private persistent artifacts (metadata, zero live/protocol logs; no PNG) are in `/home/juniorsundar/.local/state/lite-xl-wayfinder/presentation-fractional-20261020-103032`. Only this attempt's marker lines were inspected to classify the guard refusal. Current hardware context: NVIDIA module 595.104.02; Intel display device bound to i915 and NVIDIA device bound to nvidia. This does not identify which GPU presented the probe or assign fault.

The smallest readiness fix is committed as `77d4b38c` on `prototype/wayland-presentation`: wait for consecutive stable snapshots of the **same** PID/window/output/scale before arming capture. Pre/post/change checks remain in force; failure and sanitized guard events are now retained in the private output directory. Pure self-check and independent follow-up review passed. No automatic probe retry; another pair awaits the user's permission.

### Matched fractional/borderless pair — original symptom reproduced

After the user explicitly approved retrying, both eight-second cases completed from `prototype/wayland-presentation` at **`77d4b38c`**. Same native Wayland session, KWin **6.8.0**, output **DP-8**, scale **1.200**, borderless, identical guarded logical geometry `[3578,216,2119.166666666667,1730]`, presented/captured surface **2543×2076**. Each case verified exact mode-bearing title and executable PID, armed activation/geometry/caption/output notifications, and matching pre/post/final identity snapshots. Initial tiling changes were allowed only before capture; no intervening capture-interval changes were accepted. All temporary observers were verified unloaded. No clipboard or display changes.

| case | capture | user observation |
|---|---|---|
| Documented `zero` high byte | One color, **srgba(0,0,0,0)** throughout; expected strip/content absent | **Blank** |
| Unsupported `ff` control | **1301 colors**, opaque; exact green strip and dimensions pass | **Grid visible** |

The user explicitly confirmed **“Zero blank; ff showed the grid.”** Image-reader inspection independently found all expected UI only in ff; the zero PNG displays as blank white in its alpha-compositing viewer, consistent with the measured alpha-zero pixels, not an opaque white image. This is a reproduction of the observable symptom, not just a screenshot-path discrepancy.

Both actual protocol traces show `set_opaque_region` followed by a surface commit before the first non-null attach; `wl_shm_pool.create_buffer` advertises format **1 (XRGB8888)** in both cases. Missing initial opaque-region setup therefore does not explain these runs. This does not attribute fault to KWin, a driver or softbuffer. No supported remedy has been demonstrated.

**Private persistent evidence:** `/home/juniorsundar/.local/state/lite-xl-wayfinder/presentation-fractional-20261020-103445` (mode 0700), with per-case `result.json`, `live.log`, `wayland-debug.log`, `window.png`; root metadata, sanitized `guard-events.json`, and pixel comparison. Binary SHA-256: `c80ef41c2201b08c47b66c3cc2f8d8c732f30fe14b9fe5a9c4424bdbd30bef4b`; lockfile is unchanged from the earlier probe. No raw images or protocol dumps are committed. The branch is committed but not pushed; editor source remains unchanged.

Next: agree one-variable experiments against this reproduced failure, beginning with existing supported window attributes rather than a new crate, fork, or unsupported XRGB bytes. Another launch requires permission; the ticket remains claimed and stack selection blocked.

### Supported opaque-hint toggle — tested and rejected as a remedy

The user chose and authorized one-variable testing of winit's supported transparent-window attribute. Prototype commit **`cf649065`** adds `--transparent-window` and the guarded runner's `--compare-opaque-hint`. It compares `with_transparent(false)` with `with_transparent(true)`, **zero high byte in both cases**, preserving the borderless scene and XRGB8888 format. Defaults and the earlier zero/ff comparison are retained. No dependency or lockfile change. Ten Rust unit tests, offline build/format/shell checks, pure mocked guard/CLI checks and independent read-only review passed.

Two eight-second cases completed on the same **DP-8 / 1.200 / borderless / 2543×2076** setup with the same identity/capture guards. Both captures contain only **srgba(0,0,0,0)**, and their pixel difference is **AE 0**. The user confirmed **“Yes, both were blank.”** The actual trace demonstrates the intended single-variable change: default sends `set_opaque_region(wl_region#...)`; transparent-window sends **`set_opaque_region(nil)`**. Both continue to advertise format 1 / XRGB8888. All ephemeral observers were verified unloaded. No clipboard or display changes.

**Verdict:** disabling the opaque-region hint via this supported winit attribute **does not solve the reproduced failure**. This rejects that specific remedy; it does not prove that all compositing/window paths are equivalent or identify a faulty component. The ff workaround remains unsupported and is not approved for production.

Private persistent artifacts: `/home/juniorsundar/.local/state/lite-xl-wayfinder/presentation-opaque-hint-20261020-104632` (0700), same metadata/capture/protocol layout as the preceding comparison, with cases `zero-default` and `zero-transparent`. Code and runnable instructions are committed on `prototype/wayland-presentation`, currently `cf649065`, not pushed or merged; raw evidence remains outside git. The ticket stays claimed and unresolved. A same-output decorated zero-padding control could next isolate decoration mode, but is not itself an acceptable loss of borderless support and has not been authorized or run at this point in the investigation.

### Decoration control — failure is not confined to borderless mode

The user authorized two eight-second zero-padding cases on DP-8 at the existing 1.20 scale: borderless then decorated. Commit **`2e7e252f`** adds `--compare-decorations` without changing Rust rendering or dependencies. The guard now records and validates **client geometry separately from frame geometry**, so the decorated title bar cannot create a false physical-buffer mismatch. Missing/invalid client geometry fails closed. Pure guard/CLI checks, JavaScript syntax, shell syntax, whitespace checks and independent read-only safety review passed; Rust source was unchanged from the preceding ten-test snapshot.

An initial readiness check refused before opening any probe because DP-7 at 1× was active. After the user activated DP-8 and approved retrying, both cases completed. The user missed that pair, then explicitly requested a repeat; the repeated pair also completed and the user confirmed **“Both content areas were blank.”**

| case | actual output/scale | presented and captured client extent | content result |
|---|---|---|---|
| Zero, borderless | DP-8 / 1.200 | 2543×2076 | One transparent color; grid/strip/text absent |
| Zero, decorated | DP-8 / 1.200 | 2536×2028 | One transparent color; grid/strip/text absent |

Both actual traces retain an opaque region and format 1 / XRGB8888. Exact PID/title/output and client/frame checks passed, and all temporary observers were verified unloaded. No clipboard, display settings or production source changed.

**Interpretation:** the reproduced zero-padding failure also occurs with decorations in this same fractional-scale environment. It is not confined to borderless mode. This does not prove fractional scaling is the sole cause, assign fault to a component, or demonstrate a supported remedy.

The initial runner printed AE 0 even though decorated client extents differ; **that metric is not full-image pixel parity and is disregarded here**. Follow-up commit **`dd6cefb7`** explicitly skips AE for unequal capture dimensions and records both sizes. Pure self-check, syntax/whitespace checks and independent read-only review passed. No additional live run was needed for this reporting-only correction.

Private persistent evidence: `/home/juniorsundar/.local/state/lite-xl-wayfinder/presentation-decorations-20261020-105846` (first completed pair; no human confirmation) and `/home/juniorsundar/.local/state/lite-xl-wayfinder/presentation-decorations-20261020-110006` (user-observed repeat), each private with the same per-case logs/results/captures and root sanitized guard metadata. Raw evidence is not committed. Source and instructions are committed on `prototype/wayland-presentation` at **`dd6cefb7`**, not pushed or merged. The ticket remains claimed and stack selection blocked; no further desktop launches are authorized by this result.

### Upstream format-route investigation — isolated experiment authorized

Primary-source findings are preserved as **Wayland CPU presentation — upstream triage** (`docs/wayfinder/assets/wayland-presentation-upstream.md`) on `prototype/wayland-presentation`, commit **`2c32ee53`** (asset is on that branch, not merged into this checkout; retrieve with `git show 2c32ee53:docs/wayfinder/assets/wayland-presentation-upstream.md`). The checked released softbuffer 0.4.8 Wayland path hardcodes XRGB8888; upstream has an unreleased alpha-mode API that can choose ARGB8888. No matching upstream fixing commit or demonstrated remedy was identified. Search limitations are explicit; no culprit is assigned.

After the purpose and limitations were explained, the user said **“do the test then.”** This authorizes an isolated upstream-git comparison, not production adoption: **the same full softbuffer revision `9a7c9caf4ac29a8ae361d68c9a5babb97e36dc70`** for XRGB8888 / `AlphaMode::Ignored` / zero high byte versus ARGB8888 / `AlphaMode::Premultiplied` / opaque alpha. `Ignored` is necessary for the zero arm because upstream's `Opaque` mode requires alpha 0xff in debug builds. Existing source RGB is preserved in both arms; supported ARGB alpha is not the historical unsupported XRGB `ff` workaround.

Preparation is isolated on **`prototype/wayland-alpha-mode`**, based on the preserved presentation branch. Following offline checks and independent safety review, run exactly two eight-second borderless cases on DP-8 at its existing 1.20 scale with the existing PID/title/output/client-geometry capture guards and private protocol logs. No clipboard, display-setting, fork or production dependency changes. Actual wire formats must be verified as 1 (XRGB) and 0 (ARGB), and the user's live review is still required. Qualification remains unresolved.

### Pinned upstream ARGB route — demonstrated at the original failing extent

Code commit **`52747afd`** on **`prototype/wayland-alpha-mode`** implements the authorized same-revision format comparison. The Rust drawing scene remains packed RGB, converted row-by-row to upstream typed pixels so padded strides do not skew it. Twelve unit tests include both alpha-mode mappings, RGB/alpha preservation and odd-width padded rows. Locked offline build, formatting, JavaScript syntax/pure guard self-check, shell syntax, whitespace checks and independent safety review passed. No new SDL/GPU runtime dependency or production source change.

The first pair completed at **800×520** with XRGB blank and ARGB visible, but the user missed it and explicitly requested a repeat. The observed repeat matched the original reproduction: **KWin 6.8.0 / DP-8 / scale 1.200 / borderless / transparent=false / 2543×2076**. Both cases had identical guarded frame/client geometry `[3578,216,2119.166666666667,1730]`, source RGB scene, binary and full upstream pin.

| arm | alpha mode / bytes | actual create_buffer format values | capture and physical observation |
|---|---|---|---|
| XRGB zero | `Ignored`, X byte 0x00 | `[1,1,1]` — XRGB8888 | One transparent color `srgba(0,0,0,0)`; blank content |
| ARGB opaque | `Premultiplied`, alpha 0xff | `[0,0,0]` — ARGB8888 | 1301 opaque colors; grid, green strip, labels and text field visible |

The user confirmed **“Yes: first blank, second visible.”** Independent image-reader inspection agreed with the content result. Both cases exited 0; actual format validation and all PID/title/output/geometry guards passed. All six temporary observers were verified unloaded. No clipboard or display-setting changes.

**Verdict:** an **API-compliant CPU-presentation route on the pinned unreleased upstream revision** now works on the reproducing desktop and extent, without nonzero XRGB padding. Keeping the XRGB failure on the same new revision shows that upgrading alone did not eliminate it. This does not identify the original faulty component, establish other backends/input/performance after the API upgrade, or authorize using an unreleased dependency in the migration plan. That dependency disposition remains the user's decision; the ticket stays claimed and unresolved, and stack selection remains blocked until the disposition is settled.

Private evidence: `/home/juniorsundar/.local/state/lite-xl-wayfinder/presentation-alpha-mode-20261020-114218` (first, unobserved 800×520 pair) and `/home/juniorsundar/.local/state/lite-xl-wayfinder/presentation-alpha-mode-20261020-132425` (observed repeat, root mode 0700). Binary SHA-256: `1b8cf218a44f0e92f0cd3de5e6537c32fb41268272c9ca7b2f909733ab6f4ee8`; lockfile SHA-256: `5a1900bd66a367045c4ade2a89e2f241e55f499e2e67111fab1b2a097e166683`. The code and runnable instructions are on the local-only prototype branch, not pushed or merged; raw images/traces remain outside git. The released 0.4.8 baseline remains preserved on `prototype/wayland-presentation`. No further live launches are authorized by this result.

### Resolution

**Outcome: demonstrated CPU-presentation route accepted for planning, with an explicit unreleased dependency pin.** After reviewing the matched live result and the dependency limitation, the user chose **“Accept tested pin for planning.”** This resolves the presentation investigation and unblocks stack selection; it does not select the stack or authorize implementation.

The accepted route is upstream softbuffer revision **`9a7c9caf4ac29a8ae361d68c9a5babb97e36dc70`**, configured with **`AlphaMode::Premultiplied`**, producing Wayland **ARGB8888** with alpha **0xff** and unchanged RGB. This is an upstream API-compliant format, not the unsupported nonzero-XRGB-padding workaround or a maintained fork. The pin is unreleased; its package version string must not be mistaken for released softbuffer 0.4.8.

Evidence is the user-observed matched pair above: XRGB stayed blank while ARGB showed the grid and text on **KWin 6.8.0 / DP-8 / 1.20 / borderless / 2543×2076**. The actual protocol formats were verified as 1 and 0, and RGB conversion/stride checks passed. The original faulty component remains unidentified. Reproducible source and runnable instructions are preserved on **`prototype/wayland-alpha-mode` at `241ee9cf`** (test code `52747afd`); inspect `docs/wayfinder/assets/sdl3-window-probe/PRESENTATION.md` on that branch. Private evidence paths and hashes are recorded above, not committed or published.

**Limits carried forward:** accepting this exact pin is a planning decision, not production adoption or a release-readiness claim. Stack selection and the implementation handoff must preserve the recorded dependency, format and stride requirements, and revalidate affected backend/input paths and full-editor drawing/performance when integrating the new API. All existing contract, desktop and cost gates retain their previously agreed dispositions, including conditional, deferred and accepted-unverified checks; none becomes a pass or a new planning blocker merely through this closure. No feature loss, hidden SDL fallback or gate waiver is accepted. The prototype's scratch allocation/copy is diagnostic code, not a production pixel-store or performance decision.

Only this investigation is resolved. No production change, push, upstream post or further desktop launch is authorized. Continue with [Select the Rust-native platform stack](27-select-platform-stack.md); [Define the SDL3 replacement handoff](28-plan-sdl3-migration.md) must carry these limits.
