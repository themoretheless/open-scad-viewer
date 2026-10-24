# Contact rails and explicit normal intervals

Native work after `dea9cb22`, 2026-10-07.

## Implementation

`contact_normal_agreement::certify_interval` and
`edge_tangency::inspect_interval` inspect an explicit closed interval of the
normalized original world-edge parameter. Both original pcurves retain their
domains and coedge reversal. Original surfaces and curves are not cropped,
fitted or replaced. Position agreement is still checked over the entire edge.
Every report records `normal_interval`; excluded endpoints are not qualified.

The existing full-edge APIs delegate with interval [0,1].
`whole_edge_agreement_proven()` requires full normal coverage as well as
position and tangent-plane agreement. A successful subinterval cannot satisfy
this predicate. It remains diagnostic, not a solid or exact G1 certificate.

Tests cover nonunit original curve domains, reversed pcurves, invalid intervals,
singular endpoints with explicit pending partition, hidden interior angular
defects, sharp cube edges, pole refusal and exhausted normal work.

## Measured contact result

Fixture: outer radius 20, inner radius 5, height 6 mm; radii 0.5 → 1.25 mm;
angular spans [0.3,0.7,0.4]. All six original contact rails have whole-edge
position agreement at 1e-7 mm. Normal threshold is sin² <= 1e-4 (about 0.573°),
not exact G1. Model serialization is unchanged.

| Faces | Full normal interval | Explicit regular interval | Normal visits on regular interval |
| --- | --- | --- | --- |
| 0–1, entry/plane | unproven | [0.05,1] qualified | 29 |
| 0–2, entry/cylinder | unproven | [0,0.95] qualified | 1337 |
| 5–6, middle/plane | [0,1] qualified | complete edge | 17 |
| 5–7, middle/cylinder | [0,1] qualified | complete edge | 2047 |
| 10–11, exit/plane | unproven | [0,0.95] qualified | 41 |
| 10–12, exit/cylinder | unproven | [0.05,1] qualified | 3057 |

The excluded 5% is normalized edge parameter, not world arc length. These
four pole neighborhoods remain unqualified. Full pole-containing checks use
512 normal cells/1024 spans; regular checks use 5000/10000. Position agreement
uses 256 cells. Complete accepted and pending interval traces are retained.

## Scheduler experiment

Depth-first and breadth-first traversal were compared at identical inputs and
budgets. Breadth-first reduced unresolved parameter measure on faces 0–1 from
1 to 2.1010637283325195e-6, but increased it on faces 0–2 from 0.50390625 to 1
and on faces 10–11 from 0.002166748046875 to 0.003265380859375.
The breadth-first change was rejected. Production retains the original
depth-first traversal. `rails-bfs.json.gz` records the rejected experiment;
`rails.json.gz` and `rails-dfs.json.gz` record the retained result. A smaller
number of pending cells is not interpreted as a smaller unproven interval.

## Reproduction

```sh
cargo +nightly-2026-09-16 test --locked --manifest-path crates/Cargo.toml \
  -p nurbs-core --lib contact_normal_agreement::tests:: -- --nocapture
cargo +nightly-2026-09-16 test --locked --manifest-path crates/Cargo.toml \
  -p brep-core --lib edge_tangency::tests:: -- --nocapture
cargo +nightly-2026-09-16 run --locked --manifest-path crates/Cargo.toml \
  -p brep-core --example variable-annular-contact-rails -- /tmp/contact-rails.json
```

## Still open

One-sided pole-limit qualification, tighter angular tolerances, whole material
wall thickness, complete self-intersection classification, general NURBS fillet
editing and complex corners. This work does not reduce the previous 255 original
annular wall pairs. No new WASM/worker/UI package, independent STEP measurement,
named-part/history acceptance, P0 or P2–P3 completion is claimed.
