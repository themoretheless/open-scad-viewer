# Six-sided central graph — boundary G2 qualified, solid unfinished

All six complete boundary seams of one unequal-radius central graph are **natively qualified within the stated G2 tolerances**. Its spatial rim and whole-chart projection injectivity are also proved. **Three source faces are now cut and grafted into a 13-face candidate with three strip faces, one trimmed central face and three planar far stop caps. All 13 trim regions and continuous seam closure pass. Material embedding and solid validity remain unqualified; the planar stops are not smooth end transitions.** See [native-boundary-summary.json](native-boundary-summary.json) for the six bounds and explicit scope.

`brep_core::region_replacement::propose_central_graph` builds a stored B-spline graph with six projected original boundary pcurves. It fits original-support position, gradient and Hessian fields using column-pivoted Householder QR. The orthonormal chart and Greville coordinates preserve an affine planar parameter projection. Options include uniform refinement and C2 knots at junctions, with additional simple knots on nonlinear support sections. Unconstrained controls do not acquire geometric authority from the fitting objective.

`inspect_central_graph` freshly checks original curve/support positions, six G2 seams, the original spatial rim and whole-chart injectivity. Fitting diagnostics and previously issued collar receipts are not admission inputs. This is surface/rim inspection; original-face trim ownership, replacement, global material embedding and solid closure remain separate.

The position verifier previously declined a degree-40 composition and failed to admit a planar multispan control within a 100,000-cell shared budget. Its optional correlated enclosure now supports a bounded composed degree of 144 (position cross products at most 160). Graph-jet residuals additionally require bounded polynomial products through degree 256. Binomials outside the exactly representable integer range retain outward interval products/divisions. The old low-degree arithmetic is retained. No geometric tolerance was increased.

## Correlated whole-curve graph inspection

`nurbs_core::surface_g2_pcurves::inspect_owned_graph` is a separate read-only proof path. It freshly checks both original-support lifts against the common 3D curve. It then traverses normalized curve intervals, covers every intersected source/candidate knot span, and compares graph gradients and Hessians in the exact integer chart `(x-y, x+y-2z, x+y+z)`. It supports C2 internal knots, clamped Bezier pcurves and polynomial candidate weights equal to one. Unsupported/singular/exhausted cases cannot grant admission.

Authored binary64 values become exact rationals before narrow chart subtraction, derivative coefficient calculation and Bezier extraction. Conversion back to binary64 is directed and checked against each exact rational. Original rational source weights are retained. The small non-affine XY component introduced by stored world poles is bounded from original derivative coefficients and corrected in the graph jet residuals. Coupled normalization compares world normals and signed world curvature tensors. Span nets are cached only on their immutable original support; no prior receipt supplies geometry.

Coefficient work is separate from exact position-predicate work. Exact rational coefficient emissions and conservative interval convolution work are charged. The backend neither consumes sampled residuals nor changes the existing default central inspector. The optional `inspect_central_graph_correlated` integrates fresh six-seam proofs with the original rim and global projection check, using one shared coefficient budget. Neither native inspector authorizes solid replacement by itself.

Current native controls: 11 composition tests pass (including exact narrow cells, subnormal directed conversion, original multispan derivatives, rational-source chain-rule cross-check, cache replay/work exhaustion and changed curvature); 19 graph-filtered tests pass (overlapping the four graph composition tests, with four new full-traversal/position-ownership/orientation controls). Ten integration regressions for high-degree positions, original-span collars, owned seam positions and global projection injectivity also pass. Sixteen B-rep controls pass: eleven central-graph tests, two spatial-rim tests and three original-strip tests. Two owned-graft regressions also pass, including unchanged source preservation, exact face-local UV ports, unchanged original edge/face IDs and full continuous closure. One streamed-QR analytic control and four conservative box-containment controls pass. This is 59 unique focused tests, not the full kernel or body acceptance suite.

## Verified controls

- Independent quadratic graph positions and full sampled derivative fields.
- Affine chart reproduction across simple internal knots.
- Planar six-sided admission, including a nonuniform C2 junction mesh.
- Surface displacement, pcurve displacement, changed original curvature with unchanged seam positions/normals, and invalid frame/work inputs refuse.
- Degree-80 correlated composition qualifies a full planar parabola in one cell; translating the surface produces a certified mismatch.
- Existing three-strip and spatial-rim regressions pass.

## Actual unequal-radius corner

The fixture retains radii 0.1/0.2/0.35 mm and acceptance tolerances 1e-7 mm / 1e-5 normal / 1e-4 mm^-1 shape tensor. Several proposals failed these limits; their data are retained under ignored `output/qualification/central-graph-*20261028*`. The independent OCCT checker reconstructs stored coefficients and intersects pcurves with knot planes, then samples each crossing interval as well as the uniform grid. Samples can reject a candidate but cannot establish whole-seam G2.

The degree-9/subdivision-2 proposal with junction refinement 5e-5, 33 base fitting stations and regularization 1e-6 passes 2,541 adaptive OCCT boundary stations: maximum position 7.163e-10 mm, normal 3.268e-8, world curvature tensor 4.285e-5 mm^-1. Its source hash is `d7659a4ef1fcd28ebe3ca85ba4762bdd5b214ab7031fc158eb5fa3eb01513be1`. The older generic native inspector proves all six positions, the simple spatial rim and global projection injectivity, but remains unresolved for G2 after 600,000 shared cells.

**The same candidate is not suitable for cutting the cube.** Independent interior grid points give negative original coordinates down to -0.01142649 mm. One witness has graph UV `(0.64,0.8)` and world position approximately `(0.35042546,0.21182546,-0.01142649)`. Polygonal membership makes this an exploratory material check, not a trim certificate, but this result must be resolved before body replacement. Passing boundary samples does not qualify the enclosed shape.

The new correlated inspector initially runs out of a 100-million coefficient budget with no accepted geometric cells. With a 500-million budget it accepts three complete cells before returning unresolved; position ownership remains proved. The larger runs then qualify all six full seams, with 14,914,972,098 conservative coefficient-work units in total. Three source planes have opposite raw parameter orientation; a dedicated regression caught an incorrect refusal on negative denominators. Signed interval division fixes that refusal, with one coherent normal/curvature sign throughout each seam. No failed or interrupted experiment counts as admission.

| Seam | Raw orientation | Normal upper | World tensor upper, mm^-1 |
|---|---|---:|---:|
| 0 | Same | 1.545e-7 | 9.174e-5 |
| 1 | Opposite | 8.414e-9 | 9.812e-5 |
| 2 | Same | 2.127e-7 | 4.635e-5 |
| 3 | Opposite | 4.501e-8 | 3.012e-5 |
| 4 | Same | 7.155e-8 | 4.451e-5 |
| 5 | Opposite | 1.097e-7 | 9.753e-5 |

`propose_central_graph_guided` adds bounded, soft world-position guides to select interior fitting modes while retaining all original boundary jet rows. Guides grant no material, trim or G2 authority. Guided candidates need the same independent and native checks.

The first three-guide correction retains the adaptive sampled boundary limits (maximum tensor error 4.283e-5 mm^-1) and improves the exploratory interior minimum from -0.01142649 to -0.00563359 mm. It is still not an admissible material replacement and cannot inherit the first candidate's native G2 evidence. The second 20-guide proposal also passes all six native G2 seams; see [guided-boundary-summary.json](guided-boundary-summary.json). Although its 361-point interior grid is nonnegative, a denser independent 5,873-point grid exposes a remaining -0.01081961 mm excursion. Coarse-grid positivity cannot authorize trimming or solid replacement.

The midpoint-only thin-plate experiment is rejected: its sampled curvature exceeds 1e-4 mm^-1 and the dense interior grid reveals oscillatory free modes. `propose_central_graph_fair` now uses a scale-aware control-net Laplacian involving every interior pole. Sparse fairing rows are appended to the existing Householder QR factor using Givens rotations, avoiding expansion into another large dense matrix. No normal equations are used. All original boundary jet rows remain. Planar/quadratic reproduction and an independent analytic streamed-QR solution pass; no fit energy grants geometric authority.

The row-major Householder implementation produces a byte-identical candidate on a controlled input pair, with observed wall times 942.22 s before and 72.90 s after. This is one wall-time pair with other workspace activity not isolated, not a universal speed claim. That midpoint-fairing candidate fails geometry checks; the performance evidence does not qualify its shape. See [qr-row-major-pair.json](qr-row-major-pair.json).

## Actual cuts and closed boundary

`trim_central_graph` creates the real six-edge trimmed NURBS face through the existing native face recipe. Actual curved-loop orientation, region validity and all original world-edge lifts are freshly checked. The real unequal-radius face passes, with six edges and 612 agreement cells.

`propose_trihedral_graft` cuts each of the three original edges at a freshly isolated far cutter root, retains the source tails and supports, imports the original strip definitions, and inserts the contact/connector chains into the three source faces. Endpoints are shared by explicit source/strip roles. No world-space proximity welding occurs. The finite far stops are three planar caps, **not G2 end transitions**. Shell orientation propagates from unchanged original face uses through actual shared edges.

The first whole-body trim audit exposed nonliteral UV joins on the three modified source faces. New rounded pcurves now share a single face-local UV port per topological vertex, while original retained uses own their endpoints. Every resulting edge lift is reverified continuously before the candidate returns. This is authored UV binding followed by proof, not a tolerance increase or a world-coordinate snap.

The actual graft contains 22 vertices, 33 edges and 13 faces, satisfying V-E+F=2. All 13 trim regions pass and native continuous closure is WithinTolerance. [graft-summary.json](graft-summary.json) preserves the explicit scope. Closure compares each face lift to its shared edge within the original model tolerance; it does not claim exact analytic equality or prove embedding/volume.

A 28,080-byte offline binary fixture supports the native graft regression. Its rounded reconstruction is independently verified for continuous closure and cannot inherit the original candidate's G2 receipt. It is deliberately labelled boundary-only, with the material excursion preserved as a known limitation.

`face_domain::inspect_box_containment` adds a conservative trimmed-face XYZ cover. Cells are omitted only after complete UV Outside classification; midpoint witnesses can reject but never admit a whole cell. Exhaustion is Unresolved. Box containment is not general material containment or a solid certificate. The current examples use the explicit original [0,10]^3 cube.

Global high-degree fits, uniform quintic grids and initial C2 meshes have not qualified. These failures cannot authorize replacing the live source document. Isolated cut/graft proposals are diagnostics, not accepted geometry. Source-face trimming and a closed boundary with valid trim regions are implemented. Smooth far end transitions, material embedding, whole-body intersections/thickness/volume and final solid admission remain unfinished.


## Current reference candidate and independent exchange

The control-net G proposal freshly passes all six complete native G2 seams; see
[control-net-boundary-summary.json](control-net-boundary-summary.json). The graft
has 13 valid trim regions and continuous closure. A native conservative UV
Inside witness still proves a point outside the original cube's allowed box:
UV approximately `(0.0722845753,0.3638375296)`, world X enclosure
`[-1.271367929e-7,-1.271367924e-7]` mm, against the unchanged lower bound
`-1e-7` mm. This proposal is not accepted as a material-preserving fillet.
[material-box-counterexamples.json](material-box-counterexamples.json) retains
separate native rejection witnesses, without replacing them by grid claims.

Local optional C2 knot refinement is available through
`propose_central_graph_refined`, retaining the same 64-per-axis/4096-pole and
fitting-work limits. Planar reproduction and rejection of C1 multiplicities
pass. Refined and stronger-guide experiments that increase material excursion
remain rejected, even when their boundary samples pass.

The original diagnostic STEP /3 attempt had no transferable OCCT root. The
example now uses the existing complete AP242 STEP /5 export. Independent OCCT
7.7.2 transfers one root, obtains one solid/shell with exactly 13 faces,
33 edges and 22 vertices, and passes basic B-rep validity. Its numerical volume
estimate is 999.9691076169 mm^3. [step-occt-summary.json](step-occt-summary.json)
explicitly excludes global embedding, material and self-intersection admission.
Passing this representation check does not erase the native material witness.

The STEP artifact and exploratory proposals remain in ignored
`output/qualification`; only small scoped summaries and the 28 KB regression
fixture are retained as source evidence. WASM/worker/UI integration and the
bracket/flange/housing acceptance chains have not been performed for these new
native APIs.

## Hard-anchor construction follow-up

A bounded equality-constrained QR proposal path is described in
[hard-anchor-qr.md](hard-anchor-qr.md). It adds independent construction
controls and focused regressions; it does not replace the current reference
receipts or resolve the saved material counterexample by itself.
