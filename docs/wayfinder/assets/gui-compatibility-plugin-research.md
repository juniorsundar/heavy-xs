# GUI compatibility plugin research: bounded third-party sample

Evidence for the Wayfinder ticket "Assemble the GUI and plugin compatibility
reference evidence". Primary-source research only: every claim below cites the
owning source. No plugin was executed, built, or installed during this research.
No headers were modified. Subsequent byte verification, execution and the final
sample are recorded in the [compatibility reference](gui-compatibility-reference.md),
which supersedes this initial proposal for runtime outcomes. Local editor revision `e556d388892280d8343a82a625941137e3a6ac5f`
(`rust-port`); local inventory
`docs/wayfinder/assets/gui-extension-contract-inventory.md` section 7 was used
as the starting lead and is superseded here by fetched evidence where noted.

Snapshots + source manifest:
`target/gui-compat-reference.oglCZs/research/` (`plugins/*.lua`,
`SOURCE_MANIFEST.md`). These research snapshots are not execution inputs: two
had transcription differences. Use the canonical byte hashes in the
[published manifest](gui-compatibility-reference/manifest.json), not those snapshots.

## 1. Version facts (do not conflate these)

1. Local checkout declares mod-version 4: `data/core/start.lua` sets
   `MOD_VERSION_MAJOR = 4`, `MOD_VERSION_MINOR = 0`, `MOD_VERSION_PATCH = 0`.
2. `lite-xl/lite-xl-plugins@master` (HEAD `6e4aa640d679decd67d2c0b677ede52ef70a7bf9`;
   branches `2.0`/`2.1`/`2.2`/`master`) is a mod-version 3 source: its
   `plugins/indentguide.lua` still opens with `-- mod-version:3`
   ([raw](https://raw.githubusercontent.com/lite-xl/lite-xl-plugins/master/plugins/indentguide.lua)).
   It would fail the local version gate without header patching or a bypass;
   it is NOT the reference. ([branches API](https://api.github.com/repos/lite-xl/lite-xl-plugins/branches?per_page=100))
3. `adamharrison/lite-xl-plugins` branch `3.0` pins to commit
   `ce2da6f190ceea5405ec211fb1d3d47e6c770179` ("Updated for 3.0.",
   2026-09-15T03:17:19Z, unsigned)
   ([commit](https://github.com/adamharrison/lite-xl-plugins/commit/ce2da6f190ceea5405ec211fb1d3d47e6c770179)).
   All five sample files declare `mod-version:4` there, so no patching is
   needed. `3.0` is a moving ref: the pin is the commit SHA.
   ([branches API](https://api.github.com/repos/adamharrison/lite-xl-plugins/branches?per_page=100))
4. `CONTRIBUTING.md` (state-as-of October 2025) says only core plugins were
   considered ported and links non-core 3.0 work to the adamharrison fork.
   That pointer is now corroborated by the live v4 headers, but the document
   itself is stale and is not the authority for version eligibility. The pinned
   bytes/headers establish eligibility, not runtime API compatibility.
5. Local bundled `data/plugins/scale.lua` is `-- mod-version:4` (verified
   in-repo), so the sample's one cross-plugin dependency resolves to owned,
   version-compatible code.

## 2. Proposed bounded sample (5 files, 17,903 bytes, all fully inspected)

| Plugin | File | Size | Blob SHA | Coverage |
| --- | --- | --- | --- | --- |
| Big Clock | `plugins/bigclock.lua` | 2715 | `6ade5dd…` ([src](https://github.com/adamharrison/lite-xl-plugins/blob/3.0/plugins/bigclock.lua)) | View subclass + node insertion/layout, command, shared config/style |
| Indent Guide | `plugins/indentguide.lua` | 4658 | `587e9b5…` ([src](https://github.com/adamharrison/lite-xl-plugins/blob/3.0/plugins/indentguide.lua)) | `DocView.update` + `draw_line_text` wrap, config/style |
| Bracket Match | `plugins/bracketmatch.lua` | 8233 | `582fd8a…` ([src](https://github.com/adamharrison/lite-xl-plugins/blob/3.0/plugins/bracketmatch.lua)) | `DocView.update` + `draw_line_text` wrap, command + keymap, config/style |
| Autosave on Focus Lost | `plugins/autosaveonfocuslost.lua` | 973 | `6ea1bf4…` ([src](https://github.com/adamharrison/lite-xl-plugins/blob/3.0/plugins/autosaveonfocuslost.lua)) | Focus/lifecycle (`RootView:on_focus_lost` wrap), node-tree walk |
| Scale Status | `plugins/scalestatus.lua` | 1324 | `821b35d…` ([src](https://github.com/adamharrison/lite-xl-plugins/blob/3.0/plugins/scalestatus.lua)) | Structured `StatusView:add_item`, config `on_apply`, bundled dep |

Full blob SHAs, sizes, and commit-pinned download URLs: `SOURCE_MANIFEST.md`
in the research dir. Deliberately excluded: `lsp`, `terminal`, `tree-sitter`,
`plugin_manager` (native/external-process/network weight); `openfilelocation`
(uses `system.exec` to launch a file manager;
[sic](https://github.com/adamharrison/lite-xl-plugins/blob/3.0/plugins/openfilelocation.lua));
`centerdoc` (needs `plugins.treeview` + `system.set_window_mode`;
[sic](https://github.com/adamharrison/lite-xl-plugins/blob/3.0/plugins/centerdoc.lua)).

### 2.1 bigclock.lua — View subclass / insertion / layout

- `ClockView = View:extend()`, overrides `new`/`get_name`/`update`/`draw`;
  `command.add(nil, {["big-clock:open"] = ...})` inserts via
  `core.root_view:get_active_node():add_view(ClockView())`. Exercises the same
  subclass + `add_view` surface the inventory records for bundled TreeView.
- Draw path: `draw_background`, `renderer.font.copy(style["font"], size)`,
  `common.draw_text`, `style.background/text/dim`. Update path sets
  `core.redraw = true` on change and delegates to `ClockView.super.update`.
- Requires: `core`, `core.style`, `core.command`, `core.common`,
  `core.config`, `core.view`. Plus stdlib `os.date`, `collectgarbage`.
  No `require` of any other plugin. No io/filesystem/process/network use.

### 2.2 indentguide.lua — DocView drawing wrap + config/style

- Wraps `DocView.update` (recomputes per-frame indent tables over
  `get_visible_line_range`) and `DocView:draw_line_text(vline, x, y)`
  (emits `renderer.draw_rect` guides, then delegates to the captured method).
- Reads `style.guide`/`style.guide_highlight` with `style.selection`/
  `style.accent` fallbacks, `SCALE`, `config.plugins.indentguide.*`.
- Requires: `core.style`, `core.config`, `core.common`, `core.docview`. No
  other plugin dependency. No native/process/network/filesystem actions.

### 2.3 bracketmatch.lua — wrapping + input/commands combination

- Wraps `DocView:update` (bracket search, capped at 100 lines) and
  `DocView:draw_line_text`; `command.add("core.docview", …)` with the exact-
  identity predicate plus `keymap.add { ["ctrl+m"], ["ctrl+shift+m"] }`.
- Reads `style.syntax`/`style.syntax_fonts`, `SCALE`-derived `line_size`,
  `config.plugins.bracketmatch.*` incl. `get_value`/`set_value` scale
  conversions for the settings GUI.
- Requires: `core`, `core.style`, `core.command`, `core.keymap`,
  `core.docview`, `core.config`, `core.common`. No other plugin dependency.
- NOTE (sic, preserved unmodified): header is `--- mod-version:4` (three
  dashes). Still a valid Lua comment; whether the local version parser
  accepts it must be confirmed by main review (see §5).

### 2.4 autosaveonfocuslost.lua — focus/lifecycle

- Captures `RootView.on_focus_lost`, walks `core.root_view.root_node`
  (`leaf` → `views`, else `a`/`b`), saves dirty `DocView` docs except
  `CommandView`, user `init.lua`, and `.lite_project.lua`; then delegates.
- Uses the `system` global without `require` (relies on the sandbox global),
  `USERDIR`/`PATHSEP` globals, `view:is(DocView)` exact-identity checks,
  `doc:is_dirty()`/`doc:save()`, `core.log`.
- Requires nothing beyond `core`, `core.commandview`, `core.docview`,
  `core.rootview`. **Filesystem-mutating by design** (see §4).

### 2.5 scalestatus.lua — structured status surface + bundled dep

- `core.status_view:add_item({ name = "status:scale", alignment =
  StatusView.Item.RIGHT, get_item, position, tooltip, separator })` plus a
  config `on_apply` that shows/hides the item inside `core.add_thread`.
- Sole cross-plugin dependency: `require "plugins.scale"` → `scale.get()`,
  satisfied by bundled-local `data/plugins/scale.lua` (mod-version:4).
  Requires: `core`, `core.common`, `core.config`, `core.statusview`.

## 3. Dependency summary

- Zero third-party dependencies: bigclock, indentguide, bracketmatch,
  autosaveonfocuslost (all requires are `core.*` or Lua stdlib).
- One bundled-local dependency: scalestatus → `plugins.scale` (owned,
  version-compatible, no download needed).
- Dependency-heavy alternatives investigated and rejected for this bounded
  sample: `lsp`/`lsp_servers` (external server processes), `terminal`
  (pty/process), `tree-sitter` (native lib), `plugin_manager` (network).
  `openfilelocation` (`system.exec`) and `centerdoc` (`plugins.treeview` +
  `system.set_window_mode`) documented above as considered-but-excluded.

## 4. Source safety findings (static inspection; nothing executed)

- No `io.*`, `os.execute`, `system.exec`, socket/http, `require "lib…"`
  (native), or `package.cpath` manipulation in any of the five files.
- `autosaveonfocuslost.lua` **writes user files**: `view.doc:save()` on every
  dirty doc at every focus loss. That is its documented purpose, but any
  reference harness running it must use scratch userdirs — never a real
  checkout — or focus events will rewrite open files.
- `bigclock.lua`: `big-clock:open` adds a new view on every invocation with
  no dedup; `renderer.font.copy` allocates per scale change
  (`collectgarbage()` is called explicitly after). Low risk for reference
  use; note for any repeated open/close cycling evidence.
- `bracketmatch.lua` `redraw_char` temporarily assigns
  `DocView.draw_line_text = function() end` and restores it after
  `dv:draw_line_body`. Restoration is not in a protected call: an error
  inside `draw_line_body` would leak the noop onto the shared class table
  for all views. Pre-existing upstream hazard, preserved as-is.
- `bracketmatch.lua` `redraw_char`: `char_color` falls back to
  `style.syntax[token]`, which is nil for unknown token types; the nil is
  passed to `renderer.draw_text`. Whether the renderer tolerates nil colour
  is a compatibility question for the reference runs, not assumed here.
- `indentguide.lua` + `bracketmatch.lua` both wrap `DocView:draw_line_text`
  (and `DocView:update`): loading both exercises independent-wrapper
  composition/order on the same methods — an explicit evidence gap in the
  inventory (§6). Neither file sets a `priority:` header, so order falls back
  to the loader's default sort; `colorpreview` (not in sample) shows the
  `priority: 2000` form for contrast.
- `indentguide.lua` and `autosaveonfocuslost.lua` gate on `self:is(DocView)`
  / `view:is(DocView)`. Per the inventory, `is` tests exact class identity,
  so subclassed doc views would silently skip the wrapped behaviour — a
  combination worth asserting in reference runs.
- `scalestatus.lua` `on_apply` defers show/hide through `core.add_thread`;
  immediate post-config state is therefore not synchronous — timing to note
  in any status-bar evidence.

## 5. API/version incompatibilities and verification owed

- None of the five files needs a header change: all declare `mod-version:4`
  against the local `MOD_VERSION_MAJOR = 4` gate, unlike the org `master`
  (`mod-version:3`). The `---` (triple-dash) header in `bracketmatch.lua`
  needs a parser check by main review.
- APIs used that the inventory already confirms at this checkout:
  `View:extend`/`super`, `node:add_view`, `DocView.draw_line_text`/
  `draw_line_body`/`update`, `command.add`/`command.perform`,
  `keymap.add`, `StatusView:add_item`, `config` merge/`config_spec`,
  `core.add_thread`, `core.push_clip_rect`/`pop_clip_rect`, shared `style`
  table reads. Inventory refs: §§1–5 of
  `gui-extension-contract-inventory.md`.
- NOT confirmed by the inventory and owed to main review before reference
  runs: `RootView.on_focus_lost` existing/being called at `e556d388`;
  `core.status_view:get_item(name):show()/hide()`; `scale.get()`;
  `DocView:get_dline`/`get_visible_line_range`/`get_col_x_offset`/
  `get_line_text_y_offset`; `doc:get_indent_info`/`get_change_id`;
  `renderer.font.copy`; `system.absolute_path`. None are assumed; each is a
  named check, not a found incompatibility.
- No escalation needed: a plausible existing mod-version 4 sample was found
  (this one). Nothing was ported or relabelled.

## 6. Gaps

- Per-file SHA256 of the snapshot copies is not recorded: this environment
  cannot execute commands, so `sha256sum`/`git hash-object` could not be run.
  Git blob SHAs (API-sourced) are the pins; `SOURCE_MANIFEST.md` lists the
  exact re-verification steps for the main agent.
- The `3.0` branch moves; if it advances past `ce2da6f`, the commit-pinned
  raw URLs in the manifest still address the reviewed bytes, but freshness
  should be re-checked.
- Behavioural compatibility (load order effects, wrapper composition,
  focus/quit lifetimes, appearance) is explicitly not established here —
  that is the job of the reference runs this sample enables.
