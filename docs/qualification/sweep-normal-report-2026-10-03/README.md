# Normal-field gate repairs — 2026-10-03

Local unpublished work; the full sweep/miter goal remains active.

Draft angles now union local cross-product-ball direction enclosures against the actual pull direction instead of deriving the result from one broad global normal cone. Curvature uses finer interval cells (128 subdivisions per knot span and axis). A regular rational patch with an exactly constant authored control coordinate receives the exact planar zero-curvature identity; sampled or tolerance coplanarity never promotes this result.

All five normal_cone tests passed (16.73 seconds): sampled normal containment, silhouette rejection, sphere curvature, spherical and planar offsets, cylinder/cone draft. Latest observed full primary nurbs-core library suite passed: 717 tests, zero failures, 39.29 seconds. Test inventory changed through concurrent project edits; this proves only that observed native snapshot, not every former test or full workspace.

A frozen current native snapshot is at /private/tmp/open-scad-viewer-sweep-native-final-2026-10-03. Geometry WASM build from it is running; packaged-runtime correspondence and Rush/STEP/UI checks for that new artifact remain pending. Previous packaged runtime has separately qualified 27 STEP cases and 44 wide/narrow UI scenarios with 480 assertions. General all-mode guarantees and publication/CI remain incomplete.
