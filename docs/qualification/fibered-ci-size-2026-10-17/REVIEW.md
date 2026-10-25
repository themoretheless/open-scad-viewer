# Public fibered source delivery and CI repair

The exact user-authorized commit 1c58ccd30431de9fb8c9193d55fdfdbcd8ea77a8
was published on codex/source-fibered-caps and draft PR58. This successor
is local; its publication requires authorization covering its exact payload.

CI found five oversized Rust files and a packed geometry module of 4,315,924
bytes above the previous 4,145,000-byte cap. The module splits preserve all
production ownership and test cases. Focused native gates passed 76 tests.

The normal rebuilt geometry WASM is 12,767,518 bytes. Its local packed module
is 4,315,061 bytes; the full distribution is 8,581,543 asset bytes across 165
files. The measured caps are now 4,340,000 and 8,610,000 respectively; other
chunk caps, geometry tolerances, proof work budgets and production compiler
profiles are unchanged. The build log retains its original failure on the old
total cap; the separate verify-dist log records the successful final check.

Three smaller compiler profile candidates were measured in eight alternating
pairs on three Rust certificate requests. They slowed median ABI execution by
roughly 32-52 percent, so none was retained. The public binary uses the original
release/Binaryen/Brotli recipe. The transport benchmark contains no geometry.

Four fibered, six rational-cap and four implicit-sphere public WASM cases pass
construction, native audits, STEP re-audit and typed refusals, including strict
G2 refusal of true G1 and preserved sharp C0. These are fourteen finite cases;
this does not qualify the browser, UI or full semantic G1 matrix. Earlier 3361
native and fourteen independent OCCT receipts remain tied to 1c58ccd and are
not newly earned by changing a fingerprint.

The hosted Rust job was cancelled at its 90-minute limit. A test-only opt-level
experiment is still separate evidence; the complete partition must pass before
an executor change can be reported as validated. Debug assertions and overflow
checks are retained. No CI success is inferred from local results.
