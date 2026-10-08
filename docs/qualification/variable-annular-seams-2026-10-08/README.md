# Original annular contact seam qualification

The four variable-radius control bodies were inspected with their original
world edges, surfaces and pcurves. Geometry is not fitted or changed.

## Initial run

40 checks: all 40 prove whole-edge positional agreement at 1e-7 mm.
22 prove tangent-plane agreement on their explicitly recorded normal interval
at max sine squared 1e-4. This is an angular tolerance certificate, not exact G1.

- Middle support rails (edges 15 and 17): both full intervals pass on all four bodies.
- Four endpoint-touching rails per body: full intervals remain unresolved.
- Excluding an explicit 5% endpoint neighborhood: 14 of 16 checks pass.
- Decreasing radius, edge 27, interval [0.05,1]: two directional cases exhaust
  the 5000-cell budget. This is unresolved, not proof of a geometric defect.

Every omitted endpoint neighborhood remains unqualified. Full shell tangency,
pole limit normals, thickness, general fillets and named-part acceptance remain
open. The separate complete self-intersection audit is not a G1 certificate.

`summary.json` lists every interval and outcome. `contact-rails.json.gz` retains
all cell bounds and unresolved reasons from the initial run.

## Reproduce

```sh
cargo +nightly-2026-09-16 run --locked --manifest-path crates/Cargo.toml \
  -p brep-core --example variable-annular-contact-rails -- /tmp/contact.json
```

Optional second argument sets regular-cell budget; third selects specimen
indices separated by commas. Indices 0/1 use increasing radius, 2/3 decreasing;
even indices use positive sweeps, odd indices negative sweeps. Singular full
intervals retain a separate 512-cell budget and do not gain endpoint proof
from a regular subinterval.

## Source seam regression

Five native source-seam tests pass: original rational root ends, affine/reversed
carrier correspondence, rotating tangent planes, crease rejection and singular
endpoint refusal. See `source-seam-tests.log.gz`. These tests do not replace
the specimen-specific interval checks above.

## Increased-budget result

The two decreasing-radius specimens were repeated at 10000 regular cells.
Both edge-27 [0.05,1] intervals pass after 7005 cells. Combined with the
initial run, all 16 explicitly bounded regular intervals pass. Full
endpoint-touching intervals still remain unresolved; raising regular work
never changes their separate singular-endpoint status.

```sh
cargo +nightly-2026-09-16 run --locked --manifest-path crates/Cargo.toml \
  -p brep-core --example variable-annular-contact-rails -- /tmp/decreasing.json 10000 2,3
```

`contact-rails-decreasing.json.gz` contains the complete repeat. The initial
summary is preserved as initial evidence rather than overwritten.

## Exact pole obstruction

All eight original collapsed support ends have different limiting tangent
planes along their two meridian boundary paths. This is a geometric obstruction
to a unique regular tangent plane at the pole, not merely exhausted work.

The native exact projected-Jacobian predicates prove every component's
Bernstein coefficient rows 0 and 1 zero. At row 2, v=0 has exactly zero X/Y
normal numerator and nonzero Z. At v=1, X is nonzero. The order-two leading
vectors are therefore nonparallel. Positive rational weights preserve their
normal directions; higher orders vanish relative to the leading u² term.
Reversing the U chart at the exit changes orientation but preserves the plane.
Both paths approach the same original collapsed point. A unique regular pole
plane would have to equal both different limits, which is impossible.

`pole-normal-orders.json.gz` retains exact coefficient signs and work counts
for both radius orders and arc directions. Native assertions reject missing
proof, nonzero lower orders or absence of the nonparallel witness.

```sh
cargo +nightly-2026-09-16 run --locked --manifest-path crates/Cargo.toml \
  -p brep-core --example variable-annular-pole-orders -- /tmp/pole-orders.json
```

The current zero-radius taper must remain explicitly singular. A regular smooth
termination requires a different end construction and renewed radius, contact,
closure and self-intersection qualification. This does not disprove one-sided
seam tangency against each support separately; those full-seam angular bounds
remain an independent requirement.

## Whole plane rail and one-sided endpoint normal

All eight original plane-side rails now have exact tangent-plane agreement
for the complete regular interval u in (0,1], and an axial one-sided normal
limit at u=0. This closes the previously omitted plane-rail neighborhoods.
The collapsed surface point itself remains nonregular.

Every projected normal numerator coefficient for X/Y on v=0 is exactly zero.
Z rows 0/1 are exactly zero; every remaining Bernstein coefficient has the
same strict sign. Positive Bernstein bases prove a nonzero axial normal for
all u>0, with its same direction in the limit. The incident original support
face is checked to have every control Z=6. These assertions inspect the
original definitions, including rounding, rather than ideal conic formulas.

The new exact predicate optionally returns outward coefficient enclosures
in the same positive scale across projections. Existing sign-only calls retain
their original work budget. Seven focused predicate tests pass, including
known-plane component ratios and resource exhaustion refusal.

`pole-normal-bounds.json.gz` records all original plane-rail coefficient rows
and corner leading magnitudes. Cylinder-side full angular bounds remain open;
these results do not supply a unique regular tangent plane at the collapsed
end, a replacement smooth cap, thickness or general source-body admission.

## Whole cylinder-side rail angular bounds

All twelve original cylinder-side rails pass over their complete normalized
parameter interval, including one-sided limits at the collapsed ends. Maximum
accepted sine-squared bound is 9.573636078228842e-5 against 1e-4 tolerance.
Entry/middle/exit require 79/255/157 visited cells respectively. There are no
unresolved or omitted parameter regions.

Both incident original rational boundary columns have identical Euclidean
controls and weights. Each is a single clamped Bezier chart; normalized U
correspondence is exact even when their authored knot domains differ (e.g.
[0,1] versus [0,3]). No source curve or surface is refitted.

Exact normal numerator rows 0/1 vanish at the collapsed end. The U² factor is
removed only after exact-zero predicates pass. Bernstein factor coefficients
are enclosed with outward arithmetic. Both original normal polynomials are
subdivided on matching dyadic parameter intervals; cross-product and norm
bounds prove the angular tolerance on every accepted interval. A separate
positive normalization per surface preserves direction and controls scale.
The pole itself is not regular; this proves one-sided rail agreement, not a
unique pole plane or a smooth cap.

Two numerical-regression tests reject orthogonal and singular normals, accept
antiparallel planes, and preserve shared subdivision midpoint enclosures.
`cylinder-rails.json.gz` records all accepted intervals and upper bounds.

```sh
cargo +nightly-2026-09-16 test --locked --manifest-path crates/Cargo.toml \
  -p brep-core --example variable-annular-cylinder-rails
cargo +nightly-2026-09-16 run --locked --manifest-path crates/Cargo.toml \
  -p brep-core --example variable-annular-cylinder-rails -- /tmp/cylinder-rails.json
```

Replacement smooth end construction, global thickness, support-patch junctions,
general fillets and named-part acceptance remain open.
