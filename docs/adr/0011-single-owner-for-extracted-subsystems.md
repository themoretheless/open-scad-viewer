# ADR 0011: One owner for each extracted subsystem

- Status: proposed
- Date: 2026-11-10
- Evidence: `docs/refactor/inventory/duplicates.md`, `docs/refactor/ARCHITECTURE.md` section 3

## Context

Several subsystems were extracted from `brep-core` into their own crates by
copying, and the originals stayed in place. Both copies are compiled, tested
and edited:

| Extracted crate | Share of its lines that also exist in `brep-core` | Lines |
| --- | ---: | ---: |
| `cad-step` | 70% | 9727 |
| `nurbs-intersect` | 90% | 8157 |
| `brep-intersect` | 66% | 5723 |

`geometry-bridge` still calls the `brep-core` copy of STEP 16 times and the
`cad-step` copy twice. `brep-intersect` itself calls `brep_core::intersections`
54 times. Recent STEP work changed both copies in the same commit.

Inside `brep-core`, nothing depends on the STEP and IGES modules except two
references from `healing` to `import_policy`. Intersection queries are needed by
ten kernel modules, so their owner has to sit below `brep-core`.
`nurbs-intersect` has no dependency on `brep-core`.

## Decision (proposed)

1. STEP and IGES are owned by `cad-step` (layer 4). `brep-core` loses its copies;
   `geometry-bridge` imports from `cad-step`.
2. Curve and surface intersection queries are owned by `nurbs-intersect`
   (layer 2). `brep-core::intersections` re-exports them during the transition.
3. Pair solvers for analytic surfaces move below `brep-core`, next to the
   queries. `brep-intersect` keeps only the `Model`-facing entry points.
4. An extraction is complete only when the old location is deleted or reduced
   to `pub use` in the same change.

Wire formats and the WASM ABI do not change. Rust paths such as
`brep_core::step_interchange_v3::*` do.

## Alternatives

- Keep both copies. Rejected: every change is made twice, and the copies have
  already diverged (the shared share is 66 to 90 percent, not 100).
- Fold the extracted crates back into `brep-core`. Rejected: it grows the
  largest crate and puts interchange formats back inside the kernel.

## Consequences

- About 23 thousand duplicated lines disappear and `brep-core` shrinks by about
  19 thousand lines of interchange code.
- The kernel artifact is expected to shrink; measure `geometry-kernel.wasm`
  before and after.
- Source bindings in the qualification records change and need the usual
  refresh.
