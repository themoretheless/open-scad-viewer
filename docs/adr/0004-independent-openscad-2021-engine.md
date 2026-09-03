# ADR 0004: Repository-owned OpenSCAD 2021.01 engine

- Status: Accepted, implementation under qualification
- Date: 2026-09-03

## Context

The product requires its own OpenSCAD engine rather than a simplified subset or
an upstream OpenSCAD process hidden behind MCP. The existing subset remains a
frozen compatibility route, and the optional official runtime remains useful as
an oracle, but neither can stand in for the new production implementation.

## Decision

`openscad/stable-2021.01` is implemented by repository-owned parsing,
evaluation, project-file handling and geometry orchestration. A third-party
geometry kernel may execute bounded geometric primitives and booleans, but it
does not own OpenSCAD syntax, scoping, values, modules, file semantics, routing
or MCP behavior.

The independent MCP result must attest that no upstream runtime was used. The
official provider is callable only through explicit `openscad_official_*`
oracle tools and cannot be selected as a fallback. The frozen
`openscad-viewer-subset@1` route keeps its existing meaning and error contract;
the stable profile receives a new versioned engine identity.

The implementation must not claim completeness while any required syntax,
operator, modifier, built-in module, file format, scoping rule or error behavior
is missing or intentionally approximate. Development results therefore carry
`complete_language_claim: false`.

## Qualification gate

Production promotion requires all of the following through the independent MCP
route with the upstream provider unavailable or instrumented to prove zero
calls:

1. executable cases for all 38 stable functions and all 35 stable modules;
2. the canonical grammar, operators, modifiers, recursion, `undef`, lexical
   scope and argument-binding semantics;
3. bounded project VFS behavior for `include`, `use`, `import`, `surface`, text
   fonts and supported legacy assets;
4. real geometry checks, not parse-only acceptance, for 2D and 3D operations;
5. deterministic identities, diagnostics, cancellation and resource budgets;
6. browser/worker and MCP parity for the same independent engine contract;
7. differential oracle tests where useful, with differences reviewed rather
   than silently delegated.

Only after this gate passes may the engine identity lose its development suffix
and advertise `complete_language_claim: true`.

## Consequences

The upstream runtime is no longer a prerequisite for product execution. It
remains separately installable for qualification and reference artifacts under
its own license and security boundary. Missing independent behavior is a
release blocker, not a reason to fall back to upstream OpenSCAD.
