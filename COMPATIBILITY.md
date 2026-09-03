# Language compatibility policy

OpenSCAD Viewer has two explicit, non-interchangeable language profiles:

1. `openscad-viewer-subset@1` is independently implemented by the browser and
   the original MCP geometry tools. Unsupported features produce errors.
2. `openscad/stable-2021.01` is the target contract for the repository-owned
   independent engine. It contains 38 built-in value functions and 35
   non-deprecated built-in modules. The independent implementation is still
   under qualification, so its MCP result explicitly sets
   `complete_language_claim: false` until every syntax, module, file and
   semantic gate passes. Deprecated compatibility aliases remain a separate,
   non-qualified tail.

The separately installed, checksum-pinned official OpenSCAD 2026.09.01
WebAssembly snapshot is a differential oracle only. It is exposed through the
explicitly named `openscad_official_*` tools for comparison and reference
exports; it is not the independent engine, a production fallback, or evidence
that the repository-owned implementation is complete. The project is not
affiliated with or endorsed by the OpenSCAD project. It never silently
delegates from one profile/provider to another.

The implementation is authored in this repository from public language
behavior. OpenSCAD source code, grammar, tests, diagnostic text, documentation
and bundled libraries must not be copied or mechanically translated. Small
team-authored programs may compare observable behavior against a separately
installed official executable; official binaries are not redistributed by the
browser application.

The official OpenSCAD codebase is GPL-2.0-or-later. `npm run setup:openscad` is
an explicit local action: it downloads the pinned archive from the official
project, verifies its SHA-256, and creates a deterministic NODERAWFS-disabled
copy in the gitignored `.open-scad-runtime/` cache. It also downloads and
verifies the pinned Basic Regular font and its SIL Open Font License 1.1 text;
the official lane will not start if any of those manifest-bound files fails
verification. The archive, patched runtime, font, and license are not installed
by `npm install` or included in the browser bundle, and this repository does not
commit or redistribute them. See the
[official source license header](https://raw.githubusercontent.com/openscad/openscad/master/src/openscad.cc)
and [COPYING](https://raw.githubusercontent.com/openscad/openscad/master/COPYING),
plus [OFFICIAL_RUNTIME_NOTICES.md](OFFICIAL_RUNTIME_NOTICES.md) for all pinned identities.

Language behavior, limits and unsupported features are defined by
[`src/core/languageContract.ts`](src/core/languageContract.ts) for the
independent subset and
[`src/core/openScad2021Contract.ts`](src/core/openScad2021Contract.ts) for the
stable profile. Observable semantic changes require contract version/revision
review and conformance fixtures. The immutable
`openscad://language/openscad-2021.01` MCP resource publishes that canonical
registry. Its summary in `openscad://capabilities`, `openscad_official_status`,
and official check/export results keeps the stable target separate from the
installed execution identity. Using a newer official runtime is not a claim of
byte-identical 2021.01 binary behavior.

Official SCAD input is mounted in a project-local MEMFS and cannot read or write
host paths. Node permissions limit host-file reads to the runner, patched
runtime, and pinned font, but they do not enforce a network sandbox or a
WebAssembly linear-memory ceiling. The runner exposes no network API to SCAD;
OS-level network and hostile-memory containment remain residual risks.

After explicit installation, use `npm run verify:openscad-runtime` for the
manifest/integrity check and `npm run test:official` for the real-runtime MCP
conformance suite, including an actual stdio server process. The MCP surface is
`openscad_official_status`, `openscad_official_check`, and
`openscad_official_export`, with status at `openscad://official-runtime` and
session-local export bytes at `openscad://official-artifacts/{sha256}`.

The executable qualification covers every one of the 38 stable functions and
35 stable modules, named multi-file/file-backed cases, and all seven advertised
export formats with artifact-integrity checks. It does not claim exhaustive
equivalence for every grammar/operator/modifier combination or for the
separately recorded deprecated compatibility tail.

The repository-owned lane separately exposes `openscad_independent_check` and
full-quality `openscad_independent_export`. Both accept the same bounded inline
source/VFS project and optional stable animation time (`0..1`), call no upstream
runtime, and retain the independent STL/OBJ result only at
`openscad://independent-artifacts/{sha256}` for the MCP session. These exports
do not enter the legacy DuckDB build schema, and their engine attestation stays
development/non-authoritative/incomplete until the independent qualification
gate is actually complete.

The security and routing decision is recorded in
[`docs/adr/0003-official-openscad-mcp-runtime.md`](docs/adr/0003-official-openscad-mcp-runtime.md).
