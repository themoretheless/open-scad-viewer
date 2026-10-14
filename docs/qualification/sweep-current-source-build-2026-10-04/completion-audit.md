# Extended sweep/miter completion audit

This audit preserves the six requested requirements and publication deliverable. A completed fixture matrix is evidence for its stated cases, not universal all-mode geometry acceptance.

| Requirement | Existing evidence | Outstanding proof |
|---|---|---|
| Affine laws + authored frame + guides through native/WASM/Rush/viewport/Solid | Frozen 9a3 current-source artifact: 40 integration tests and 74 browser cases passed, including accepted/refused moving-frame and closed combinations | Publish remaining feature integration coherently; prove any additional claimed combinations before admission |
| Complete continuousBound, including walls, frames, miter, decomposition, filled caps and corrections | Previous report: complete boundary bounds for accepted open/closed corrected fixtures; closed bodies have empty cap scope | 9a3 current-source fixtures passed; arbitrary frame/weight and cap configurations are not established by these sufficient proofs |
| Regularity, intersections, holes, nesting and shell orientation in every claimed mode | Independent actual-BRep material/boundary/volume audits and 43 STEP fixtures verified for 9a3 | Universal geometric embedding is not proved by finite fixtures |
| Applicable G1/G2, moving frames, multispan surfaces and closed seams; sharp miter stays C0 | Previous report: complete 128 closed seams, 96 G2 soft joins and 32 C0 sharp joins; current Rust fixes pass G1 fan regressions | 9a3 regression passed; nonuniform rational fan joining remains unsupported by the polynomial solver |
| Independent STEP matrix for frames, guides and joint modes, geometry/topology/volume | 9a3 qualified artifact: 33 baseline plus 10 reconstructed cases, including three authored conic weights | Matrix completed for 9a3; repeat affected cases after native coefficient adapter migration |
| Wide/narrow UI: Solid success/refusal, build/Solid cancellation, source change and recovery | 9a3 qualified artifact: 74 cases / 852 assertions, widths 1440/600 | Matrix completed for 9a3; held dispatch does not prove mid-kernel cancellation latency |
| Publication and fresh CI | Draft PR #30 publishes native predicates, strip API and coefficient audit, head 85318706 | PR attached; CI running. Remaining feature integration/WASM/examples/reports are not all delivered by this native foundation PR. |

Goal completion remains unproven. Native test and artifact-source reconciliation is necessary but does not close the global guarantees or all-mode mathematical requirements by itself.

## Current checkpoint: native cap contours and periodic integration

Native contour coefficient matching now handles cyclic starts, opposite traversal, rational weights, malformed data and an atomic work budget. Nine focused primary tests and a runnable Rust example passed. The TypeScript cap adapter delegates matching to this operation and shares work across both endpoint caps.

Artifact `d295ea767639e878dc79a8e7e917a5eed815ee679fab49025a706fdfa3ac2029` (10,743,688 bytes) was built from frozen snapshot `1791107630`; all 3,082 recorded compiler inputs were verified unchanged. Its integration run passed 49 tests and failed one newer periodic-profile case because that frozen native loft still rejected periodic source curves. This failure remains recorded; this artifact is not installed as the complete current integration. All 35 baseline and ten smooth/reconstructed independent STEP cases passed. Its 74-case browser run is active; no final UI success is claimed yet.

Snapshot `1791108338` includes the newer native periodic-profile admission and regression. Full native units passed: 910 NURBS, 694 BRep and 23 predicates; two BRep measurement tests are ignored. Its fresh WASM build is active. The adapter and low-work refusal regression are overlaid separately. Qualification-only UI source adds periodic and unclamped profiles at both viewport widths. Repeat full integration and affected STEP/UI cases before installing this artifact; do not replace newer concurrent primary work.

Publication PR #30 head is `b78e651998cf6a9e38e86581156e3a34942a49f0`. Its predecessor failed the dist geometry chunk budget (3,321,602 bytes versus 3,305,000). The head adds 25,000 bytes to the named geometry chunk and total budget. A fresh local production build passed exact packed/raw byte checks and verify-dist; 14 fingerprint tests passed. New CI run 37194147154 and STEP run 37194147148 are active. Own-Rust v32 / G0 v40 / G1 v57 retain only observed Mac and Ubuntu artifact identities. Full sweep feature delivery remains separate from the foundation PR and is required before goal completion.
