# GUI and plugin compatibility reference

Evidence for [Assemble the GUI and plugin compatibility reference evidence](../issues/45-assemble-gui-compatibility-reference.md). This is a bounded reference observation, not a GUI architecture choice, comprehensive regression suite or performance qualification.

## Reference and provenance

- Editor source: `e556d388892280d8343a82a625941137e3a6ac5f`, the agreed pre-redesign reference. No production code, existing tests or goldens changed.
- Executable: 8,419,392 bytes, SHA256 `bde5ae899aef0302c41201ad54569f045bdef0db6a46874bd0a6f835642cb186`. This is the clean release artifact established in the [GUI cost baseline](gui-cost-baseline.md), with its recorded Rust/Cargo versions, input hashes and build profile: normal default X11 features, no `test_hooks`, no native Wayland feature; thin LTO, one codegen unit, abort-on-panic. The runner checks the executable hash, pinned checkout and unmodified production inputs before launch.
- All 90 staged runtime files were checked byte-for-byte against tracked reference data/API files, with only the installer's `@PROJECT_VERSION@` → `2.1.7` substitution in `core/start.lua`. Their hashes, including bundled fonts and plugin files, are in [results.json](gui-compatibility-reference/results.json).
- Environment: Linux 6.18.57, x86-64, Intel Core i9-12900H; Node v24.21.0, GCC 15.3.0, bubblewrap 0.12.0. Private Xvfb: 1280×800×24, DPI 96; editor window 1024×640, `LITE_SCALE=1`, `LANG=C.UTF-8`. The recorded display provenance identifies the owned server and cookie. No host-desktop window or input was used.
- Collection: 2026-10-23, 11:34:28–11:34:30 UTC. One final launch per configuration; these timestamps describe collection, **not latency measurements**. Earlier setup/debugging runs are not pooled into the results.
- The normal `core.init`, bundled plugin configuration, user-plugin discovery, `core.run` and rendering loop execute. The disposable init wraps `core.step`; it does not substitute a fake GUI. Successful runs complete nine drawn scenario phases and exit normally after a forced quit of scratch documents. The probe rejects logged `core.error` calls and asserts absence of `lxl_test`.

Artifacts: [manifest](gui-compatibility-reference/manifest.json), [results](gui-compatibility-reference/results.json), [probe](gui-compatibility-reference/probe.lua), [runner](gui-compatibility-reference/run.mjs), [isolation helper](gui-compatibility-reference/isolation.c). Raw logs, exact sandbox arguments, verified plugin bytes and setup attempts remain in `target/gui-compat-reference.oglCZs/` (about 3 MiB). The JSON snapshot retains stdout/stderr and markers; it does not depend on retaining those scratch logs.

## Source selection and version findings

The initial [source research](gui-compatibility-plugin-research.md) investigated the dated upstream guidance. `lite-xl/lite-xl-plugins@master` was still mod-version 3 at the recorded revision, whereas the fork's branch named `3.0` supplied mod-version 4 files. **Neither a branch name nor an accepted header establishes runtime compatibility.**

All seven inspected candidates are pinned to [adamharrison/lite-xl-plugins commit `ce2da6f190ceea5405ec211fb1d3d47e6c770179`](https://github.com/adamharrison/lite-xl-plugins/commit/ce2da6f190ceea5405ec211fb1d3d47e6c770179). The manifest records commit-pinned raw URLs, byte sizes, full Git blob SHA1s and SHA256s. Canonical downloaded bytes matched both expected blob IDs and sizes before execution; no source/header edits or version bypasses were used.

The original research snapshots were transcriptions, not canonical downloads: Indent Guide lost one trailing space and Scale Status lost its final newline. Those snapshots are not execution inputs. `verified/plugins/` and the hashes in the published manifest identify the actual bytes.

### Observed sample

| Candidate | Exercised pattern | Individual result |
| --- | --- | --- |
| `bigclock` | Real `View` subclass, command-driven `Node:add_view`, active-view drawing, shared style/fonts, configuration-driven font resize and close | Passed exercised scenarios |
| `autosaveonfocuslost` | `RootView:on_focus_lost` override, node-tree walk, document save, delegation to prior handler | Passed; only a scratch document was saved |
| `scalestatus` | Structured status item, bundled `plugins.scale`, shared configuration, item show/hide | Passed |
| `custom_caret` | `DocView.update` wrapper and replacement `draw_caret`; configuration/style, block-caret drawing | Passed |
| `smoothcaret` | `DocView.update` and `draw_caret` wrappers, per-view animation state, captured-method composition | Passed |
| `bracketmatch` | Intended `DocView` update/text-drawing wrappers plus commands/keymap | **Pre-existing failure:** first update calls absent `DocView:get_selection()` at plugin line 128 |
| `indentguide` | Intended `DocView` update/text-drawing wrappers and shared style | **Pre-existing failure:** first update calls absent `DocView:get_selections()` at plugin line 85 |

The two failures occur on the untouched reference, before the probe's first scenario phase. Their methods execute through the normal plugin loader, including acceptance of Bracket Match's triple-dash mod-version comment, but the document-owned selection methods are not available on `DocView`. Their drawing and navigation checks therefore never run. Further API incompatibilities are possible; the first failure is not an exhaustive diagnosis. These are recorded failures, **not passing probes, repaired plugins, newly approved exclusions or candidate regressions**.

The five working files total 12,781 bytes; all seven inspected candidates total 25,672 bytes. Selection is by extension behaviour, not popularity. Heavy native/process/network integrations were deliberately not selected.

### Configurations and composition

The final snapshot contains three precondition launches and eleven plugin launches:

- Isolation negative checks: passed. No-display launch: expected exit 1 with the specific missing-display error. Normal-core/bundled-plugin control: passed all scenario phases.
- Seven individual plugin launches: five passed, two failed as above.
- `caret-combination`: `custom_caret` + `smoothcaret` passed together.
- `working-five`: all five successful plugins passed together.
- `drawing-combination`: Bracket Match + Indent Guide failed in Bracket Match's update. `initial-five`: the original five candidates failed at the same point. Neither combination is a passing compatibility configuration.

The working combination's observed loader order is `autosaveonfocuslost,bigclock,custom_caret,scalestatus,smoothcaret`. Its update chain reaches the captured base through `smoothcaret → custom_caret → bundled linewrapping → DocView`. `smoothcaret.draw_caret` captures `custom_caret.draw_caret`; Custom Caret intentionally replaces the base rather than calling it. This distinction matters: compatibility cannot require every legitimate replacement to delegate to a base method. Reversed load order was not tested or endorsed.

User configuration otherwise starts empty, with the reference's bundled defaults and a fixed session/fixture. During scenarios, the probe temporarily disables caret blinking and sets Custom Caret's shape to `block` for one drawn frame, then restores both. Big Clock's scale is exercised at 2 then 1. Scale Status's item is hidden/shown directly. Its optional settings-UI interactions and deferred configuration callback are not certified. `scalestatus` requires the owned bundled `plugins.scale`; `custom_caret` optionally attempts `plugins.settings`, absent here and safely skipped. No other third-party dependency was installed.

## Focused GUI-contract observations

The normal-core control and each successful configuration ran the following checks. “Passed” describes the assertions in this probe, not all behaviour in the category.

| Area | Automated evidence | Boundary / remaining gap |
| --- | --- | --- |
| Identity, ownership and lifecycle | Additive `View` subclass has exact/subclass identity; inactive tab updates without drawing; a right split lays out/draws; activation and close restore a leaf and remove the panel | Synthetic panel, not a third-party locked/resizable sidebar; no comprehensive node mutation coverage |
| Geometry, clipping, measurement and hit testing | Positive layout sizes; panel clip equals assigned geometry within 0.5 px; node/tab logical hit tests; text-draw call accepts three arguments and returns one number | No pixel comparison, glyph/caret hit-testing audit, complex text or fractional-scale validation |
| Order and overrides | Deferred draw callbacks run `1,2,3`; captured update/text-draw chains reach base; real caret replacement/wrapper order is inspected and exercised | Caret pair tested in one loader order; failing text-decoration plugins do not establish their draw-chain behaviour |
| Caret drawing | Native X11 focus is acquired on the private display; focused document caret reaches renderer rectangle calls; Smooth Caret has visible caret state; Custom Caret block shape executes | One visible selection; no visual animation-quality, off-screen/multiple-caret or IME-caret correctness claim |
| Commands/keymap | Class predicate forwards the same active-view object; invalid first binding falls through to the next valid command | Direct Lua keymap call, not native keyboard delivery; failing Bracket Match navigation is unverified |
| Focus/save | Synthetic `core.on_event("focuslost")` saves the dirty scratch file with exact content and delegates to the predecessor; fixture stays clean | Native focus acquisition is real, but focus-loss dispatch is synthetic; real desktop focus transitions and exclusions for other document types remain unverified |
| Prompt/deprecated forms | Deprecated `set_text` and positional `CommandView:enter` draw a prompt, retain text and focus, then cancellation restores the document and supplies the expected callback argument | Not a complete legacy overload audit; submit/validation paths not asserted |
| Status and custom view | Scale Status's getter matches the bundled scale value, visibility toggles; Big Clock draws, changes font size with config, then closes | No settings GUI, status-click audit or human appearance review |
| Text/IME consumption | Synthetic `textinput` inserts text; `textediting` enters composition; `ime.stop` ends it | No real IME, composition-window positioning, native text delivery or input-to-visible latency claim |
| Context menu | Menu becomes active and draws; logical pointer hit selects an item through RootView; command receives the same argument and focus returns to the document | Synthetic pointer call; no native pointer delivery, submenu or keyboard-menu audit |
| Shutdown | Successful process exit after the scenario's forced quit; no logged core errors | Not an unsaved-document confirmation or every close/restart path |

No live human observations were collected. Native golden suites and existing startup checks remain separate hard gates; they were neither weakened nor rerun as part of this composed-GUI evidence collection.

## Execution isolation and limits

Source inspection and advisory review preceded third-party execution, including a separate advisory for the two caret additions. The first gates ran trusted code only; a failed gate stopped the collection before any plugin launch. Setup corrections (namespace flags, closure enumeration, bare-Xvfb focus driving) are retained as environment failures, not plugin outcomes.

Each launch uses unprivileged bubblewrap user/PID/network/IPC isolation, dropped capabilities, disabled nested user namespaces, a cleared environment, private `/proc` and `/dev`, read-only reference/runtime/plugin inputs and a fresh writable tmpfs. Only 45 enumerated Nix runtime closure paths are mounted, not the whole store or developer RPATH. No personal home/configuration, project checkout, SSH agent, host Wayland socket or live DBus service is exposed. The only display socket/cookie belongs to the private Xvfb. The existing [focus helper](gui-cost-baseline/focus.c) verifies its expected display before setting focus.

The trusted negative helper verified host-only file and checkout reads fail, the executable is not writable, environment canaries are absent, PID/network namespaces are private, the isolated namespace has no IPv4 routes, and scratch writes work. Removing `DISPLAY` fails specifically at native window creation; restoring the private display permits the normal-core control to complete. A request to raise a window was insufficient on bare Xvfb, so focus is driven externally and asserted inside the editor before caret checks proceed.

Per-process limits: 2 GiB virtual address space, 128 processes, 256 open files, 16 MiB file size, 20 s CPU, core dumps disabled. `/work` tmpfs is 128 MiB and `/tmp` 16 MiB. External timeout is 45 s with a further 5 s kill deadline; the parent also bounds captured output and has a 55 s outer deadline. The address-space limit was admitted only after the trusted control passed; it is a containment limit, not a memory measurement.

This is a filesystem/network/display isolation arrangement for these inspected Lua files, **not a different-user privilege boundary, hardened malware service or whole-host aggregate resource guarantee**. Review the setup again before executing other plugins. Autosave on Focus Lost deliberately writes files; an ordinary disposable userdir alone would not have protected real documents.

## Reproduction

This is a machine-specific evidence runner, not a portable plugin-testing product. It pins this executable and Nix paths; changing environment/pins requires review rather than relaxing checks. Use an owned checkout at the pinned editor revision with unchanged production inputs, the matching staged reference install from the cost baseline, and the recorded tools. Do not switch or modify a developer's live checkout to satisfy the checks.

From that checkout, with these evidence helpers available:

```bash
set -euo pipefail
ulimit -c 0
root=$(mktemp -d "$PWD/target/gui-compat-reference.XXXXXX")
prefix=/path/to/verified/reference/usr/local # exact recorded executable + runtime data
mkdir -p "$root/verified/plugins"
node - "$root" <<'JS'
const fs=require('fs'), root=process.argv[2];
const s=JSON.parse(fs.readFileSync('docs/wayfinder/assets/gui-compatibility-reference/results.json'));
for(const [name,m] of [['manifest.json',s.plugin_manifest],['followup-manifest.json',s.additional_plugin_manifest]])
  fs.writeFileSync(root+'/verified/'+name,JSON.stringify(m,null,2)+'\n');
JS
jq -r '.plugins[] | [.name,.source_url,.sha256,.git_blob_sha1,.size_bytes] | @tsv' \
  "$root/verified/manifest.json" "$root/verified/followup-manifest.json" |
while IFS="$(printf '\t')" read -r name url hash blob bytes; do
  file="$root/verified/plugins/$name.lua"
  curl --fail --location --proto '=https' --max-time 30 --retry 2 "$url" -o "$file"
  printf '%s  %s\n' "$hash" "$file" | sha256sum --check
  test "$(git hash-object --no-filters "$file")" = "$blob"
  test "$(wc -c < "$file" | tr -d ' ')" = "$bytes"
  chmod 444 "$file"
done
cc -O2 -Wall -Wextra -Werror docs/wayfinder/assets/gui-compatibility-reference/isolation.c -o "$root/isolation"
cc -O2 -Wall -Wextra -Werror docs/wayfinder/assets/gui-cost-baseline/focus.c -o "$root/focus" $(pkg-config --cflags --libs x11 xtst)
node docs/wayfinder/assets/gui-compatibility-reference/run.mjs --self-test
. scripts/rust-port/private-display.sh
trap 'pd_profile_stop' EXIT
pd_profile_start xvfb "$root/display"
pd_describe > "$root/display-provenance.txt"
pd_run env GUI_REFERENCE_DISPLAY="$pd_display" node \
  "$PWD/docs/wayfinder/assets/gui-compatibility-reference/run.mjs" "$root" "$prefix"
```

`--gates-only` can be appended to run only the trusted preconditions. Each launch ID requires fresh input directories. Read `results.json`: the runner records reference failures and may exit successfully with them present; its exit status is **not** a plugin-compatibility gate. The expected snapshot has seven passing plugin configurations and four explicitly failed configurations, plus the three precondition launches.

## Inputs to the next decision

The working five-plugin combination and normal-core scenarios provide reproducible, bounded configurations for the [calibration prerequisite](../issues/46-calibrate-gui-comparison-noise.md). They do not choose numerical tolerances or certify arbitrary plugins. Reference failures must remain visible, without becoming permission to break other legacy behaviour.

Required desktop/IME/input/scale checks, a small real-project workload, broader retention observations, visual review, and the final candidate acceptance/stop rules still belong to [Define GUI performance and customisation acceptance criteria](../issues/43-define-gui-acceptance-criteria.md) and its calibration prerequisite. No architecture, numerical performance budget or adoption approval follows from this reference.
