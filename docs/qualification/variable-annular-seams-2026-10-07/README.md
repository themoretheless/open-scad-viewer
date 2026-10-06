# Whole-edge tangency diagnostics for unequal-radius transitions

Native work after `10b523b0`, 2026-10-07.

## Implementation

`edge_tangency::inspect` obtains both incident faces from the original model,
including hole loops. It follows the original shared world curve and both
original pcurves with each coedge's actual reversal. The existing native
contact-normal qualifier bounds positions and tangent-plane agreement over
the complete parameter interval, including all relevant original knot spans.

Collapsed edges, missing edges and edges without exactly two distinct incident
faces are refused. This is diagnostic output, not a source-body admission
certificate. It does not prove shell orientation or exact G1 continuity.

Tests confirm a sharp cube edge has matching positions and a proved angular
break. A collapsed annular pole is refused; a one-cell/two-span normal budget
does not qualify a regular unequal-radius seam.
All three tests passed (7.51 s). The positive regression verifies complete
closed interval coverage and unchanged model serialization.

## Full seam result

All eight inter-transition seams (two per specimen, two radius orders, both
angular directions) have whole-interval position agreement at 1e-7 mm and
tangent-plane bounds at sin² <= 1e-4. This angular bound corresponds to at most
about 0.573 degrees; it is not exact G1. Every original interval is covered
without pending cells or proved angular breaks. Visits range from 1373 to
5307, normal spans from 2746 to 10614. The recorded budgets are 10000 cells
and 20000 normal spans per seam. Complete interval traces are retained in
`seams.json.gz`; these bounds cover inter-transition meridians, not poles or
plane/cylinder contact rails.
All four generated model definitions and STEP files are byte-identical to the
previous radius-qualified fixtures. An independent trace check verified exact
interval adjacency from zero to one and bounded accepted angles for all seams.

## Bounded self-intersection result

Both positive-direction unequal-radius specimens (0.5 → 1.25 and 1.25 → 0.5 mm)
have 28/31 face injectivity proofs. At the recorded budget, 163/465 distinct
face pairs were visited: 106 disjoint, 11 shared-boundary and 46 unresolved.
The remaining 302 pairs were not visited. Absence of self-intersections is
explicitly unproven. The original model serialization is unchanged after
inspection. These counts concern new 31-face specimens and do not replace the
previous original annular wall qualification's 255 unconfirmed pairs.

## Reproduction

```sh
cargo +nightly-2026-09-16 test --locked --manifest-path crates/Cargo.toml \
  -p brep-core --lib edge_tangency::tests:: -- --nocapture
cargo +nightly-2026-09-16 run --locked --manifest-path crates/Cargo.toml \
  -p brep-core --example variable-annular-transitions -- /tmp/annular-seams diagnostics
cargo +nightly-2026-09-16 run --locked --manifest-path crates/Cargo.toml \
  -p brep-core --example variable-annular-defects -- /tmp/annular-defects.json
```

## Scope still open

Whole-wall material thickness, complete self-intersection classification,
pole-limit G1, contact-rail tangency, general source NURBS fillet editing and
complex corners remain unqualified. No new WASM/worker/UI package or
independent STEP measurements are claimed. Named-part acceptance, P0 and
P2–P3 remain open. Only qualified cases may proceed to product admission.
