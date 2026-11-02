# Natural-root cuts: recipe, WASM and worker

2026-11-02.

The existing version-1 frozen recipe remains unchanged. Version 2 carries nonempty naturalCuts in a separate ordered cut list. Restore validates canonical rational parameters and bounded combined cut count, rebuilds original transition geometry, cuts natural source roots, sews neighboring faces, and freshly proves volume/embedding/G2/curvature. Saved geometry and identity links must equal the recomputed result. No receipt or saved solid flag substitutes for proof.

TypeScript and worker receipts bind the requested format as well as exact geometry. V2 replies cannot silently downgrade to V1. Existing read-only inspector uses the same restore/display protocol; document application remains disabled.

## Evidence

- Native: 24 tests passed (exact_graph_strip 19, frozen_graph_volume 5).
- Actual separate worker, transport and panel: 13 tests passed. The new actual worker test restores/displays V2 with 39 edges, rejects duplicate cuts and successfully retries.
- vue-tsc and git diff --check passed.
- Final packaged WASM: 16,474,875 bytes, SHA256 b2ce37f3b0dcdcc44692b8642a26b5e5782808f9172885de7c5186d2ef20714a. Embedded compressed payload explicitly decodes to this same hash and length.
- Both V1 (27 edges) and V2 (39 edges, 12 natural-root cuts) pass direct WASM and worker-handler checks. Reports attached. V2 proves one 9-face body with no open edges, global embedding, curvature upper 9.867331764661005 mm^-1, and unchanged exact signed volume 263308489/7741440 mm^3.
- Native negative cases include stale geometry, duplicate/unknown roots, endpoint/negative/noncanonical parameters, downgraded version, empty V2 cut list, exhausted work, corrupted identities and unknown profile versions.

## Limits

A direct script attempt during the still-running packaging step reported unknown V2 format. It is not counted as qualification. Final successful runs occurred after terminal build success and now explicitly bind the embedded payload hash too.

Browser automation could not attach to the existing verification tab (browser control unavailable); no new visual acceptance is claimed. UI checks here are component tests and the actual separate geometry worker, not browser rendering proof. Handler timings are warm local measurements, not UI latency.

This remains the supported polynomial source construction. General rational NURBS, arbitrary source-domain trimming/holes, mixed-radius complex junctions, radius-law/unrestricted thickness, document Apply, STEP/independent part measurements/history and remaining P0/P2/P3 are open.
