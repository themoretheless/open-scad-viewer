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

## Owned seam endpoint diagnostic

`source_seam_tangency::inspect_endpoint_limits` binds the diagnostic to a privately admitted original shared edge. Literal endpoint control identity selects the original U boundary. Total exact factor work is bounded across both sides; errors without a consumed-work report reserve the remaining budget. The routine returns diagnostics and never supplies a regular G1 certificate.

A replay of edges 0, 1, 16 and 18 returns `source-seam-endpoint-distinct-tangent-limits`, using 18,186,223; 18,233,929; 18,462,713; and 18,549,238 work units respectively, within the 100,000,000 global allowance per edge. Every result identifies the same collapsed original point and separated limiting planes as the initial chart inspection.

Six seam tests pass, including reversed carrier orientation, tiny budget exhaustion, invalid options, refusal for mapped/root-ended identity, and continued regular G1 rejection. Mapped carriers require separate endpoint identity qualification. WASM/worker/UI integration is still pending.

Correction to interpretation: zero-radius endings are deliberately constructed to return to the original sharp rim. Their differing limiting planes are inherent in that geometry, rather than evidence of a control-net implementation defect. A requirement for a smooth endpoint needs additional transition geometry or adjacent-edge treatment; relaxing the normal test is invalid.

## Bridge and interface integration

The optional `seamQualification.endpointLimitsExactWork` request adds a separate global native factor-work allowance in 1..100,000,000. Diagnostics are returned only for an unresolved literal original carrier endpoint. They remain additive to the regular seam refusal. The response is bound to the original request and canonical endpoint; validation rejects missing requested diagnostics, excess work, duplicate sides, invalid component/angle intervals and inconsistent separated-limit claims.

The interface requests the diagnostic on explicit edge qualification, explains zero-radius sharp endings, and displays original endpoint coordinates in millimetres. Existing generation, selection and document checks govern cancellation and stale responses. Controlled-port cancellation and late-response tests pass; they are not browser acceptance evidence.

The full native B-rep run passes: 941 unit tests, 3 ignored, plus every integration suite. The bridge dispatch test passes with the added request and invalid-budget rejection. 319 interface/transport tests pass; a subsequent transport run after tightening paired-normal bounds passes 2 tests. Vue and MCP typechecks pass. The new real-worker regression for all four endpoints has been added, but its execution and production package verification await the current WASM build.

Endpoint location admission also requires every reported collapsed point to lie in the native endpoint enclosure of the selected original carrier. The new real-worker regression for changing that point awaits the same WASM build. Transport/cancellation tests pass after this added consistency gate; Vue typechecking passes.
