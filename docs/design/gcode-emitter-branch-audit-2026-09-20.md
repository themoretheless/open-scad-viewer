# Legacy Emitter Integration Audit

The existing read-only `node scripts/audit-gcode-branch.mjs` now exercises
the unchanged emitter at a96efc1793f17b77c5494c9a5d8b7fcd1c05ff8b as well.
It records original source hashes, fixture hash, compiler version and results.

For a single open path from (10, 0) to (20, 0) mm, layer Z=0.2 mm:

| Profile | Last deposited X observed by legacy parser |
| --- | ---: |
| Default absolute millimeters | 20 mm |
| Relative XYZ | 30 mm |
| Inches | 508 mm |
| Default with start template G91 | 30 mm |

Independent output assertions confirm identical X10/X20 and F7200 words in
all four outputs, despite G91/G20 declarations. Coordinates/feedrates are not
converted to the selected units or relative deltas. The last deposited move
is inspected because the default epilogue issues G28 and resets position.
The emitter also successfully returns text containing NaN for a NaN endpoint.
These are reproducible software defects, not printer execution results.

The current emitter instead fixes G21/G90/M82/M200 D0 explicitly and validates
finite bounded path coordinates. Its job API already supplies flavor-specific
heating, retract/recover and fan commands. The branch's broader profile is not
a compatible replacement: relative/inch output and arbitrary templates need
state-aware emission and independent round-trip checks before integration.
Chamber heating, z-hop and output-to-writer remain separate candidates; this
audit neither proves them correct nor discards their source history.

Production code and WASM are unchanged. Native gcode-core/gcode-optimize tests
pass. The branch remains preserved; this evidence narrows the unresolved
integration decisions rather than claiming a completed merge.
