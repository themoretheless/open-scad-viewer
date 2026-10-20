# Three-part history replay and current STEP exchange

`check-cad-history-step-current.mts --replay-history` now replays each archived
mouse/keyboard bracket, flange and enclosure geometry sequence through the
current authoritative parser and DirectHistory. Each of six chains passes:
20 commits, seven explicitly canceled asynchronous loads whose late results
leave both stacks and current geometry unchanged, 20 asynchronous Undo steps,
20 Redo steps and exact serialization/reparse of the final state. Export,
import and re-export preserve all six topology identity sets.

`transport-report.json` records the six chains and kernel SHA
`ec4828e66d4bfb072d5153d85fb12d9a7974ddb84ffb0712add3a4d648bc015b`.
The kernel is warmed and validated against its imported immutable identity;
this run is not credited to the subsequent optimizing build.

Independent OCCT line/face gauges pass all 32 specified wall, bore and
radius-section measurements across 12 STEP exports (`dimension-report.json`).
The separate shape-validity/volume/bounds verification records its outcome in
`occt-report.json` when complete. Logs:
`/tmp/cad-parts-history-replay-20261010.log`,
`/tmp/cad-parts-history-replay-gauges-20261010.log`,
`/tmp/cad-parts-history-replay-occt-20261010.log`.

Scope: current parser/history execution and current native STEP exchange of
archived geometry states. Geometry commands, mouse/keyboard events, browser
reload, general fillets and full-wall thickness are not requalified by replay.

Independent shape checks completed successfully for all 12 exports: STEP read,
one valid solid, exact bounding boxes and volume. The generated OCCT preview
was visually inspected; all 12 panels display PASS and the expected bracket,
flange or enclosure. Full numerical evidence is in `occt-report.json`.
