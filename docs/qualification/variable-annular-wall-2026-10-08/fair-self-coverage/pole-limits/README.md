# Original collapsed boundary limits

Native source inspection proves two distinct tangent-plane limits at each of two collapsed original boundaries. Every Euclidean control point on each clamped rational boundary is exactly equal, so the boundary maps to one original point. The exact U² normal numerator factor is retained. Interval normal bounds at the two original V endpoints exclude zero; their squared-sine angle has a strictly positive lower bound.

| Original face | Collapsed point (mm) | Squared-sine interval | Affected shared edges |
| --- | --- | --- | --- |
| 0 | (20, 0, 6) | [0.9999999999999353, 1] | 0, 1 |
| 10 | (0, 20, 6) | [0.9999999999999603, 1] | 16, 18 |

These are differing one-sided limits at the same point, so a unique regular tangent plane cannot be assigned there. The four regular endpoint seam refusals remain required. Increasing seam budgets cannot remove this original geometry defect. Repair requires changing the transition geometry or its retained endpoint structure and requalifying its complete seams and closed body.

The bounds are native diagnostics, not regular normals, new seam admissions, whole-wall certificates or proof of absence of self-intersections. Other collapsed charts and the interiors of general NURBS boundaries remain outside this endpoint comparison.

A conservative interval de Casteljau fallback also fixes subnormal parameter restriction: when normalized upper endpoints cannot safely invert, it encloses the complete original interval directly. It preserves the regular pole refusal and never alters original geometry.

Validation: 726 NURBS unit tests passed, 2 ignored; 2 integration tests and 1 doctest passed. Regressions cover two distinct limits at one collapsed point, different limits at different points (which must not assert pole divergence), constant limiting planes, reversed U orientation, and subnormal intervals with an independent polynomial normal formula.

Replay:

```sh
cargo run --release --locked --manifest-path crates/Cargo.toml -p geometry-bridge --example source-gap-uncertainty -- /tmp/gap-original-request.json /tmp/gap-native-inventory.json /tmp/pole-divergence-native.json seams-limits
```

The replay inputs are the existing restored annular request and native coverage inventory. Detailed output and logs are archived beside this note. Production WASM SHA cdf42a4f5d788b945e10b7a83210d820a595850830ed37b8e58570e42935b1cc predates this native change; these new diagnostics are not yet wired to worker or UI.
