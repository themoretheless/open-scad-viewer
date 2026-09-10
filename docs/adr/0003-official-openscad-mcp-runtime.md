# ADR 0003: Official OpenSCAD runtime as a qualification oracle

- Status: Superseded for production execution by ADR 0004; retained as an oracle
- Date: 2026-09-03

## Context

The browser compiler and the existing `openscad_check`, `openscad_analyze`, and
`openscad_export` tools implement the independent
`openscad-viewer-subset@1` contract. Extending that handwritten evaluator one
built-in at a time would still leave observable gaps in scoping, `undef`, file
resolution, text shaping, imports, modifiers, and error behaviour.

The stable compatibility baseline is OpenSCAD 2021.01, tag
`openscad-2021.01`, commit
`41f58fe57c03457a3a8b4dc541ef5654ec3e8c78`. It contains 38 non-deprecated
built-in value functions and 35 non-deprecated built-in modules. The exact
inventory is kept in the repository as a machine-readable contract and is
exercised through MCP.

The current geometry router is intentionally frozen to the independent
Manifold and reserved B-rep engine classes. An official OpenSCAD result cannot
truthfully be recorded as either of those engines, and official artifacts do
not carry the viewer's triangle-level source provenance.

The official OpenSCAD implementation is GPL-2.0-or-later. The browser bundle
and npm package must not silently acquire or redistribute that runtime.

## Decision

Upstream execution is exposed by a separate official-runtime MCP provider for
differential qualification and reference exports. It does not change the
meaning or route of the existing subset or independent full-profile tools, and
it is never a production fallback. Production stable-target execution is owned
by the repository implementation described in ADR 0004.

The provider uses the official `OpenSCAD-2026.09.01-WebAssembly-node.zip`
snapshot, pinned by archive SHA-256
`82054dfb4911686de0ee3ea36771dbf81f3d014c3460c8ea069ab4f933f6d888`.
This newer official runtime executes the stable 2021.01 surface; snapshot-only
experimental features remain disabled unless a caller explicitly enables a
published, bounded feature name.

Installation is explicit. A setup command downloads the archive from the
official OpenSCAD snapshot host, verifies the digest before extraction, applies
a deterministic local MEMFS isolation patch, verifies the pinned digest of that
patched runtime independently of the writable manifest, and writes the result
under the gitignored `.open-scad-runtime/` cache. Neither the archive nor the
patched GPL runtime is committed or included in the browser build. The same
command
downloads the checksum-pinned OFL Basic Regular font and its license text; all
three installed files are bound into the verified manifest. Exact identities
and redistribution obligations are recorded in
[OFFICIAL_RUNTIME_NOTICES.md](../OFFICIAL_RUNTIME_NOTICES.md).

Each request executes in a fresh Node subprocess with:

- an injected Emscripten `Module` and NODERAWFS disabled by the runner;
- Node's permission model enabled;
- a canonical, bounded in-memory project bundle rather than host paths;
- no inherited application environment (`NODE_NO_WARNINGS` is the only child
  process environment entry);
- bounded file count, source/assets, arguments, logs, output, and wall time;
- one active official job with fail-fast busy rejection rather than an internal
  job queue;
- cancellation followed by hard process termination;
- a verified runtime manifest plus patched-runtime, font, and font-license
  digests before execution.

The public request supplies source mounted as `/project/main.scad` plus relative
text or base64 binary files. Project-file inputs reject absolute paths,
traversal, backslashes, duplicate/reserved paths, and symlink references. The
runner exposes no URL-fetch or other network API to SCAD and performs no
arbitrary host-library search. This is the file boundary for `include`, `use`,
`import`, `surface`, and project-supplied fonts.

Node permissions restrict host-file reads to the runner, patched runtime, and
pinned font, and deny host-file writes. They do not enforce a network sandbox.
The Node V8 old-space setting does not bound WebAssembly linear memory either.
The one-shot process, bounded wire data, deadline, and hard kill limit exposure,
but OS-level network isolation and hostile-memory containment remain residual
risks rather than claimed guarantees.

Official exports are content-addressed and exposed as bounded, session-local
MCP resources. They are not written into the legacy DuckDB build/artifact
tables because those tables attest the frozen independent geometry execution
descriptor. Persisting complete project snapshots and official provenance can
be introduced later with a new storage contract.

The public MCP surface is deliberately small:

- `openscad_official_status`, `openscad_official_check`, and
  `openscad_official_export` are the three official tools;
- `openscad://official-runtime` reports the exact execution version, archive,
  runtime, patch, font and license identities, availability, limits, and honest
  isolation flags together with the stable-language summary;
- `openscad://language/openscad-2021.01` publishes the immutable canonical
  contract, while `openscad://capabilities` includes its names/counts/target
  revision summary and resource URI;
- `openscad://official-artifacts/{sha256}` exposes bounded export bytes for the
  current server session.

The repository's versioned `openscad/stable-2021.01` registry owns the stable
compatibility inventory; MCP resources expose that same canonical data rather
than reconstructing it from the executable. The language summary and runtime
capabilities keep the 2021.01 target distinct from the exact 2026.09.01
execution snapshot. Successful check/export payloads include the same contract
summary.

## Consequences

The complete stable 2021.01 target can be exercised through MCP by the upstream
oracle. That proves the oracle and fixtures, not the completeness of the
repository-owned engine. Independent completeness requires the separate gate
in ADR 0004. Deprecated compatibility aliases remain recorded outside the
38 + 35 qualification gate.

Clients may explicitly install the oracle to compare observable behavior or
produce a reference export. Normal independent execution must not require the
oracle installation.

The execution runtime is a newer official snapshot than the stable inventory.
Therefore conformance claims name both versions; they never describe snapshot
behaviour as a byte-for-byte reproduction of the 2021.01 binary.

The provider is fail-closed when the runtime, font, license, manifest, digests,
patch, or Node permission-model precondition cannot be verified. It never falls
back to the independent subset.

## Qualification gate

Release qualification requires:

1. exact, unique registry entries for all 38 stable functions and 35 stable
   modules;
2. team-authored executable smoke coverage for every registry entry;
3. multi-file and binary-asset cases covering `include`, `use`, `import`,
   `surface`, and `text`;
4. all seven advertised export formats with canonical MIME type, file name,
   resource bytes, and digest checks;
5. hostile bundle tests for traversal, absolute paths, duplicate files,
   malformed base64, aggregate budgets, deadline, cancellation, and oversized
   output;
6. schema-valid MCP success and failure results plus resource reads;
7. a real stdio MCP run against the checksum-verified official runtime; and
8. the normal repository quality gate.

`npm run test:official` first verifies the installed cache and then executes the
38 + 35 stable smoke programs, multi-file/binary cases, and all advertised
export-format integrity checks through the MCP facade, followed by the real
stdio status/check/export/resource/clean-shutdown case in item 7. `npm run
check` covers the normal protocol, hostile-input, type, build, and test gate.

This executable gate proves one successful MCP smoke program for each stable
built-in and the named integration cases. It is not an exhaustive equivalence
proof for every grammar/operator/modifier combination or for the separately
recorded deprecated compatibility tail.

The official sources of truth are the
[2021.01 release](https://github.com/openscad/openscad/releases/tag/openscad-2021.01),
[2021.01 cheat sheet](https://files.openscad.org/documentation/manual/CheatSheet.html),
and [official snapshot index](https://files.openscad.org/snapshots/).
