# Frozen native and runtime qualification — 2026-10-03

Frozen snapshot: /private/tmp/open-scad-viewer-sweep-native-final-2026-10-03. Local unpublished; the complete sweep/miter goal remains active.

Full nurbs-core library suite: 717 passed, zero failed, 43.63 seconds. Full brep-core library suite: 688 passed, zero failed, two ignored, 52.41 seconds. Full cad-predicates library suite: 20 passed, zero failed. Copied STEP and imported-flange fixtures resolve native include_str dependencies. The manifest hashes 1,930 native Rust/Cargo files; these results qualify the snapshot rather than the concurrently changing primary checkout.

Geometry WASM rebuilt from that snapshot: 12,015,415 raw bytes, optimized to 10,680,455. SHA256 47e6d7c9bd822449f4d845e59ba669a45dbcab1a48c9059285422a51815802e3, byte-identical to the previous qualified artifact. The repaired private mathematical modules do not change code reachable through the browser bridge; this is source/artifact correspondence, not browser availability of those new private APIs.

Transport and retained sweep checks: 67 tests in ten suites passed. Broader sweep suite: 24 files passed immediately; four failures in two files came from obsolete sequential Rush node IDs. Tests now find the actual framed node and use real input IDs. All eight tests in those two repaired files passed. Combined qualification covers all 134 tests across 26 files; the original failing run and repaired focused run are both retained.

Independent OCCT STEP matrix rerun: all 27 fixtures passed surface/control-net, edge/pcurve, topology, nesting/material orientation and analytic-volume checks. The new G1 quartic profile retains exact native Solid admission and volume 512/7 mm³.

Production build succeeded. All 135 dist files are byte-identical to the previous G1 qualification build, whose wide/narrow UI matrix passed 44 scenarios and 480 assertions. That UI evidence remains applicable by complete dist-byte identity; UI was not rerun here. Held dispatch cancellation does not measure mid-kernel latency, and headless CPU rendering is not GPU qualification.

Remaining: requirement-level global all-mode guarantee audit, applicable smoothness beyond the current sufficient exact strip identities, and publication/CI. Do not interpret finite fixture matrices as universal geometric proofs or whole-boundary G2. Sharp path miter and cap joins remain explicitly C0.
