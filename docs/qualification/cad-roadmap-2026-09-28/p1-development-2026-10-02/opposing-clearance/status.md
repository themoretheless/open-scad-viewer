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
new lower bound remain pending. Endpoint G1 and general geometry remain open.

Frontend witness admission now checks coordinate containment, consistency of
point distance with the global interval, and coverage of the complete witness
box by the upper bound. Fifteen face/shell/surface measurement regressions and
Vue type checking pass. Actual-WASM six-pair and worker budget tests are
prepared; the browser script accepts these source fixtures and records worker
round-trip time separately from subsequent rendering. Those new runs remain
pending while the corresponding WASM build is live.
