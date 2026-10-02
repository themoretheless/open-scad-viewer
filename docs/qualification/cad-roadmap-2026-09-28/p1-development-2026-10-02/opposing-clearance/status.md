# Selected opposing-face clearance

The distance kernels supplement Cartesian lower bounds with conservative
axis-radius separation. Each coordinate-axis radius map is 1-Lipschitz.
Its outward Bernstein interval covers the complete authored surface image;
therefore separation of two such intervals is a lower bound for every point
pair, including trimmed subsets. Numeric failures and unsupported charts
keep the original subdivision bound. Upper bounds still require admitted
points in the original trim domains. No sample or nominal radius supplies a
lower bound. Projection changes one coordinate exactly and preserves the source.

Native qualification on the canonical partial annular model:

- Entry, middle and exit to their inner walls: 13.75 mm enclosed, maximum
  interval width below 1.8e-8 mm, 29–39 geometry cells.
- The same three zones to their bottom faces: 4.75 mm enclosed, maximum
  interval width below 5.6e-6 mm, 119–351 geometry cells.
- Retained concentric wall pair: 15 mm enclosed within 1e-6 mm using one
  geometry cell; original-face parameters and point enclosures retained.
- 22 distance tests, 5 B-rep face-domain tests and the axis-radius regression
  pass. The latter covers all three axes, translated origins, source
  preservation, malformed input and invalid axes.

Reproduce the six-pair report with the `transition-wall-clearance` example.
`native.json` retains the exact source model and original-face witnesses.
This proves selected face-image clearances, not a whole-body minimum thickness
or material chord certificate. WASM and application qualification of this
new lower bound now pass on the canonical source. Endpoint G1 and general geometry remain open.

Frontend witness admission now checks coordinate containment, consistency of
point distance with the global interval, and coverage of the complete witness
box by the upper bound. Fifteen face/shell/surface measurement regressions and
Vue type checking pass. Actual-WASM six-pair and worker budget tests are
prepared; the browser script accepts these source fixtures and records worker
round-trip time separately from subsequent rendering. Those runs now pass on the corresponding packaged WASM.

## Expanded qualification

- Eighteen axis-aligned specimens (three scales, three radii, two directions)
  pass all 108 native selected-face measurements. Each STEP import is valid.
- Independent OCCT association uses unique interior anchors and surface distance
  below 1e-7 mm, rather than relying on face order. All 108 independent STEP
  distances agree; maximum nominal-size error is 3.552713678800501e-15 mm.
- Actual WASM passes all six canonical clearances with source-point re-evaluation
  and three worker-handler cases, including geometry/domain work exhaustion.
- Four final browser scenarios pass: inner wall and bottom, mouse and keyboard.
  They verify numeric intervals, visible witness endpoints, cancellation,
  localized invalid-face input, Retry, unchanged documents and exact reload.
  Both final mouse screenshots were visually inspected.
- Forty focused frontend regressions pass on the new kernel. The full native
  B-rep suite has been started; its result is not yet recorded here.

The twelve observed worker round trips range from 17.3 to 78.9 ms during four
concurrent browser qualifications. These measurements include worker startup
and messaging and exclude subsequent rendering. They are not isolated CPU
benchmarks or a latency guarantee for arbitrary geometry.

WASM SHA256: `fb4171cb184302b37ce61543b9fee885a4f66b37b2420fc7ecc0ec91ce66afc3`.
Build verification: 140 artifacts, 7,293,083 asset bytes, 11,839,163 raw WASM
bytes. Evidence is in `../opposing-clearance-matrix/`,
`../opposing-clearance-wasm/`, and the four `../opposing-clearance-*-final/`
directories. Source, STEP files, original and imported face associations,
point enclosures and browser timing samples are retained.

Whole-body minimum wall thickness, material-chord proofs, endpoint G1 and
arbitrary placements are still open. These selected-face results do not
change the preview Apply gate or close the full P0–P3 roadmap.
