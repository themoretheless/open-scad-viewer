# Experimental shared surface

Choose Shared surface · experimental in the photo panel. This uses two-scale slanted depth estimation and a shared volume surface. Draft resolution 128 is currently supported; selecting this mode resets detail to Draft. Standard remains the default.

Rust-only kernel; no new production dependencies. Native/WASM triangle arrays match on shell6, shell12 and monstree6 after arithmetic-aware depth-range checks. Synthetic 160-pixel sphere area coverage remains about 88%, below the OpenMVS reference. This is not a claim of superior general reconstruction. Thin-detail quality at 160 pixels is preserved after the numerical fix; tilted/sphere cases have documented small tradeoffs.

Volume construction avoids a second node membership lookup below the node cap. Final vertex validation stops after enough cameras agree. Neither change relaxes support or geometry rules. The bounded continuation radius remains 0.75 pixels; a 1.0-pixel experiment improved coverage but reduced precision and was not promoted.

The integrated browser panel was exercised with real JPEG input and its worker: successful shared-surface reconstruction, two dense-stage cancellations followed by a successful restart, and a parsed PLY export with valid indices. These checks do not prove every cancellation timing, absence of memory leaks, or broad reconstruction quality.

Simplification functions are included in the kernel but are not exposed by this panel. Large-input count caps are not memory guarantees. The experimental surface may contain holes or artifacts. Research and integration evidence remain in scan/optimization/photogrammetry-2026-09-09/reports/shared-surface and scan/optimization/photogrammetry-2026-09-09/integration.

For experimental dual-scale reconstruction, a curved edge with one missing neighboring depth can use the measured slope on its other side. A present but incompatible neighboring depth still rejects this fallback; two missing neighbors do not establish support. Centroid validation from other cameras remains required. The 160-pixel synthetic sphere gains about 0.78 percentage points of visible-area coverage. Thin-fixture area precision loses approximately 0.008–0.010 percentage points at 80/160 pixels, with unchanged ribbon recall. This is a measured quality tradeoff, not a universal accuracy improvement.

At the browser's 128-pixel limit, the same sphere test improves visible-area recall from 84.80% to 85.34% and grazing recall from 48.08% to 49.53%. Thin recall is unchanged. At 64 pixels, total sphere recall improves slightly but two grazing targets are lost; the rule is not better in every metric. Detailed 64/80/128/160 results are retained in the research reports.


Volume validation reuses camera-space coordinates and specializes interpolation at compile time to compute depth without unused confidence/color attributes. Integration still computes all attributes. The depth validity, triangle domain, bounded continuation and view-support rules are identical. No new runtime dependency or UI setting is added. Comparative evidence is in the scan workspace report `volume-depth-only`; performance percentages apply to the validation stage, not the entire reconstruction.
