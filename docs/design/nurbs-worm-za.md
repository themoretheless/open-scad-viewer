# Original-spec cylindrical ZA worm working surfaces

Rust API: `nurbs_core::worm::patches_certified`, also under
`nurbs_core::domains::curves::worm`. NURBS-0087 remains planned until finite-end
clipping, caps and an owned native body are qualified. This is a separate axial
trapezoid construction, not an alias of the truncated 60-degree thread.

The original source inputs are axial module m, diameter quotient q, axial
pressure angle alpha in radians, dedendum factor k, axial backlash b, start
count n, turn count, hand and world-error budget. Reference conventions for
axial pitch and diameter quotient: [KHK worm geometry](https://khkgears.net/gear-knowledge/gear-technical-reference/calculation-gear-dimensions/).
This implementation is an explicitly axial straight-profile ZA surface;
it does not claim KHK Type III, normal-plane input conversion, a manufactured
cutter envelope, conjugate wheel contact or strength qualification.

```
pitch radius = q*m/2
crest radius = pitch radius + m
root radius = pitch radius - k*m
axial pitch p = pi*m
lead L = n*p
lead angle = atan(n/q)
crest width wc = p/2 - b - 2*m*tan(alpha)
root width wr = p/2 - b + 2*k*m*tan(alpha)
profile Z = [0, wc/2, wr/2, p-wr/2, p-wc/2, p]
profile R = [crest, crest, root, root, crest, crest]
```

Five linear axial profile segments span one complete period. Each start is
shifted by one axial pitch. Every turn is divided into four signed quarter
rotations, with positive axial advance L/4. The exact nominal target on each
patch is a Z-axis helicoid sweeping its original axial line segment. The output
is untrimmed: it includes the complete original start/turn/profile coverage,
including portions that a finite body will later clip away.

Parameter boxes use outward arithmetic, Taylor trigonometric bounds, Machin pi
and the existing bounded atan series. Radius and axial profile-point derivation
are independently bounded. The stored quintic surface is requalified with the
original helical saved-net checker against nominal phase/twist/height boxes;
rotation preserves the source profile-point error, so the sum gives a continuous
whole-patch world-error bound including all saved controls and nominal inputs.
Future point-evaluation rounding is separate. No libm value or sampled residual
is used as continuous evidence.

The verifier requires exactly 20*n*turns original patches in canonical order,
with every start, turn, quarter and profile-segment role checked. Stored U basis
and weights are qualified against the original source. The immutable owner
saves the spec, parameters, actual patches and receipt and replays all proofs.
Changing actual controls increases the measured bound; omitted/relabelled patches
cannot reuse a receipt. Construction rejects any bound above the unchanged
caller world tolerance.

Supported bounds: module 0.05..100, quotient 6..80, axial pressure angle
0.15..0.65 radians, dedendum factor 1..2, backlash in [0,m/2), starts 1..8,
turns 1..16, 1..2560 caller patch budget. Input combinations must prove positive
root/crest widths, nonoverlapping adjacent root teeth and ordered stored axial
phases. The inherited six-quintic-span surface limit may refuse very small
error budgets; geometry/tolerance are not changed to bypass it.

Tests cover independent nominal rack/radius/pitch/lead/lead-angle equations,
all working-patch positions for both hands and multiple starts/turns, changed
actual surfaces and original pressure angle, missing/reordered inventory,
invalid specs, maximum 8-start/16-turn census and exact resource boundaries.
Finite slab clipping, shared-edge sewing, cap sections, complete profile/body
embedding and worm-wheel generation remain open.

## Complete finite-slab source coverage

`trimmed_patches_certified` owns the qualified ZA source, retained/excluded trim
inventory and independent affine clipping certificate. It reuses geometric
screw-patch containers and the clipping algorithm, without constructing a
60-degree thread spec or using thread nominal geometry as proof.

`thread::trim_patch_candidates` clips supplied original surfaces; it validates
all original control nets before using positive-weight hull exclusion. Its
candidate flags alone do not assert nominal source or complete coverage.
`thread::certify_patch_trim_coverage` independently enumerates the rectangle
perimeter and exact plane recipes, verifies original role/surface bindings and
complete retained/excluded census, and checks stored polygon isotopy and the
outward axial error band. The old thread entry points retain their original
spec generation and delegate geometric work to these shared functions.
The ZA wrapper first replays its own nominal saved-patch certificate.

This proves complete clipping of the authored affine Z field and bounded
stored-surface/UV discrepancy. It is not an exact surface/plane intersection
certificate or a combined ideal finite-body Hausdorff bound.

## Native finite worm body

`brep_core::worm::finite_with_certificates(spec, slab, limits)` creates a finite
ZA worm blank with independently certified original patches, complete slab
coverage, sewn retained surfaces, two exact represented planar cap profiles,
positive numerical volume and complete continuous current boundary agreement.
The opaque owner binds every actual wall surface to exactly one original
retained source patch, owns cap proofs, exposes receipts and does not accept
caller-supplied certification flags.

The geometry-only screw network and external cap helpers are shared with the
existing thread kernel. Original ZA profile/module/pressure/lead qualification
remains in the worm API. No fake thread spec or copied 60-degree certificate
is used. Existing external/internal thread regressions remain required gates.

A finite slab must admit complete connected end contours inside the original
untrimmed coverage. At least two source turns are needed for a nonzero complete
axial slab. For example, default dimensions with `turns: 3` admit slab [4,8].
The constructor refuses incomplete contours, insufficient work, unresolved
coverage or boundary agreement. Network capacity is 254 retained patches;
curve, cap, agreement and mass budgets use the shared `BuildLimits` defaults.

Native guarantees match the existing finite-thread constructor criterion:
original nominal working-wall error <= spec.error_budget (default 1e-4), complete
trim/axial-band admission, exact represented cap-profile geometry and source
binding to one closed owned body candidate. `solid_geometry_certified()` remains
false and `ideal_finite_body_error_upper()` returns None. Global wall/cap
embedding, cutter envelopes, conjugate wheel contact and a combined ideal
finite-body bound remain separate qualifications; no integration/export gate
is implied by native Rust status.

Body tests cover both hands, more than one full lead of axial extent and two
starts. An independent numerical volume oracle uses
pi*(wc*Rc^2+(p-wr)*Rr^2+(wr-wc)*(Rc^2+Rc*Rr+Rr^2)/3)/p times slab length.
Invalid work and incomplete source axial coverage must refuse. The volume
comparison is a regression oracle, not a whole-solid certificate.
