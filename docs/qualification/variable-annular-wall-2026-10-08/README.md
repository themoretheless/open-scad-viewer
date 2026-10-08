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
The independent 496-pair material-wall audit is running separately.
