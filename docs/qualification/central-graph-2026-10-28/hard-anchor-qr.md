# Hard-anchor proposal solver

The central graph proposal now accepts up to 32 physical world-point
interpolation anchors. Original seam Hermite rows remain unchanged.
Anchors are solved as equality constraints in the completed QR objective
metric, including column scaling, permutation and streamed control-net rows.
The correction solves R-transpose systems and orthogonalizes at most 32
constraint rows twice; it does not form normal equations or increase penalty
weights. Dependent inconsistent anchors refuse. A fresh physical-unit
interpolation residual must be within 1e-10*(1+abs(target height)); this is a
floating-point construction check, never an interval geometry certificate.

No material or G2 receipt is inherited by the new candidate. The existing
control-net G material rejection remains authoritative for that saved surface.
The new solver is experimental until actual-candidate seam and material
inspection have completed.

Diagnostic entry: OSV_CENTRAL_GRAPH_ANCHORS points to a JSON array of [x,y,z]
world points. Saved candidate metadata records the actual anchors.

## Focused native validation

- `cargo test -p brep-core --lib central_graph`: 8 passed (3 new QR
  constraint controls plus the 5 previous fitting controls).
- `cargo test -p brep-core --test central_graph`: 13 passed, including
  nonzero hard interpolation against zero soft targets and conflicting
  anchors. The invalid-anchor test was rerun after adding NaN and
  out-of-chart cases and passed.
- These are focused regressions, not full-kernel or whole-solid acceptance.

## Actual three-anchor counterexample

The new bi-degree-9 proposal interpolates all three original witness-region
anchors with independent OCCT errors below 1.3e-16 mm. Its sampled seam
checks pass, but this supplies no fresh whole-seam certificate. Native graft
inspection again gives 13 valid trim regions and WithinTolerance closure.
Native box inspection rejects it with an Inside witness at UV
(0.10165686758737737, 0.4020356811722602), X interval
[-0.0006793832518104983, -0.0006793832518096585] mm.
Independent 151-grid exploration finds X approximately -0.000815338 mm.
The hard interpolation solver works; three isolated anchors do not solve
the continuum material obstacle. This candidate is rejected and cannot
replace reference G or enter the live document.

See [hard-anchor-summary.json](hard-anchor-summary.json) for the compact
native witness and independent anchor measurements.
