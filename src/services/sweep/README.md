# Loft, sweep and miter integration

The Rust `nurbs-core` kernel owns the rational loft/sweep geometry and predicate algorithms.
These TypeScript modules adapt kernel calls and assemble evidence for the same
constructor request:

- `construction/`: loft and sweep B-rep constructors, streaming, bounded section
  correction, and constructor-owned boundary certificates.
- `certificates/`: native audit adapters and complete-boundary evidence composition.
- `admission/`: recompute Solid evidence on the exact geometry being transferred.
- `viewport/`: validate the reported seam set before displaying smoothness claims.

`geometry/brep.ts` is the stable public facade. `geometry/brep/core.ts` contains
other B-rep operations and shared carrier types. Former flat service paths remain
compatibility re-exports. Internal sweep modules import their concrete owners;
they do not depend on those compatibility facades. Keep the constructor ownership
WeakMap in one module: copied or mutated geometry must not inherit a certificate.

Rust certificate entry points are grouped under
`sweeps::certificates`, `progressive_miter::law_certificates`, and
`progressive_miter::boundary_certificates`. Existing module paths remain supported.
These namespaces preserve each certificate's scope and result types.

## Qualification selection

`docs/design/sweep-qualification-catalog.json` owns the selected browser sources,
STEP filenames, smooth reconstruction modes, and native/WASM/Rush suite selection.
It is a finite coverage inventory, not a geometric proof or a cross-target parity
claim. Native and WASM suites cover families; they do not each execute every UI case.

Run `node scripts/run-sweep-qualification.mjs list` to inspect selection, or replace
`list` with `native`, `wasm`, `rush`, `browser`, `step`, or `step-smooth`.
Browser and STEP targets accept the existing export/output arguments. STEP targets
export fixtures; `step-smooth` also fills the volume reference using the independent
existing independent polynomial-generator oracle. OCCT checks still run through
`scripts/verify-sweep-step-occt.py` against the generated manifest. The STEP exporters
reject missing, extra, or duplicate catalog fixtures. Their construction recipes,
volume calculations, and geometric assertions stay outside the selection catalog.
Browser provenance includes both compatibility facades and implementation sources.
Historical qualification records remain unchanged.
