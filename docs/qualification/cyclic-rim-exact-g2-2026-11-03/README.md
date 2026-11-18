# Cyclic rim construction and exact original G2 coefficients — 2026-11-03

Native Rust construction/qualification progress only. No closed-body, whole-field
10/mm curvature, exterior-contact, volume, embedding, STEP or WASM/UI acceptance.

## Same owned outer boundary, different interior construction

`propose_trihedral_center_at_rim` selects cyclic origins 0/2/4 of the SAME six
original profile/connector curves. This changes the interior rectangular
construction; it is NOT an image identity for the previous central surface.
Nominal radii, source supports, runout and source boundary curve definitions are
unchanged. All source/support positional checks are fresh. The default API keeps
origin 0; a regression checks its literal surface equality with the old entry.
Reverification reconstructs the actual selected origin instead of silently using 0.

## Full Bernstein orientation transport

A new G0 fallback mirrors both world curve and pcurve poles/weights while keeping
original natural endpoint knots. It is limited to full, nonperiodic, clamped
single Bernstein spans. Exact normalized-clock reversal preserves the complete
correspondence-distance supremum, including an already reversed coedge flag.
No rounded extracted or fitted source is substituted. Only a successful whole
cover is transported back. A negative witness is not blindly rounded to 1-t.
Both attempts share the original cell allowance; partial traversals and missing
budget cannot inherit a result. This fixes the actual alternate quartic rim's
previously unresolved boundary-3 position check.

## Bounded adjacent-float proposal

If nearest-rounded P1 forces P2 outside the UNCHANGED L1 control-motion budget,
a finite proposal search considers at most three neighbouring floats in each
direction, recomputes P2 from the actually stored P1, and keeps an admissible
choice closest to the target first jet. Extra trials are charged. Roots, weights,
clocks and external tolerances stay fixed. Fitting a control-motion budget grants
NO G2 certificate: every result must pass the independent original field check.
Origin 2 illustrates this distinction: it constructs, but one internal G2 join is
actually rejected and another is unresolved at the reference handle.

## Exact original boundary coefficient preparation

For the strict common-V-denominator clamped-cell class, original float64 poles,
weights and knot widths become EXACT binary rational inputs. Original U/UU/UV
coefficients and the shared V/VV fields are computed from them; the two source
jet polynomials are subtracted BEFORE outward conversion. Every conversion is
checked against its exact rational fraction. Near-zero source differences are
never snapped to zero.

Preparation has at most min(128*shape_cell_allowance,1000000) charged rational
operations and 8192 bits per coefficient. Missing/exhausted preparation falls
back to the original interval route. Shape queries retain the original cell
budget. Preparation alone grants no geometric authority. Tests prove exact
cancellation of large source terms and reject a real 1e-12 source perturbation.

## Verified internal G2 result

The unequal-radius quartic fixture remains radii 0.1/0.2/0.35 mm, stock 10 mm,
runout 0.1 mm, span 2 mm, positional/normal/tensor tolerances 1e-7 mm, 1e-5,
1e-4/mm. At cyclic origin 1, all FOUR complete original internal seams are
proven regular and WithinTolerance without increasing these tolerances.
This is now a dedicated actual-geometry regression, not just a planar fixture.

At handle 1/256 mm, the four seam proofs use 1212, 726, 1164 and 2106 shape cells.
At handle 1/32 mm, they use 720, 450, 468 and 582. There is no inherited claim
about the six ORIGINAL exterior contacts or the rest of the patch domains.

## Why interior-only search cannot finish the cap

Four larger handles were checked with the same origin/radii/tolerances. All
constructible candidates still have point curvature witnesses above 10/mm.
At handle 1/32, the largest checked INTERIOR-grid witness lower is
20.605252148487693/mm. Additional ORIGINAL points directly on internal seams
prove lower bounds 11.833735491370867, 15.830636291232329,
15.392129481932633 and 23.65372057329217/mm (one certified side per seam).

The existing interior search freezes the first/last three U columns. These
columns determine each seam's entire source two-jet. Consequently these cap
violations cannot be removed by changing only interior poles. More iterations
of that same search would preserve the obstruction.

Next construction must jointly move shared INTERNAL seam curves and their
neighbouring cross-jets, while preserving owned seam endpoints and all EXTERNAL
boundary two-jets. Shared curve columns must remain literal on both neighbours;
G2, whole-field cap, source agreement, material/volume and embedding must then be
reverified before trimming and body assembly. The 10/mm cap remains unchanged.
A zero/absent point lower bound is not a zero curvature or a cap certificate.

## Scope of checks

58 B-rep region regressions, 49 original curve/surface-agreement regressions and
47 original-surface G2 regressions passed. All three cyclic origins retain full
original boundary positions in both legacy and quartic fixtures, and reverify
their own construction. The selected origin-1 network proves all four internal
G2 joins. The same does not hold for every optional chart.

Native diagnostics exit 0 when reporting finishes, even on rejection. Existing
`sew_corner` still targets the old single-surface proposal; its refusal is NOT
an assembly check of the new five-patch network. No new body was published.

## Reproduce

```sh
cargo test --offline --manifest-path crates/Cargo.toml -p nurbs-core --lib curve_surface_agreement:: --config 'profile.dev.opt-level=2' --config 'profile.dev.debug=0' --target-dir /tmp/osv-sewn-curvature-20261102
cargo test --offline --manifest-path crates/Cargo.toml -p nurbs-core --lib surface_g2_pcurves:: --config 'profile.dev.opt-level=2' --config 'profile.dev.debug=0' --target-dir /tmp/osv-sewn-curvature-20261102
cargo test --offline --manifest-path crates/Cargo.toml -p brep-core --lib region_replacement:: --config 'profile.dev.opt-level=2' --config 'profile.dev.debug=0' --target-dir /tmp/osv-sewn-curvature-20261102
cargo run --offline --manifest-path crates/Cargo.toml -p brep-core --example trihedral_corner_curvature --config 'profile.dev.opt-level=2' --config 'profile.dev.debug=0' --target-dir /tmp/osv-sewn-curvature-20261102 -- --quartic-profile --rim-origin=1 0.03125
```
