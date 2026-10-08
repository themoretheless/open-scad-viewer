# Original annular wall coverage

## Intrinsic same-face evidence

All 31 original surfaces of each of four variable annular bodies were checked
for continuous exclusion of chords aligned with endpoint normals at sin² <=
1e-4. Sixteen full-chart certificates per body succeeded; fifteen remain
unproven. The original model is serialized before/after and must remain equal.

The certified curved faces include 2/3 for increasing radius and 3/12 for
decreasing radius. This is full parameter-chart coverage, not finite sampling.
The remaining curved charts need adaptive coverage or explicit treatment of
singular domains. Failure to certify does not establish a thin region.

Distinct-face pairs, material-volume admission and whole-wall minimum remain
separate requirements. No global thickness or wall safety is claimed.

```sh
cargo +nightly-2026-09-16 run --locked --manifest-path crates/Cargo.toml \
  -p brep-core --example variable-annular-self-wall -- /tmp/self-wall.json
```

`self-wall.json.gz` records face identifiers, exact scope, bounds and reasons.
## Initial whole-wall audit

All 496 original face pairs, including 31 self pairs, are enumerated in each
body. The conservative lower bound remains zero: 91 pairs per body still have
zero lower bounds. The material chord is unproven because its volume stage is
unproven. No wall interval or convergence is claimed.

Per body: 17 curved self pairs remain unresolved, 14 planar self pairs are
excluded, 42 coplanar pairs and 172 endpoint-normal pairs are excluded. The
remaining pairs carry distance/control-hull bounds, including 8 or 10 refined
face-distance bounds depending on arc direction.

`whole-wall-initial.json.gz` preserves every face-pair identifier and reason.
This initial route does not allocate the explicit whole-chart linear/quotient
injectivity budget. A later route must separately qualify embedded boundary,
material roles and orientation before the material-chord claim can succeed.

## Adaptive same-face coverage

At minimum threshold 4.74999 mm and sin² <= 1e-4, adaptive complete-chart
pair coverage certifies two further curved surfaces in each body. Combined
with intrinsic results, 18/31 same-face cases are certified. Remaining face
indices: 0,5,7,8,10,16,17,20,21,24,25,28,29. The report preserves pending
regions and exclusion counts; finite work is not treated as safety.

```sh
cargo +nightly-2026-09-16 run --locked --manifest-path crates/Cargo.toml \
  -p brep-core --example variable-annular-self-wall -- /tmp/adaptive.json adaptive
```

An explicit volume audit now accepts separately bounded whole-chart
injectivity work. Its regression requires embedded boundary, consistent
material roles and independent outward orientation together; invalid budgets
are refused. Original default admission behavior remains unchanged.

## Initial explicit-volume result

With explicit chart injectivity work, all four bodies pass original boundary
agreement, exact joins, trim validity and all 31 face injectivity checks.
Pair classification remains incomplete under the initial 256-cell/4096-domain
per-pair budget. Boundary admission therefore refuses volume admission, and
nesting/orientation stages are correctly not run. No volume claim is made.

`volume-initial.json.gz` preserves the original stage results. A repeat uses
the earlier successful embedding audit's per-pair 4096-cell/100000-domain
limits, global 1000000-cell/8000000-domain limits, and explicit independent
orientation work. Pair reasons are recorded to distinguish remaining causes.

```sh
cargo +nightly-2026-09-16 run --locked --manifest-path crates/Cargo.toml \
  -p brep-core --example variable-annular-volume -- /tmp/volume.json
```

## Regular-face refinement

Repeating the eleven regular unresolved charts at 1024 cells/8192 spans per
face gives no additional certificates in any of the four bodies. The reports
retain their explicit selected-face list; omitted singular faces 0/10 remain
unqualified. The verified total remains 18/31, not 29/31.

```sh
cargo +nightly-2026-09-16 run --locked --manifest-path crates/Cargo.toml \
  -p brep-core --example variable-annular-self-wall -- /tmp/refined.json adaptive 1024 5,7,8,16,17,20,21,24,25,28,29
```

The next algorithm investigation is derivative-column scaling in the original
intrinsic chord inequality. Any stronger bound must preserve the full original
pair domain and pass independent analytic and rejection regressions.

## Complete original volume admission

All four bodies pass the repeated native volume audit. Exact boundary/joins,
valid trims, 31 injective faces, all face-pair classification and embedded
boundary pass together. Material nesting roles are consistent. Each original
shell independently proves outward orientation on its first attempted ray.
`proven=true` therefore covers all volume prerequisites for these specimens.

`volume-qualified.json.gz` records all stage outcomes and individual pair
reasons. This supersedes the initial low-budget volume failure while preserving
that evidence. It does not certify global minimum wall thickness, unique regular
pole tangency, general fillets or named-part STEP acceptance.

## Balanced intrinsic differential bound

The native self-chord inequality now also evaluates a fixed positive diagonal
rescaling of derivative columns. With D diagonal, chord=M D (D^-1 delta) and
n.(J D)=0; the original intrinsic inequality therefore remains valid. All
products and ratios use outward intervals. The original bound is retained,
and the smaller of two proved upper bounds is used. Unsupported or nonfinite
balancing does not weaken the original proof.

Nine native intrinsic/adaptive tests pass. An affine parameter-unit regression
uses an independent parabola chord oracle; it explicitly demonstrates an
inconclusive original bound and a successful balanced certificate for the same
original geometry. Strong curvature, collapsed/discontinuous charts and a
rational half-cylinder with a short exact normal chord remain refused.

Repeating the same eleven annular faces at the same 1024-cell budget gives no
additional certificates. The balanced report is preserved as evidence; total
coverage remains 18/31. The change improves parameter-unit robustness, but
these annular charts still require a stronger geometric exclusion or further
complete adaptive work. No new WASM/UI package is claimed for this change.
