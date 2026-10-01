# Exact sphere WASM and browser qualification — 2026-10-01

Packaged geometry WASM: 9,557,745 bytes; SHA256 `b2ebc7ddc537db72d15b0449123d506cfa75a15cd47e788b07c38a3f6146eb06`.

Validation: 24 tests passed without skips across solidSelfIntersection, solidDistance and solidVolumeDistanceUi; Vue typecheck, Vite production build and distribution verification passed. Distribution: 137 artifacts, 7,153,462 asset bytes and 11,613,865 raw WASM bytes. The asset budget increased by 4,000 bytes for a measured 3,992-byte feature increment; individual file limits remain enforced.

Mouse and keyboard browser runs passed for seven distance cases, cancellation, invalid target localization and unchanged exported models. Sphere distance is bounded by [1.9999999999999991, 2.0007439550628416] mm at 0.001 mm tolerance. The viewport shows both original-face witnesses and their connecting line. Keyboard run used 3,398 Tab actions.

Actual WASM scale qualification covers five separated sphere pairs, containment inside radius 786432 and rejection beyond the supported coordinate bounds. Native radius-family evidence is recorded separately.

Both face-diagnostic browser runs passed. All eight sphere faces have certified perspective injectivity at 968 work units; a 967 limit leaves the final face unproved. The general face-pair diagnostic still reports absenceProven=false because not every pair is classified. This differs from the specialized exact boundary certificate used by volume validity.

Screenshots were inspected. Tessellation remains visibly faceted. Arbitrary radii, rotations, general self-intersection absence and the complete P0–P3 roadmap remain unqualified.

Current-kernel history regression: all 17 cadRoadmapHistory tests passed (29.67 seconds), including 20 mixed edits for bracket, flange and enclosure and exact history snapshots. This is runtime evidence; a new keyboard browser history run remains in progress.

Current-kernel P0 keyboard crash/retry qualification passed for bracket, enclosure and flange: 3,458 Tab actions; real worker error event, new-worker retry, command focus, repeat, late-reply cancellation, Undo/Redo, reload and zero page errors. Evidence: current-kernel-preview-crash-retry.json. This covers cap Push/Pull only.

Current-kernel keyboard browser history qualification completed successfully: bracket, enclosure and flange, 20 edits each, full Undo/Redo snapshots, late-reply cancellation, reload and no page errors; 41,096 Tab actions. Evidence: current-kernel-history20-keyboard.json. This proves this cap-edit scenario, not every supported command or every control-part operation sequence.
