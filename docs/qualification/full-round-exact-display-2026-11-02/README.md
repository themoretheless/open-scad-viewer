# Source-owned exact Full Round display sampling

`cad_full_round_exact_display` restores and proves the complete frozen artifact
before sampling its admitted Cartesian-product extrusion. There is no f64 BRep
conversion. Exact rational homogeneous controls are evaluated by rational
De Casteljau; only final Cartesian display points are rounded. Straight spans
use their endpoints. Curved spans use 1..32 samples in their natural clocks.

The cap triangulator and ruled walls share the same indexed profile samples.
Through-hole rings are retained. Triangle face indices refer to the actual
rational topology, with source-derived topology face IDs returned separately.
Profile/triangle budgets, finite coordinates, binary64 point collisions,
closed/oriented mesh checks and the existing Float32 collapse/reversal checks
can refuse display without changing the exact geometry.

This mesh is display data. Uniform sampling has no Hausdorff error certificate,
self-intersection proof or exact mass certificate. The returned polygon report
retains its uncertified diagnostics. This is limited to the admitted exact
extrusion family, not arbitrary rational BRep rendering.

A TS transport helper is provided separately. The application's scene builder
still refuses exact output until exact artifact snapshot/edit ownership is
integrated. The packaged kernel is unchanged, and this change has not yet
received a fresh WASM/worker qualification.
