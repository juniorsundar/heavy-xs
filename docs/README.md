# Heavy XS documentation

The files under `api/` document the Lua-facing interfaces provided by the Rust
core. They contain metadata and annotations, not executable implementations.
They use [EmmyLua annotations](https://emmylua.github.io/annotation.html) for
completion and descriptions in Lua language servers.

## Core interfaces

The editor's Lua layer uses these native namespaces:

* [system](api/system.lua)
* [renderer](api/renderer.lua)
* [regex](api/regex.lua)
* [process](api/process.lua)

Finally, all global variables are documented in the file named
[globals.lua](api/globals.lua).

## Guides

- [Font behaviour and supported formats](fonts.md)
- [Regex syntax and migration](regex.md)
- [Project provenance and upstream credits](provenance.md)

## Architecture and planning

Historical Rust-port plans, architecture decisions and Wayfinder evidence live in
[heavy-xs-architecture](https://github.com/juniorsundar/heavy-xs-architecture).
Wayfinder maps and decision tickets are maintained in that repository's GitHub
issues, not in local Markdown.

The architecture repository is private and checked out as the optional
`docs/architecture` submodule. With access, fetch it using
`git submodule update --init docs/architecture`. Building the editor does not
require it.
