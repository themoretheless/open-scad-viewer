# Original source volume and complete retained boundary smoothness

This work extends PR #57 (`1447375db27b676002a057cef39df2958cc6faac`). All geometry, predicates, ownership, budget handling and decisions are in Rust. The bridge only decodes requests and serializes native reports. This is a native development checkpoint; the public WASM and frozen delivery qualifications have not been updated for it.

## Original filled source volume

`MultiSweep::certify_ideal_volume` proves a sufficient condition for the **original** mathematical volume, before retained station interpolation. Given two original rational cap carriers on normalized UV, define

```
C(u,v,t) = (1-t) C0(u,v) + t C1(u,v)
F(u,v,t) = path(t) + frame(t) [scale(t) axis_scale(t) q(C) + center(t)]
```

The initial coordinates `q` belong to the original initial pose. Contact fitting, when present, multiplies its original transverse coefficient and its derivatives. Values and first derivatives are enclosed from original coefficients, positive rational weights, original knot cells, and outward arithmetic. Independent cap degrees, knot domains and weights are supported by the low-level volume map.

One fixed finite matrix P is used on the **entire** convex normalized cube. Every cell must prove a positive lower bound m for the symmetric part of P DF. Strict interval diagonal dominance is sufficient; numerical eigenvalues and Jacobian determinant signs are not accepted.

For any two parameters x,y, the line segment is contained in this cube. Integrating the one-sided derivatives on its finitely many smooth pieces gives

```
(P(F(x)-F(y))) dot (x-y) >= m ||x-y||² > 0, for x != y.
```

Thus F is globally injective; its one-sided volume Jacobians are nonsingular. Continuity on the compact parameter domain makes F a homeomorphism onto its image. Restricting it to a valid trimmed region times the traversal interval preserves that region's topology, including holes. This does not infer the validity of caller-supplied trims.

`brep_core::sweep_boundary_owned::construct_with_ideal` binds this theorem to the **same** source used by the retained-body constructor. It additionally requires:

- complete original contour/cap agreement, including all holes;
- literally common cap UV trim curves and natural domains;
- nonzero one-sided original profile tangents;
- retained filled-cap/wall error bounds, whole-face regularity and solid validity.

An unresolved source proof returns `Attempt::Refused`, with no constructed model. Retained solid validity remains separately visible; it cannot replace the ideal-volume proof. Existing requests without `idealBody` retain their previous scope and return `idealBodyEmbeddingCertified:false`.

The optional Rust/JSON request field is:

```json
{"idealBody":{"projection":[[1,0,0],[0,1,0],[0,0,0.015625]],"maxCells":100000,"maxExactWork":1000000}}
```

Cell ledgers include unsuccessful parent boxes, all original cap knot cells, source-frame premises, law restrictions, source profile regularity and original anchor work. No partial cover supplies embedding/topology credit. The exact-work ledger is separately reported. Original curves, weights, knots and carriers remain immutable.

For guided parameter laws with a literal common basis, an error-free TwoDiff check may prove each G-P coefficient subtraction exact. Only then may the exact difference curve supply correlated original jets. Inexact subtraction uses the independent original enclosures. This avoids subtracting two wide translated boxes without introducing a fitted proxy.

### Supported source scope and remaining obligations

The convex-cube certificate currently covers open parameter-spaced authored, fixed, fixed-normal, Frenet, guided/contact, and recognized planar/straight RMF laws when its sufficient monotonicity test succeeds. Strong bends can legitimately remain unresolved under a single fixed projection. Distinct source cap interiors are interpolated by the explicit homotopy above; they must share the same profile boundary and UV region in the owned constructor.

It **does not yet establish** closed/periodic quotient-volume embedding, spatial Bishop volume jets, curved arc-length volume jets, general corrected-Frenet volume jets, arbitrary cap atlas correspondence, or a complete intersection decision for bodies outside the monotonicity condition. These need chart-local injectivity plus global inter-chart separation and source-law derivative correspondence. A small boundary error alone cannot discharge them.

## Original open spatial Bishop frame smoothness

For explicitly owned spatial RMF, a regular C(k+1) original open path and Ck twist give C(k-1) Bishop ODE coefficients

```
N' = ((P' cross P'') / |P'|²) cross N.
```

The original whole-domain speed cover and nondegenerate initial seed establish the unique Ck orthonormal frame, for k=1,2. Positive speed also makes normalized arc-length reparameterization smooth. This proves source **frame** smoothness only. It supplies neither retained surface Gk nor source volume embedding, and excludes the closed seam.

## Complete retained G1/G2 scope

`brep_core::sweep_smoothness::complete::inspect` and `brep_complete_boundary_smoothness_audit` enumerate every authored edge use. There is no selectable cap/seam list. Admission requires:

- exactly two classified natural-boundary uses for each edge, including closure;
- exact original world-edge/pcurve/surface correspondence and exact loop joins;
- a single connected degree-two vertex-link graph at every vertex;
- independent regularity of every complete face;
- exact sufficient projective strip jets for every owned join at the requested order.

Reversed along traversal is supported by exact pole/weight permutation for literal Bezier directions. Unsupported trimmed or multispan reversals stay unresolved. A nonzero budget cannot remove a cap or closure from the scope. G2 implies G1; a request for G1 does not first consume its allowance attempting G2. Exact seam work, agreement work and regularity cells have separate declared ledgers.

The conic predicate now accepts an exactly factored positive along-row gauge: each row has cross weights `A_i [1,w,1]`, shared by both sides. All factorization identities are proved with exact products. The coefficientwise conic relations then hold in the same rational along basis. No rounded weight product or division is accepted. Tests compare this sufficient branch with the independent general projective predicate, including one-short budgets and damaged coefficients/weights.

The closed positive fixture is a deliberately authored binary-exact conic toroidal shell with dyadic tensor weights. It does not qualify the public torus generator's rounded tensor weights. Sharp caps remain unproved for G1/G2. Pole quotient G1/G2, arbitrary trimmed joins and general variable cross reparameterizations remain separate work. Retained smoothness does not imply ideal-source smoothness or Solid validity.
