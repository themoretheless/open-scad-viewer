# Native CAD feature extensions

These additions belong to the host Rust libraries. The frozen
`crates/independent-modeling/modeling-clean` comparison is unchanged.
No Plasticity, XNURBS or PolyhedralSplines source is included or linked.

Continuation: [freeform geometry and complete boundary constraints](cad-freeform-extensions-2026-10-31.md).

## Public entry points and admitted domains

| Module | Capability | Current boundary |
| --- | --- | --- |
| `nurbs_core::constrained_surface::solve` | Hard point, first derivative, tangent-plane and control coefficient constraints; positive-definite cage fairness objective; actual evaluated residuals and constraint rank | Fixed nonperiodic rational basis/weights; <=256 controls, <=384 constraints. This entry point applies pointwise constraints; complete G2 boundaries use the separate `boundary` API. |
| `nurbs_core::polyhedral_spline` | Retained cubic spline cage, immutable control edits, Bezier decomposition, G0 or parametric C1 boundary reconciliation | Rectangular regular quad cages >=4x4, <=256 controls. No extraordinary vertices, T-junctions or arbitrary subdivision topology. |
| `brep_core::feature_ops::full_round` | Replace the middle face between opposite support faces with a rational half-cylinder; radius comes from support separation | One axis-aligned cuboid with enough depth. General prism/freeform Full Round remains open. |
| `brep_core::feature_ops::reblend_faces` | Recognize a complete constant-radius chain, suppress it, reconstruct the sharp support intersection, author a new radius | Existing suppression and exact simple-prism authoring domains; planar supports. No variable-radius or general freeform reblend. |
| `brep_core::feature_ops::pattern_faces` | Build extrusion tools from trimmed planar faces and apply transformed instances through B-rep union/difference | Signed depth along outward normal; <=16 faces, <=32 placements, <=64 total instances. This patterns pad/pocket features, not arbitrary face-set history. |

All B-rep operations return a new model, validate it and attach the existing
solid ownership/incidence audit. That audit does **not** certify a globally
embedded, self-intersection-free boundary. Inputs remain unchanged on failure.
Unsupported domains return errors rather than a display-mesh approximation.

The continuation adds retained arbitrary quad geometry, full-boundary G2
constraints, freeform offset expressions, constrained B-rep replacement and
open fillet/chamfer sheets. General automatic closed-solid freeform editing
remains **open**; see the continuation for exact contracts. This is not
Plasticity parity.

## Verification

```sh
cargo test --manifest-path crates/Cargo.toml -p nurbs-core --test constrained_surface --test polyhedral_spline
cargo test --manifest-path crates/Cargo.toml -p brep-core --features codec --test feature_ops
python3 crates/independent-modeling/verify_clean.py
```

Surface tests check simultaneous/redundant constraints, conflicting constraints,
fixed nonuniform rational weights, bad inputs, retained geometry edits and
complete seam position/derivative sampling. B-rep tests check analytic round
geometry and volume, radius replacement, joined/cut pattern topology, invalid
inputs and unchanged source models. No STEP, WASM, UI or end-to-end qualification
is implied by these native tests.
