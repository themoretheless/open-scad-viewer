# Selected wall thickness and automatic candidates — 2026-10-03

The native wall API, WASM, typed worker and redesigned UI are integrated. The panel selects two disjoint groups of original faces, supplies a millimetre tolerance, displays a thickness interval and original boundary markers, and supports Enter, Esc, cancellation and Retry. Changing inputs or source cancels the job and removes the old result. Late replies and changed source/configuration reports are rejected.

## Meaning of the result

The interval bounds the minimum straight interior material chord between the selected complete face unions, subject to the requested endpoint normal angle tolerance. Every selected face pair contributes a lower bound, including pairs whose coarse bounds suffice and pairs left coarse when subdivision ends. The upper bound comes only from a chord with qualified original volume, an outside start, exactly two original boundary roots, complete boundary exclusion and complete endpoint normal coverage. A surface-gap witness cannot substitute for this material upper bound.

Convergence requires the outward width of the whole interval to meet the requested tolerance. Wrong endpoint groups, oblique lines, unresolved normals and unproven volumes cannot yield a material interval. This scope does not certify the minimum thickness of an arbitrary complete body.

## Automatic search

The UI samples original face points and normals, interleaves faces under a bounded candidate count, and audits each candidate through the same worker. It retains the smallest qualified upper bound and stops when the global selected-union interval converges. A source volume refusal stops the search, because repeating line candidates cannot fix unchanged source geometry. Samples alone never certify the regions between them.

Canonical cuboid 10 mm and the six-by-six annular cylindrical wall unions 15 mm pass. The expanded 54-pair transition union retains a wide interval under exhausted work. The tests cover oblique/through-hole refusal, reversed shells, invalid/changed groups, incomplete normals, forged sources, cancellation, late replies and recovery through a fresh worker.

Placement-aware radial estimates use matching shared finite origins for both compared faces. Nine distance regressions pass, including translated/reoriented complete annular wall unions and a one-cell coarse-bound test. Rotated/translated cuboid search passes; annular candidate geometry is covariant under rigid placement. The curved volume qualifier still refuses the placed partial-annular specimen, including a full-budget native trial (71.14 s). The explicit bounded-refusal regression preserves its distance evidence and emits no material interval. `placed-limitation.json` records the limitation.

## Packaged and UI evidence

Final WASM: 9,842,562 bytes, SHA-256 `cff6a19a77de3f20241aa1f8ea1c3d57fe4e6dc5e9372908e3df407c53f94798`.

Five actual WASM cases, five real worker-handler cases and automatic cuboid/annular searches pass. Final mouse and keyboard browser scenarios each produce nine checked results, including an oblique wall refusal with red scene markers; group selection, original-face markers, direct and automatic cancellation, localized failure/Retry, unchanged document export and exact reload pass. Detailed reports are compressed under `wasm` and `browser`, with inspected screenshots.

Remaining: automatic grouping and complete wall-domain coverage, stronger original curved-volume qualification after rigid placement, general minimum-thickness qualification and local defect coverage between samples. Independent controlled bracket/flange/enclosure wall and radius gauges pass in the new-design acceptance; those gauges do not replace whole-body coverage. General P0–P3 completion remains open.
