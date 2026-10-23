# Controlled mixed-operation history qualification

Mouse and keyboard Chromium runs passed all three parts: bracket, enclosure, flange. Each runs six cycles of Push/Pull, Boolean and translation, then exact fillet and translation: 20 committed geometry edits. Seven prepared previews are cancelled per part before reopening and applying.

Every saved state has unique topology IDs matching entity counts and expected dimensions. The complete Undo and Redo traversal and final reload match full exported documents. Volume, topology and untouched future Boolean tools are separately audited for all 21 states in each interaction. Independent OpenCascade accepts current STEP and native reimport/reexport for all final parts.

The history collector independently compares 422 documents across six runs and archives 140 unique JSON byte payloads. documents.jsonl.gz stores sha256/json pairs; history-audit.json maps original filenames to hashes. Every decompressed payload hash was verified. Source fixtures and expected histories are included.

Mouse and keyboard flange screenshots and independent mouse flange preview were inspected. No runtime source change or new WASM build in this group. Baseline 9b678316, geometry WASM SHA256 07cd41b96faa8f5bb05507c461845711c4355613c0fb856eb4f5fb20d736b5fe.

This establishes these controlled 20-edit histories. It does not prove arbitrary fillets, every command, every error, all stale-result scenarios, general STEP or full P0-P3 completion.
