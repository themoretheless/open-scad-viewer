# Rush emitter snapshot migration

[Same-host Linux audit](https://github.com/themoretheless/open-scad-viewer/actions/runs/38608902537) compares old ModelGraph runtime at main 2d63a3bec and current Rush runtime at 6dff3e94a. All 81 old snapshot hashes reproduce. Current extra test documents are migrated from obsolete modelgraph/1 to canonical rush/ir-1 before comparing results; leaving the old language would silently replace eight former successes with schema refusals on non-Linux hosts.

All 81 statuses are preserved: 54 successes and 27 expected refusals. The exact comparator and all cases remain unchanged. 54 reference rows change for canonical labels/language/source headers, corrected fitted-gear comments, one signed zero, and 50 observed sketch values. Maximum observed numeric delta is 4.136198938908819e-9; all six sketch solutions satisfy their unchanged 1e-6 tolerance and all constraints. This finite comparison adds no tolerance to the bit-exact snapshot test and makes no universal error-bound claim.

All nine native packages from Rush onward and all three exact Linux clippy gates passed after the corrected snapshot. This covers the tail that the full workspace run could not reach after its snapshot failure. Full CI at the new source remains a separate delivery gate.

Only the extra-case language literal in the Rust test and its reference TSV change. No production Rust algorithm changes. The post-fixture complete WASM build pipeline succeeds with byte-identical geometry, language, VR and photogrammetry artifacts. The new source record explicitly binds both emitter fixture files.

Extract payloads.tar.gz and run python3 verify-payloads.py /absolute/extracted/directory to verify every FNV/length pair, all 81 statuses, exact wire structure, allowed source-text changes and observed sketch tolerances. Raw original/current payloads and native/lint logs are preserved. No historical G1 result supplies credit to the successor.
