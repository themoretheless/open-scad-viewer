# G1/G2 of the original closed source boundary

This successor adds original-boundary differential compatibility and ties it to
the constructor-owned original global body certificate. The retained affine
Solid retains its independent coefficient binding, complete Gk inspection and
global filled-volume atlas. Original and retained smoothness never substitute
for each other. All decisions are in Rust.

## Exact geometric source profiles

`cad-predicates::rational_bezier_geometric_endpoint_identity` reads original
positive-weight Bezier controls on both sides. Each array starts at the shared
endpoint and points into its own side. It checks literal rational position
identity and opposite nonzero Cartesian tangent directions. There is no dummy
surface, sampled normal, tolerance, square root or rounded speed ratio.

For homogeneous endpoint jets `H,H1,H2`, with weight jets `W,W1,W2`, use exact
Cartesian derivative numerators:

```
D = H1 W - H W1
E = H2 W² - H W2 W - 2 H1 W W1 + 2 H W1²
```

Cartesian first and second derivatives are `D/W²` and `E/W³`. Choose one
nonzero tangent coordinate `k`. Opposite collinearity gives the positive local
speed ratio `lambda = -Db[k] Wa² / (Da[k] Wb²)`. G2 holds when

```
R = Da[k]² Wb Eb - Db[k]² Wa Ea
R cross Da = 0
```

These identities prove that the second-derivative difference after the speed
change is tangent. All divisions have been eliminated by cross multiplication.
Consequently a nonbinary speed ratio is proved without replacing it by a nearby
floating value. Positive weights and nonzero tangents supply its denominator
and orientation premises. Zero tangents receive no regularity credit.

The NURBS source owner checks spanwise original profile regularity and exact
original-to-Bezier decomposition before using this predicate. Every join,
including internal source knots and each ring closure, is inspected. No selected
seam or caller exclusion list stands for the full set.

## Original motion and global ownership

`MultiSweep::certify_original_closed_boundary_smoothness` requires periodic Ck
of the original path translation, scale, twist and affine scale/center laws,
and the existing native closed-frame Ck theorem. RMF/Bishop, authored and guided
frames retain their own nondegeneracy and seam obligations. Arc clocks additionally
require positive original path speed; existing independent guided clocks keep
their own joint cover. First/second source-jet jumps are not hidden by smoothing
the retained station geometry.

The report establishes **differential compatibility only**. For a profile seam,
the exact curve reparameterization jets are independent of traversal time.
Composition with one common periodic Ck affine motion therefore matches the
surface jets, including mixed derivatives and the profile/time seam crossing.
The native body owner additionally requires its original filled-volume
regularity/embedding certificate, valid material region and exact UV/world trace
binding. Those same-call obligations make the resulting boundary a regular Gk
manifold and keep its material topology. No caller-created source report is
accepted by the constructor or auditor.

`SmoothClosedLimits.original_smoothness` is an optional strict stage whose order
matches the retained order. Requested but incomplete source Gk returns no Solid.
Omitting it preserves the separately qualified retained-only API and grants no
source Gk. JSON `originalSmoothness` supplies explicit `order`, `maxCells`,
`maxExactWork`, and `maxProducts`. Rust emits `idealSourceBoundaryG1Certified`
and `idealSourceBoundaryG2Certified` from the native body report. On refusal these
top-level flags are false, while nested partial reports retain their exact scope.

## Evidence and remaining limits

Focused cases prove source G2 differential compatibility for spatial RMF,
authored and guided frames; parameter and arc-length clocks; affine laws; every
zero/one-short source budget; and distinguish a genuine G1 profile curvature
jump and a C1-only twist seam from G2. Strict Solid cases add holes, reversed
winding, and 40 nonbinary time intervals. The native STEP-imported basic model
is freshly audited with both original-boundary G2 and retained-boundary G2.

The frozen full native successor qualification is tracked separately. A focused
matrix is not external STEP, browser, public WASM, fresh CI or a complete semantic
G1 qualification.

This is a closed construction with **zero physical end caps**. General cap/wall
G1/G2, pole quotients, mixed C0/G1/G2 policies, unsupported contact/frame modes
and arbitrary reindexed STEP gauges remain unproved and receive no success
flag. Sharp corners and curvature jumps are preserved; a strict higher-order
request refuses. This checkpoint does not complete the full user goal.
