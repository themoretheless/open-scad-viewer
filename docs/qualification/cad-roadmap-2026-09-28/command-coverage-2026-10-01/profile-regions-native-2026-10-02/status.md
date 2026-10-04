# Exact profile region operations — native browser qualification

Six final scenarios pass: union, difference and intersection, each with mouse and keyboard, Chrome Canary 157.0.8081.0. The active WebGPU layer and zero page/console errors are checked. Reports pin production component/source/WASM hashes. Keyboard command search, numeric extrusion input and target selection now use real keyboard events rather than fill/selectOption shortcuts. Theme selection and native file input are fixture setup.

Each scenario checks preview, cancel, Apply, Undo, Repeat, 5 mm extrusion, current STEP export, undo of body/profile, JSON reimport and real page reload. Document comparisons cover the complete exported object, not just sketches. Difference also changes the target to the inner circle, verifies empty-result refusal and changes back to the plate. Keyboard Tab counts: union 788, difference 696, intersection 785. Preview actionability has one trial-only mouse call which dispatches no modeling action.

Independent OpenCascade 8.0.1.0.0 reads all six exported STEP files as one valid solid. Expected volumes: union (42+2*pi)*5 = 241.4159265 mm³; plate-minus-circle (48-4*pi)*5 = 177.1681469 mm³; intersection 4*pi*5 = 62.8318531 mm³. Maximum bound deviation ~1e-7 mm, maximum volume deviation ~4.86e-9 mm³. The native extruded plate-with-hole screenshot was inspected.

## Failures and remaining scope

Initial runs encountered the old empty-result message expectation and timed out waiting for visible CPU polygons after deliberate GPU destruction. This is not proof of successful GPU-loss recovery. The qualifier retains that path behind --gpu-loss; the final six runs exercise the active WebGPU path. The initial local failed-run directories remain outside the committed final evidence. GPU-loss recovery for these scenes remains unqualified and must be investigated separately.

These fixtures cover lines/circular arcs in XY. General NURBS Boolean regions, arbitrary topology/planes, stale-worker responses, Redo and the complete 95-command matrix remain open. The current six scenarios do not close P0–P3.
