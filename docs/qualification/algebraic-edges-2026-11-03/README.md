# Exact branch strata and algebraic-edge B-rep — 2026-11-03

This extends the preceding algebraic Boolean qualification. It is a bounded
native Rust extension, not completion of arbitrary NURBS/NURBS volumetric Boolean.

## Implemented

`profile_certificate::certify_algebraic_branches` constructs the complete real
zero-set strata of one exact rational bivariate polynomial on the closed unit
square: critical-fiber vertices, open slab sections, vertical arcs, connected
components, reduced-locus singular flags, edge multiplicities and counterclockwise
incident ports. Coefficients are v-row/u-column power basis. Tensor degree is
bounded by 16 and all existing exact arithmetic/projection budgets remain active.
The verifier replays the construction and compares the full source-bound receipt.
Dependent algebraic zero tests use modular gcd, exact field-factor selection and
root isolation; no floating root matching is used. Singular vertices terminate
regular strata: this does not claim Puiseux analytic continuation pairings.

`certify_surface_algebraic_branches` binds that complete graph to the exact height
numerator of two original rational NURBS graph carriers with affine planar
projection. Original carrier hashes and normalized parameter maps are verified.
The whole first square must lie inside the second square; partial-footprint
arrangements and general non-graph NURBS carriers explicitly refuse.

`AlgebraicModel` owns actual algebraic vertex, edge and pcurve payloads in the
generic B-rep topology. Original NURBS caps are retained. It contains no proxy
rational curves and has no implicit conversion to the legacy rational `Model`.
Construction replays the region proof and uses the newly verified owner; closed
shell incidence is checked. Exact rational parameter queries return algebraic
isolators. Queries on unsupported critical vertical fibers refuse.

Tagged `ExactModel` persistence supports legacy NURBS models, graph slabs and
algebraic models. Loading reconstructs proof/topology from source carriers and
compares the entire serialized topology. Merely relabeling an algebraic object
as a NURBS model or modifying an edge selector is rejected.

`operations::boolean_exact` dispatches the typed models. The nontrivial new case
is intersection of two independent ordered graph slabs over triangular
footprints. Exact envelope order and footprint inclusion are proved; all
positive intersection components must be strictly interior, with regular cap
seams. The result is an algebraic-edge B-rep. Swapped input order is supported.
The Boolean receipt binds all four original caps, operation, envelope order,
witness and result. Identity operations on the same exact model are supported;
legacy NURBS/NURBS inputs delegate to the existing Boolean.

The quartic test retains a Fermat-type algebraic seam with irrational
fourth-root fiber parameters. It is not a fitted circle or rational substitute.
Keeping the complete exact height numerator separate from triangle boundary
factors avoids an unnecessary projection-budget failure; the existing checker
verifies the factor product and work limits were not increased.

## Explicit remaining work

- Non-graph NURBS/NURBS intersection atlas in four source parameters, arbitrary
  trimmed/multispan source overlays and coincident-face cancellation.
- Analytic germ pairing at general singularities, and admission/sewing of
  singular or pinched solid boundaries. The regularity gate remains mandatory.
- Boundary-reaching lens regions requiring ruled side caps, arbitrary envelope
  changes, and distinct-model union/difference/XOR for the new algebraic type.
- Repeated general editing/Boolean of algebraic results.
- Migration of legacy `operations::boolean`, editor transport, tessellation,
  export and UI consumers to the typed exact model. The new native entry point
  is `boolean_exact`; the old entry point does not return algebraic edges.
- WASM/dist/browser/STEP and full-workspace qualification.

Exhausted budgets and unsupported cases remain explicit unproved results.

## Validation

32 distinct tests passed: 12 new branch/model tests, 2 Bézier arrangements,
3 previous standard Boolean tests, 6 cap-region tests, 7 branch-decomposition
tests and 2 algebraic-event tests. All 12 new tests also passed in the live shared
checkout (replays, not extra distinct tests). The 12 changed source/module files
match the frozen snapshot byte for byte. API-map and whitespace checks pass.

See adjacent source hashes, validation summary and full logs. Builds use separate
frozen and live targets, debug assertions enabled, brep-core/nurbs-core opt-level 0
and dependencies opt-level 1. These are correctness checks, not performance data.
The frozen source derives from the previous immutable qualification snapshot
plus this task's files; unrelated shared-checkout work is exercised separately.

An exploratory `--no-default-features` check on that inherited snapshot failed:
its old `minkowski3d::curved` module is codec-gated although existing Boolean
modules and the new exact model import it unconditionally. No no-codec support
is claimed by this qualification; see `no-codec.log` for the failure.
