# Rush-to-Solid circle correction qualification — 2026-10-03

Explicit circle_correction_tolerance (mm), circle_correction_quantum (power-of-two mm, default 2^-40), circle_correction_max_work (shared section budget, <=1000000) now lower through native Rush, embedded JSON schema, language WASM and TypeScript into progressive B-rep construction. The schema embedded in modelgraph-runtime was updated together with the TypeScript schema. New example: examples/rush/progressive-miter-circle-corrected-hollow.r. Quantum/work without a displacement tolerance refuses.

Language WASM installed SHA256 a19a753d547682f87aa742a412db8cb29106d6bd7ae9ebb85f316c1c376c2892; geometry WASM c2fcb258eb53fc66edba14aa7d47ad0376799259cf0fabb2d5c6d3cc03fc7094. Native lowering preserves the quantity/unit; application pipeline independently rejects angle units in place of length. Three runtime tests passed, including synchronous/stream identity, shared-budget refusal and actual Rush-to-viewport/Solid evidence. Typecheck and production build passed.

Both wide (1440) and narrow (600) UI scenarios for the new mode passed: 24 assertions including explicit G1/G2 positive display, complete boundary, Solid, cancellation, source change, restore and refusal with prior-body preservation. Held dispatch tests lifecycle cancellation; no mid-kernel latency or GPU qualification claim.

Independent OCCT STEP matrix expanded to 28 cases; all passed geometry, topology and analytic-volume checks. The new corrected scale/twist example is compared against the independently integrated retained circular-family volume. Global reconciliation gives 26 positive native Solid certificates and two deliberate unproved refusals, with exact artifact hash linkage and complete wall/cap/nesting/orientation coverage.

Remaining: integrate and qualify three spatial repairs and two affine smoothness cases, full updated UI/source/artifact audit and publication/CI. Full sweep/miter goal remains active and unpublished. Runtime source snapshot /private/tmp/open-scad-viewer-sweep-circle-native-2026-10-03.
