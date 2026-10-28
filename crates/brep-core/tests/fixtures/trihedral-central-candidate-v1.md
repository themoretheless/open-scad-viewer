# Boundary-only central graph fixture

The 28,080-byte `OSVCGF01` fixture stores six little-endian u32 dimensions,
original U/V knot arrays, six affine projection coefficients and the world-Z
control grid as binary64. XYZ is reconstructed in the test, producing a new
rounded candidate. Every boundary lift must be freshly verified; no G2 receipt
is inherited from the source artifact.

Source: the 20-guide degree-9, 58-by-58-pole candidate for a 10 mm cube with
strip radii 0.1/0.2/0.35 mm, 0.1 mm clearance and 2 mm finite strips.
The source JSON SHA-256 is recorded in
`docs/qualification/central-graph-2026-10-28/guided-boundary-summary.json`.

This fixture exercises owned root cuts, strip/connector incidence, continuous
closure and source preservation. It is **not a valid-material solid reference**:
the source candidate has an independently observed interior excursion, and
the finite far stops are planar caps rather than G2 end transitions.
