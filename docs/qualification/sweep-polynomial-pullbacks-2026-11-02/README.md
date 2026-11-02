# Exact nonlinear polynomial pullbacks for algebraic chart transitions

2026-11-02. Rust proof-only extension; original authored geometry remains unchanged.

The cofactor-sign fallback now tries the four triangular polynomial automorphisms
`(s,t) -> (s,t ± s²)` and `(s,t) -> (s ± t²,t)`. Each has an explicit global
polynomial inverse. Exact expansion followed by exact inverse reconstruction
must reproduce the original rational polynomial coefficient-for-coefficient.
Only then may the existing weighted-square or AM–GM certificate of the transformed
polynomial grant sign evidence. Global bijectivity transfers that sign to the
original polynomial. Nonzero is required. Original monotonicity, jet divisibility,
regularity and all downstream global/Solid requirements remain mandatory.

This extends the directly certifiable monomial cones. It is **not** a general
positivity solver or a proof method independent of AM–GM/SOS in every coordinate
system. No arbitrary automorphisms, rational denominators or complete
Positivstellensatz search are claimed.

Regression: four nonlinear pullbacks of the bivariate Motzkin polynomial. Both
old direct constructors refuse every original expanded polynomial; the new
constructor proves its sign. The shared Bounds::cofactor path is exercised on
[0,1]² for all four cases. Lowering the original constant by one ULP makes the
polynomial negative at a real point (bijectivity preserves this counterexample),
and every such case refuses. Exact work minus one refuses. The zero polynomial
refuses. All work uses the unchanged one-million-unit ceiling and existing exact
arithmetic limits. Degree only selects proposal order and cannot grant evidence.

Development: the first integrated test exhausted the original work budget after
trying the wrong shear axis first. Logs `failed-wide-box.log` and
`failed-unit-box.log` preserve these attempts. Degree-based proposal ordering fixed
the tested integration cases without raising any limit. The final integration
fixture uses [0,1]²; no all-box performance claim is made.

Qualification uses a separate snapshot derived from the prior hash-verified
circuit milestone, overlaid with only three changed Rust files. `source.json`
records every staged file; `changed.json` binds those three files to main.
`qualify.py` guards both before each phase and after completion. See `result.json`
and individual logs for current results. The full dirty main repository is not
qualified. No WASM publication, UI actions, independent OpenCascade/STEP matrix,
or whole sweep completion is claimed. No limits, deadlines or tolerances changed.
