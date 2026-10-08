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
