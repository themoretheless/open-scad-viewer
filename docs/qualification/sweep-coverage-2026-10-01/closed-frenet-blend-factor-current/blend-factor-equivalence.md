# Equivalence of nonnegative blend-factor multiplication

Scope: the private N `mul_blend_factor` helper on valid finite ordered intervals. This argument concerns this arithmetic substitution only; it is not a whole-sweep continuousBound, global geometry, packaging or UI proof.

## Preconditions and branch

Let A=[a0,a1], B=[b0,b1], with finite endpoints, a0<=a1, b0<=b1. The fast branch requires b0>=0. `interpolation_plan` obtains alpha and beta through validated Interval operations followed by intersection with [0,1]; interpolated controls are also validated Interval results. Inputs manufactured with invalid public struct fields are outside this argument.

For the four rounded binary64 endpoint products, multiplication by a nonnegative operand is monotone. Therefore their minimum and maximum are respectively:

| A sign | lower product | upper product |
| --- | --- | --- |
| a0>=0 | a0*b0 | a1*b1 |
| a1<=0 | a0*b1 | a1*b0 |
| a0<0<a1 | a0*b1 | a1*b1 |

The fast branch accepts these products only when both are finite and nonzero. Every other endpoint product lies between them by the same monotonicity, so all four original products are finite. No original product-finiteness failure is skipped. These nonzero extrema also avoid the signed-zero tie ambiguity of min/max. A nonzero finite binary64 value has a unique representation; all nonzero ties therefore have identical bits. NaNs are impossible from finite endpoint multiplication. Rust release settings retain ordinary IEEE arithmetic and do not enable fast-math reassociation.

The helper applies exactly the original next_down/next_up outward rounding to those same extrema, and calls the same Interval::new. If outward rounding itself exceeds the finite range, both paths produce the same validation error. If a selected product is zero or nonfinite, or B is not nonnegative, the helper calls the original Interval::mul verbatim. Underflow and signed zero therefore retain the original behavior.

## Composition and limits

Only the two multiplications in each planned de Boor blend are substituted. Addition, control-hull intersection, weight intersection, traversal order, knots, coefficients and work charges are unchanged. The original generic interpolate routine remains the oracle. This argument does not assert that unrelated constructor or global certificate logic is complete, or that WASM optimization preserves all behavior without its own qualification.

## Executed evidence

Native release NURBS1384/1384 passed, including the helper edge cases/generated finite interval cases and2187 full rational tensor-net configurations compared to original interpolation bits. The original actual hollow pair retains361/2889, zero unresolved cells, no witness, and identical complete model SHA1466930912255099b45e298ee0f02d56308c9c25bd41e6643177b3a344b225bb. These are implementation regressions supporting the branch argument, not a replacement for remaining BRep/bridge/WASM/UI gates.

Frozen sources:

- surface_distance.rs SHA256 `5e5a7c5d0c23497494f1c46c8176a518a903c23971e9affbd293a702e9fae246`
- distance_bounds.rs SHA256 `ac96eeb9149633b5bbde1c3525a65e92ac61e6a64dfd370fb0bc57971e1ef1fe`

Full source inventory: native-source-manifest.json. Executed native receipt: source-delta.json.
