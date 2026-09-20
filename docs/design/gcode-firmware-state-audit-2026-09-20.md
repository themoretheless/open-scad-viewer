# Legacy Firmware State Audit

`node scripts/audit-gcode-branch.mjs` extracts unchanged Rust sources from
`a96efc1793f17b77c5494c9a5d8b7fcd1c05ff8b`, compiles them directly with the
repository toolchain and runs `scripts/fixtures/gcode-legacy-state.rs`.
No workspace overlay, production source edit or dependency download is used.
The temporary directory is removed afterward. The JSON report includes every
source SHA-256, fixture SHA-256 and compiler version. Local report:
`/private/tmp/osv-gcode-state-audit.json`.

## Observed Defects

With absolute E, `G1 X10 E5` followed by `G1 X20 E6` reports 6 mm total.
Inserting G10 between them reports 7 mm: the legacy interpreter subtracts
one from its programmed E coordinate, so the second move advances by two.
Inserting both G10 and G11 instead reports 6 mm, masking the defect in a
simple retract/recover pair test. This concerns logical E deltas, not net
deposition or a prediction of physical extrusion while retracted.

`G10 P0 S210 R210` also produces 7 mm in that fixture: the old command
classifier mistakes the temperature-setting form for firmware retraction.
The current job emitter itself uses that temperature form for RepRapFirmware.
Neither source-header hints nor a universal G10 handler suffice to distinguish
all firmware dialects safely.

`M104 S210; M109 R180` (separate lines) returns `hotend_c = 0`, `wait = true`.
The classifier reads only S and supplies zero when absent. A targeted
`M104 T1 S210` followed by T0 retains 210 in the single hotend scalar while
changing its tool field back to zero. There is no independent per-tool target
state; that representation cannot reliably describe a multi-tool timeline.

Primary references inspected on 2026-09-20:
[Marlin retraction implementation](https://github.com/MarlinFirmware/Marlin/blob/2.1.x/Marlin/src/feature/fwretract.cpp)
keeps per-extruder retract offsets separate from the logical destination;
[M109 documentation](https://marlinfw.org/docs/gcode/M109.html) distinguishes
S heating-only waits from R heating-or-cooling waits and permits target indices.
The audit does not execute firmware or validate a printable file.

## Integration Contract

- Keep logical E, physical retract offsets and cumulative positive advance
  distinct. Do not mutate programmed E simply because G10/G11 was observed.
- Require explicit supported firmware semantics before interpreting ambiguous
  commands; do not fabricate a default retract length from an unrelated profile.
- Store bounded per-tool targets and retraction state. A heater target selector
  is not a motion tool change. Unknown prior targets must remain unknown.
- Separate requested targets from measured temperatures and wait policy from
  elapsed time. Record source line/move position for timeline attribution.
- Handle S/R and no-new-target waits explicitly; unsupported autotemp, units,
  presets or firmware extensions need visible refusal/unsupported semantics.
- Bound event count and serialized output in addition to input/tool budgets.

Two current native tests preserve logical E/material accounting for skipped
G10/G11 and temperature-form G10 across absolute, relative and volumetric modes.
They protect the existing tolerant preview contract, not implementation of
physical retraction or thermal history. The legacy audit's passing assertions
mean the old defects were reproduced, not that those features are acceptable.

Production behavior and WASM are unchanged by this audit. The branch remains
preserved and unmerged pending a replacement satisfying these contracts.

Validation: two independent audit runs produced identical reports; all 82
current `gcode-core`/`gcode-optimize` tests passed. Script syntax and diff
whitespace checks pass. The full JS suite was not rerun for these audit/test
changes; its preceding run had nine known qualification-binding failures.
