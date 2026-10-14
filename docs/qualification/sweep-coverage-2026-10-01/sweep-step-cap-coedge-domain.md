# Independent full-domain cap coedge agreement — 2026-10-04

The independent OCCT verifier previously tested cap pcurve-to-world agreement
only at seven parameter samples. It now also requires a whole-parameter rational
enclosure for every imported cap coedge, both outer and hole boundaries.

The admitted imported cap is a clamped degree-1 by degree-1 two-by-two surface
with uniform positive weights. Normalize its arbitrary affine knot rectangle
to unit coordinates. Its exact binary64 coefficients define
`S(u,v)=a+b*u+c*v+d*u*v`. Imported UV and world edge splines must have identical
degrees, knots, multiplicities, positive weights and pole counts, be nonperiodic,
and have every normalized UV pole inside the cap's natural rectangle.

With that shared positive rational basis, the affine residual at every parameter
is enclosed by the convex hull of its control residuals. UV hulls bound the
bilinear remainder: each coordinate satisfies
`|S(pcurve(t))-edge(t)| <= max_i |a+b*u_i+c*v_i-P_i| + |d| max_i|u_i| max_i|v_i|`.
The squared Euclidean upper enclosure is compared to the squared tolerance using
Python Fraction throughout. This admits stored rounding of the affine cap while
covering the complete edge parameter, independently of the sampled checks.
Unsupported caps/curve bases refuse this gate. Exact cap-use count is required;
closed fixtures with no caps have a vacuous zero-use cap obligation.

Fresh source export and independent check:

- Current generated/public/embedded WASM identity verified by exporter:
  `34a4e5be7bd2b9fb9b4156adb42da4c30400e444d50331103594b12d8b27e0e6`,
  10731689 bytes.
- `external-step-cap-coedge-domain`: 34/34 cases pass, 488 cap coedge uses,
  together with the existing full-domain wall UV, shared curve/surface basis,
  topology, material-side witnesses and analytic volume checks.
- Export log: `sweep-step-cap-coedge-export.log`.
- OCCT log: `sweep-step-cap-coedge-current-occt.log`.
- Negative helper checks on a real imported straight hollow cap: original curve
  passes, changing only an internal UV pole by .125 refuses, and changing only
  its rational weight refuses. Results: `sweep-step-cap-coedge-negative.json`.
- Python compilation and scoped diff check pass.

Initial diagnostics assumed unit cap knots; actual STEP cap charts use affine
knot rectangles such as [-.5,.5]. That diagnostic refused all 31 open cases and
is retained as `sweep-step-cap-coedge-domain.log`. General affine normalization
corrected the premise without relaxing the curve basis or tolerance. The final
old-artifact matrix log is `sweep-step-cap-coedge-domain-final.log`; the fresh
current-artifact matrix above is the authoritative new qualification.

This proves fixture cap coedge/world agreement over their full parameter domains.
It does not prove whole filled-cap material correspondence, arbitrary cap
embedding, shell nesting, ideal sweep error, or general G1/G2 continuity.
