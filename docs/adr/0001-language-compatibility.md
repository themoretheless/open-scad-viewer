# ADR 0001: Versioned independent language subset

- Status: accepted
- Contract: `openscad-viewer-subset@1`

OpenSCAD Viewer implements an independently maintained, explicitly versioned
subset of the OpenSCAD language. It does not claim drop-in compatibility with
the official runtime. Unsupported syntax fails with positioned diagnostics;
silent approximation is not permitted.

The browser build does not embed or execute the official OpenSCAD runtime.
Geometry is evaluated through the separately packaged Manifold kernel behind a
project-owned kernel interface. Any future official-runtime integration is a
separate distribution/licensing decision and must not silently change subset
semantics.

Multi-file work will use the in-memory project VFS only. Paths are canonical,
POSIX-style, project-relative and budgeted; host paths, URLs and traversal above
the project root are rejected. `include` and `use` remain unsupported until
their scope/evaluation order, cycle diagnostics and conformance fixtures are
specified for a new contract version.

Compatibility changes require a contract-version bump, conformance fixtures
and migration notes. Additive implementation work within an already documented
feature may retain the version when existing valid programs keep their meaning.
