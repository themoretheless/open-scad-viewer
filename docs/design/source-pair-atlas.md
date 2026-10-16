# Original source-volume pair atlas and periodic Bishop smoothness

Native successor of `3e6044fed` and `bishop-source-volume.md`. All geometry, decisions, subdivision, exact endpoint ownership and source ledgers remain in Rust. This is a sufficient native proof family, not completion of the full general geometry goal or a public WASM qualification.

## Exhaustive parameter-pair cover

`MultiSweep::certify_ideal_volume_atlas` encloses the ORIGINAL map F and its three first Jacobian columns on original UV/traversal boxes. It uses original positive rational carriers, their own knot domains, path/guide phases, orientation fields and affine/contact laws. Retained meshes, loft patches and sampled separation never supply the source proof.

The bounded queue starts at the entire parameter square (Q,Q). A diagonal split covers (L,L), (R,R), (L,R), with symmetry supplying (R,L). A distinct pair splits one factor and retains both products. Thus completing the queue covers EVERY unordered pair of source parameters, including all diagonals. There is no whitelist of tested stations or unprocessed pair credit.

Each pair must satisfy one of these sufficient conditions:

1. Strict world-space AABB separation from outward original value enclosures.
2. A regular entire interval Jacobian matrix on the FULL convex parameter hull of both boxes. A single constant projection is applied to that enclosure, and its outward determinant excludes zero.
3. A full subdivision of that joint hull with ONE common constant metric and a strictly positive symmetric-Jacobian bound on EVERY subcell. The numerical inverse of a midpoint matrix is only a proposal for that metric. A rounded inverse or separate metrics on different pieces cannot certify anything. The bounded trial cover must finish; discarded trial work remains charged.

For condition 2, the segment integral gives F(x)-F(y)=M(x-y), where M is an averaged Jacobian. Every entry of M belongs to the same interval matrix DF(H). Excluding zero from the determinant of that ENTIRE matrix box proves M nonsingular, hence injectivity on H. Merely proving positive determinants at endpoint matrices, or even actual pointwise Jacobians, does not establish this averaged-matrix property. The unit regression contains two positive-determinant endpoint matrices whose convex matrix hull includes singular matrices.

Condition 3 integrates the same strict positive symmetric form along the whole segment. Source continuity is independently owned, and all one-sided knot derivatives are enclosed. Consequently the piecewise C1 segment integral remains valid without claiming smoothness at an unsupported knot.

Diagonal pairs prove one-sided volume regularity. Exhaustive distinct pairs prove global injectivity. On a compact valid material UV region this gives a homeomorphism onto the original image, preserving its topology and holes. The NURBS-level report explicitly remains relative to a valid UV trim region; it does not invent trim validity or original contour binding.

The existing `certify_ideal_volume` global strong-monotonicity API is preserved. Atlas reports have `monotonicity_lower=None`: local metrics do not establish one global monotonicity lower bound.

## Closed source quotient

The closed atlas currently requires one literal traversal-independent filled carrier and nonperiodic clamped source curves. Exact stored Cartesian endpoint equality is checked independently for path, uniform scale, twist, affine axes/center and any guide. Different positive endpoint weights may represent the same exact clamped endpoint; proximity and evaluation tolerance never substitute for identity. Authored, fixed-normal or guided frame periodicity comes from the existing native closed-frame owner.

For disjoint early/late traversal boxes, a wrapped joint chart is evaluated as the original tail `[late.start,1]` and head `[0,early.end]`, with the SAME UV hull, projection and any proposed metric. This is the lifted interval `[late.start,1+early.end]`; both pieces have unit traversal derivative. Original knot vectors are never shifted or rounded. Exact endpoint ownership makes their source images continuous at the seam. The matrix-union proof or the complete common-metric subdivision includes BOTH one-sided seam limits.

Equal lifted parameters identify only the same UV coordinates at t=0 and t=1. Distinct parameters halfway around a twice-traversed physical loop remain distinct and must be rejected. The negative double-cover regression has a regular path and periodic regular frame, but receives no embedding/topology claim.

This proves a D x S1 source quotient for admitted modes. It does not treat the duplicated endpoint cube as injective. Closed spatial holonomy-corrected volume jets, periodic-storage splines, arbitrary cap atlases and pole quotients remain outside this sufficient source-volume family.

## Original pose reuse and source identities

Within ONE atlas call, an original temporal affine pose `F=o(t)+A(t)q` and its first derivative are certified from the same source laws. The zero vector supplies o; literal unit vectors with a zero translation law supply columns of A directly. No independently enclosed translated-image subtraction is used. Every original restriction is charged once before reuse in different UV boxes. The cache is owned by that call and cannot borrow proof work across budgets or requests.

For distinct moving carriers, the time column includes BOTH `A' q` and `A q_t`; UV columns use `A q_u` and `A q_v`. The independent polynomial pose regression checks all these terms. Actual spatial parameter/arc examples include distinct rational/nonplanar moving carriers, not only the constant-carrier optimization.

Authored axis/normal laws and exactly owned guide offsets preserve a useful literal source identity: an axis seed with strictly signed original coefficients, orthogonal to the longitudinal plane, has an exactly constant projected unit normal. Its first/second normal jets vanish. The native shortcut uses original coefficient identities, never sampled parallelism, approximate planarity or a rounded guide difference. Altering one source plane coordinate or changing seed sign disables the identity.

## Closed original spatial Bishop C1/C2

The explicit original spatial Bishop policy now admits periodic original C(k+1) Cartesian path jets, positive whole-domain source speed, a nondegenerate initial normal and periodic Ck twist. Both parameter and arc policies retain that conservative Cartesian owner; geometric-only clock/seam joins are not inferred.

Let raw Bishop angular velocity be omega=T cross T'. Let the exact holonomy angle be theta and let a(t)=theta*normalized_arc(t). The corrected normal satisfies

```
Omega = omega + a' T
N_corrected' = Omega cross N_corrected
N_corrected'' = Omega' cross N_corrected + Omega cross N_corrected'
```

By definition of exact holonomy, the corrected endpoint normal equals the initial normal. Periodic source tangent jets and correction rates make Omega and its applicable derivatives periodic, hence the normal and binormal derivatives also match. Twist preserves the established periodic Ck field. No binary64 TAU identity, sampled reflection derivative or rounded angle endpoint equality is assumed.

This is a frame well-posedness/smoothness theorem. Corrected derivative interval enclosures for original volume/error certification remain to be implemented; the raw open-transport jet API continues to refuse corrected normals. The JSON frame report keeps retained seams, profile/cap joins, boundary error and Solid flags false.

## Native ownership and delivery scope

`brep-core::sweep_boundary_owned::construct_with_ideal_atlas` retains the same exact cap/profile/trim binding and retained-solid construction as the existing stronger constructor. No model is emitted on an unresolved original atlas, even if retained solid validity is already true. `idealBody.method="atlas"` selects it in the Rust JSON adapter; the omitted method keeps the legacy monotone proof. Unknown methods and caller-provided success flags cannot replace the native decision.

The positive native source-volume matrix includes a 180-degree turn, a closed quotient, authored parameter/arc frames, a closed guide, affine laws, rational carriers/domain remapping and open original spatial Bishop parameter/arc maps with distinct moving carriers. Near closure, singular carriers/projections, an actual source crossing, a locally regular double cover and one-short/zero ledgers are negative cases. The independent direct native STEP check remains scoped to its listed constructed model; it is not a general OCCT qualification of the new closed sources.

Remaining full-goal work includes corrected closed Bishop derivative enclosures/volume admission, corrected-Frenet general modes, arbitrary cap atlases and pole quotient smoothness, general source-to-retained G1/G2 correspondence, filled cap/wall joins and sharp-join C0 ownership. Fresh main integration, rebuilt public WASM, immutable delivery fingerprints, fresh CI/semantic G1, independent STEP and browser matrices remain separate gates.
