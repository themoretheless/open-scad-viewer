# STEP v9 topology identity regression — 2026-10-09

The archived mouse bracket after 20 edits was re-exported/imported with packaged WASM `5f6001640ee46ec3640d980ed4292c58419fc6b1f8100b4b909ccc4901fbc9df`. The importer reported preserved=false, createdCount=167, preservedCount=0, source=assembly-definition-plus-occurrence. The diagnostic and failing STEP are retained here. This is a failed qualification, not a passing exchange report.

The Rust importer unconditionally regenerated identities for every v9 import. It now restores validated original metadata for one occurrence and retains occurrence-specific identity generation for repeated instances. The native regression failed before the fix and passed after it. It compares identity sets for vertices/edges/loops/faces/shells/bodies and verifies each vertex identity remains attached to its original point; array order is allowed to change during STEP topology traversal. All 31 STEP-module tests passed, including assembly placement and refusal cases.

Fresh WASM packaging and the independent OpenCascade measurements are still pending at this checkpoint. The historical UI archive is previous evidence; current UI Undo/Redo/reload has not been rerun by this CLI check.

Fresh packaging and acceptance subsequently completed successfully. See `../cad-parts-step-identity-fixed-2026-10-09/` for the 12 current STEP payloads, preserved identity checks and passing independent measurements.
