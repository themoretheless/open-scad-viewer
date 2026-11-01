# Original material-body face regularity

The Rust owner `source_volume::regularity::RegularBody` borrows the exact admitted
`source_volume::Body`. Its private construction requires fresh nonzero-normal
proofs over every complete original face chart, including filled caps. Since the
material trim regions are subsets of those unchanged charts, the rank evidence
also covers all their material points. The body continues to own the independent
embedding, topology, material orientation and nonzero volume proofs. No rounded
retained face, caller report, mesh or injectivity flag replaces differential rank.

One shared cell budget covers every face. Zero or exhausted work, unresolved
enclosures and declared collapsed polar faces yield no certificate. Failed
interval calls consume the remaining allowance. A certificate cannot escape the
immutable body borrow; its per-face reports are available only by shared reference.

The rank proof is spanwise, including one-sided span closures. It does **not**
assert C1/C2 across internal knots, ordinary seams or vertices. An authored C0
crease remains C0. Original polar parameterizations have rank loss at their
collapsed boundary even when their physical image has a smooth C2 graph disk;
those require the separate owned pole-neighborhood route. No general G1/G2
completion, ideal-law-to-shell binding, WASM, STEP or remote CI is claimed here.

Regression coverage includes all six faces of an admitted projective source
material body, same-body pointer ownership, exact work replay, insufficient and
zero work, excessive budget rejection, source immutability, and refusal to
relabel an existing physically C2 parabolic pole as a regular original chart.

Implementation: `d3ecf8ad9`. Frozen full native `brep-core --lib` passed:
2064 passed, 0 failed, 5 ignored. All 2874 recorded Rust source files remained
unchanged. Ignored tests provide no evidence. `result.json` records the exact
command; full output is in `brep-full.log.gz`. This receipt does not qualify
the later polar aggregation, WASM, STEP, browser or remote CI.
