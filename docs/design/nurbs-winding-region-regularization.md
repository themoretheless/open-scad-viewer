# Exact original NURBS nonzero-fill region regularization

`brep_core::profile_certificate::regularize_nonzero_winding` is the UV-region
stage needed between periodic chart-chain assembly and owned surface faces.
It accepts separated simple represented planar NURBS wires with bounded signed
integer coefficients. It returns immutable `WindingRegularization` components,
each with one material-left outer and its holes. It does not author 3D faces.

## Admission and source ownership

The independent original profile geometry checker reconstructs every stored
homogeneous knot span using exact rational arithmetic. It checks complete
monotonicity/separator witnesses, orientation and nesting. Its internal
orientation-neutral entry point remains private. Ordinary public `certify` and
`verify` still require the existing alternating material-left orientation policy;
they do not accept an arbitrary winding input as an ordinary approved profile.

Exact winding queries on the certified simultaneous polygon homotopy establish
the containment forest. For each original wire, outside winding is the sum of
its containing wires' signed orientation times integer coefficient. Inside
winding adds this wire's contribution. Only a zero/nonzero transition is retained.
Nonzero/nonzero and zero/zero borders are removed. Each retained hole belongs to
the deepest retained containing outer. Negative filled winding is material too.
The result regularizes the nonzero-filled region, including its material
boundary; it does not preserve the original chain multiplicities or winding
number as an output face coefficient.

Output wires are original curves, or exact reversals using reversed controls,
reversed weights and negated reversed knots. Their parameter domains may change
from [a,b] to [-b,-a]; no rounded a+b-k reflection is used. No polyline fitting or
new geometric boundary is introduced. Every output component then passes the
ordinary independent material-left profile checker again.

The artifact owns its original wires, integer coefficients, geometry witness,
winding claims, source-index/reversal bindings and component witnesses.
`verify()` replays these without witness search and rejects stale source,
tolerance, coefficient, winding, removed-border, owner and output claims. Read-only
accessors expose the source, original winding pairs and component provenance.

## Limits and remaining work

Original owner limits remain: 0–32 wires, at most 128 source spans and 256 exact
proof pieces, degree 1–8, positive bounded weights and bounded source encoding.
Each integer coefficient has absolute value at most 1,000,000; sums fit i64.
Zero coefficients and the empty chain produce empty regions only after required
source geometry validation. Input wire crossings, coincidences and contacts
between distinct wires remain unresolved. Exact opposite cancellation belongs
to the earlier periodic weighted-chain stage. Common chart-perimeter contacts,
full owned-face assembly on the supporting surface, world boundary mapping and
seam topology remain separate obligations. NURBS-0173 remains planned and the
catalog remains **195/200**.

## Verification

**45 Rust checks pass**: 24 profile certificate unit/replay checks, 5 new region
integration checks, 6 planar-cap checks, 6 finite thread checks and 4 gear profile
checks. New cases include redundant same-orientation loops, signed multiplicity,
holes and material islands, independent rectangle-grid fill observations,
compact original rational ellipses, cubic curves on non-dyadic domains, exact
reversal, immutable source ownership, stale-claim refusal, zero chains and
crossing/touching/coincident source refusal. Both crates compile for WASM.
[Evidence](../qualification/nurbs-winding-regularization-2026-10-15/native-tests.json).
