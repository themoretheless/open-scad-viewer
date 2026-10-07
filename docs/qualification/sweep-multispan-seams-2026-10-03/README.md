# Exact multispan sweep seams — 2026-10-03

A common **clamped along-seam B-spline basis** is now supported by the native exact strip audit. The cross direction remains a single Bezier span. The same authored along-seam knots, degrees and control count are required; periodic/unclamped or different bases are refused. Both strip directions remain bounded to at most 33 controls. Different Bezier along domains retain the existing normalized-parameter support.

The exact predicate compares original weighted cross-jet coefficients, not sampled positions or rounded decompositions. A shared B-spline basis gives equality over its whole domain. Positive weights permit the homogeneous identities to imply rational C1/C2 identities. An independent outward interval audit proves seam regularity on every span. Internal along knots must supply at least the requested continuity order, so a C1 multispan basis cannot claim C2. This is a sufficient represented C1/C2 certificate implying the requested G1/G2 for these regular seams; it does not solve arbitrary geometric reparameterization or moving frames.

Native tests passed **3/3**. Public WASM tests passed **34/34** across exact strip jets, sweep audit and progressive miter. Real Chromium passed **11/11** checks through TypeScript → binary transport → delivered native WASM. Cases cover rational C2 in both U/V strip orientations, C1, one-ULP mismatch, knot mismatch, unsupported unclamped basis, exact-work exhaustion, singular exact jets, all declared seams, and shared budget exhaustion.

`browser.json` identifies the actual browser and raw WASM hash. `manifest.json` binds the observed source files and packed/raw artifacts; it is not a clean-tree all-source reproducibility proof. Native/TypeScript logs preserve the passing run. WASM used the required Binaryen 116 size pass with the matching installed native optimizer.

Sharp miter corners continue to have C0 semantics. Full moving-frame/all-mode/closed global smoothness, source-family continuousBound and independent STEP seam qualification remain open.
