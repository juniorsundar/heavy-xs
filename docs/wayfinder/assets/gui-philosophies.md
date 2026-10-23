# Editor GUI philosophies and extension costs

Research for **Compare editor GUI philosophies and extension costs**, a child of **A fast, deeply customisable GUI without breaking Lite XL plugins**. The canonical tracker remains in the main checkout; this throwaway research branch is evidence, not a merge candidate.

## Scope and source context

Investigated 2026-10-23 against first-party documentation and source references. This is a qualitative comparison of extension ownership and iteration mechanisms, not a performance contest. No editor was built or benchmarked. Current documentation is unversioned and can change independently of the source snapshots below.

The research checked that these tagged source locations exist. The snapshots provide retrievable implementation context; they are not a claim that all current documentation describes those older snapshots:

- Zed `v1.25.2`, commit `d25fa01ccd788d812f236810b71af4feff6b2e38`: [extension API crate](https://github.com/zed-industries/zed/blob/d25fa01ccd788d812f236810b71af4feff6b2e38/crates/extension_api/Cargo.toml), [GPUI crate](https://github.com/zed-industries/zed/blob/d25fa01ccd788d812f236810b71af4feff6b2e38/crates/gpui/Cargo.toml).
- VS Code `1.85.0`: [extension-facing declarations](https://github.com/microsoft/vscode/blob/1.85.0/src/vscode-dts/vscode.d.ts), [extension-host API implementation](https://github.com/microsoft/vscode/blob/1.85.0/src/vs/workbench/api/common/extHost.api.impl.ts).
- Emacs `emacs-30.1`: [command-loop manual source](https://github.com/emacs-mirror/emacs/blob/emacs-30.1/doc/lispref/commands.texi), [window functions](https://github.com/emacs-mirror/emacs/blob/emacs-30.1/lisp/window.el), [redisplay engine](https://github.com/emacs-mirror/emacs/blob/emacs-30.1/src/xdisp.c), [input and command loop](https://github.com/emacs-mirror/emacs/blob/emacs-30.1/src/keyboard.c).

## The distinctions that matter

| Dimension | Zed | VS Code | Emacs |
| --- | --- | --- | --- |
| Reviewed extension model | Enumerated capabilities such as language support, themes and debuggers; some extension logic runs as Wasm. | Declared contribution points and extension APIs, plus custom webview content. | Lisp functions and editor objects used directly by extension code. |
| GUI ownership | GPUI provides the Rust UI machinery; the reviewed extension index does not list a general custom-view interface. | The workbench supplies standard views/layout; extensions contribute content and can control webview content. | Lisp can manipulate editor windows and presentation, within Emacs's buffer/window/frame and redisplay model. |
| Trying an extension change | Data-only changes and Rust/Wasm logic have different development paths. | Launch an Extension Development Host rather than rebuild the editor. | Evaluate Lisp expressions or load changed Lisp in the running editor. |
| Important limit | Do not infer arbitrary GUI extensibility from the existence of GPUI. | Broad extension capabilities do not mean unrestricted replacement of the workbench. | Broad programmability does not mean arbitrary pixel-level rendering or freedom from a display model. |

The rows summarise the cited evidence below, not an ordering from worst to best.

## Zed: distinguish the UI framework from the extension interface

**Facts.** The official [Developing Extensions](https://zed.dev/docs/extensions/developing-extensions) page enumerates languages, debuggers, themes, icon themes, snippets and MCP servers. The reviewed index does not enumerate a general GUI/custom-view hook. This is a bounded observation about those sources, not proof that no other mechanism exists or could be introduced.

[GPUI](https://gpui.rs/) describes a GPU-accelerated Rust UI framework with immediate and retained aspects, and shows Rust `Render` implementations constructing UI elements. A core UI framework and a public extension interface are different surfaces: having the former does not establish the latter.

Not every Zed extension needs Rust or Wasm. [Themes](https://zed.dev/docs/extensions/themes) are JSON; [language extensions](https://zed.dev/docs/extensions/languages) include configuration, queries and grammar integration. Where Rust extension logic is needed, the developing-extensions guide documents its Wasm target and the **Install Dev Extension** workflow. A pure theme edit should not be described as a full Zed rebuild.

**Inference.** A capability-specific interface makes supported extension roles explicit. A desired UI change outside those roles can move work into the core-development path instead. That is a mechanism-based distinction, not a measurement of how long either path takes.

**Unknowns.** This study does not enumerate the complete versioned extension ABI, quantify core build time/disk use, or establish which future visual extension proposals will ship. No specific frame-time or startup claim is made.

## VS Code: broad contributions through managed surfaces

**Facts.** [Contribution Points](https://code.visualstudio.com/api/references/contribution-points) let extensions declare commands, menus, views, themes and other features through their manifests. These are defined integration surfaces, not an instruction to modify any workbench object.

The [Extension Host](https://code.visualstudio.com/api/advanced-topics/extension-host) documentation distinguishes local Node.js, browser and remote hosts, with availability determined by the environment and extension capabilities. Separating extension execution from the UI is a design mechanism; it is not proof that extensions cannot affect responsiveness.

[Webviews](https://code.visualstudio.com/api/extension-guides/webview) provide extension-controlled HTML content and messaging between that content and the extension. [Custom editors](https://code.visualstudio.com/api/extension-guides/custom-editors) use webviews for their view. This gives substantial freedom inside a hosted region, which is different from replacing the entire workbench's layout or input machinery.

[Activation Events](https://code.visualstudio.com/api/references/activation-events) and [Extension Anatomy](https://code.visualstudio.com/api/get-started/extension-anatomy) describe when extension code activates; contribution declarations and execution need not happen at the same time. Some activation is inferred from contributions in newer releases, so old manifest examples should not be treated as universal current requirements.

The [first-extension workflow](https://code.visualstudio.com/api/get-started/your-first-extension) launches a separate Extension Development Host with F5. An ordinary extension experiment does not require rebuilding VS Code itself.

**Inference.** Declarative contribution surfaces can keep ordinary extensions out of the core build loop and allow activation to be deferred. The cost is that capabilities outside those surfaces require a different route. Messaging, activation and hosted-view lifetimes introduce their own constraints; this brief does not quantify their costs.

**Unknowns.** No extension-heavy startup, memory, UI latency or core build costs were measured. Process separation alone is not a performance guarantee.

## Emacs: runtime programmability within an editor model

**Facts.** The GNU [Lisp evaluation documentation](https://www.gnu.org/s/emacs/manual/html_node/emacs/Lisp-Eval.html) describes evaluating expressions from the live editor; the [evaluation reference](https://www.gnu.org/software/emacs/manual/html_node/elisp/Eval.html) explains evaluation of Lisp forms. The common Lisp experimentation path is not a rebuild of the editor executable. Loading changed definitions and evaluating them is distinct from proving that all existing state or advice has been reset.

The versioned [command-loop manual source](https://github.com/emacs-mirror/emacs/blob/emacs-30.1/doc/lispref/commands.texi) describes reading key sequences, executing their definitions and displaying the results. [Window-manipulation Lisp](https://github.com/emacs-mirror/emacs/blob/emacs-30.1/lisp/window.el) illustrates access to editor window operations, while the [redisplay engine](https://github.com/emacs-mirror/emacs/blob/emacs-30.1/src/xdisp.c) and [input/command loop](https://github.com/emacs-mirror/emacs/blob/emacs-30.1/src/keyboard.c) remain core implementation mechanisms.

**Inference.** Direct runtime programmability can shorten the change-to-result loop and make editor behaviours composable or replaceable without compiling the executable. It still operates within a particular editor/display model. Synchronous extension work in the read-execute-display path can delay the next interaction; this is a scheduling risk, not a claim that all Emacs configurations are slow. Long-running work can have other execution strategies, which were not surveyed exhaustively here.

**Unknowns.** This brief does not assess every redisplay hook, package interaction, asynchronous facility, native-compilation path or startup optimisation. Direct GNU HTML fetches were unreliable; command-loop details were checked through GNU-maintained manual source in the tagged mirror, with search results for the evaluation pages. The limitation matters if exact version-specific evaluation semantics become a design dependency.

## Questions this evidence supports, not answers for this project

- Which customisations need direct access to editor objects, and which benefit from a stable contribution interface?
- Where can the default editor provide structure without making that structure impossible to replace?
- Can extension discovery/declaration be cheap while expensive activation waits for actual use?
- How will plugin work interact with the input/update/draw path, and how will competing overrides compose?
- Which changes can be tried through Lua, which require a Rust build, and what are the measured time/disk costs of each?
- Which guarantees must new Lua access provide without changing legacy method overrides, callbacks or lifetimes?

These remain live project decisions. No framework, rendering backend, isolation model, lazy-loading scheme or interface design is selected by this research.

## Evidence limits

- The three products are not interchangeable architectures or a single freedom-versus-speed spectrum. Different extension surfaces make different comparisons meaningful.
- Current docs and tagged source snapshots are explicitly separate evidence contexts. The historical VS Code snapshot is not presented as its latest release.
- There are no controlled startup, responsiveness, memory, build-time or disk comparisons here. The user's experiences motivate the project priorities but are not benchmark data.
- API enumeration and source existence checks are not exhaustive implementation audits. Validate any mechanism the project comes to depend on against a specific source revision during later design work.
