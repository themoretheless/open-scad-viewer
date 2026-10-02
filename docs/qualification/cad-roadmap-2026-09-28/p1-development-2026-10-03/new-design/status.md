# Redesigned shell and CAD acceptance — 2026-10-03

The CAD branch integrates the app redesign from the first-parent diff of `27218913` and the menu overlap fix from `a86940cb`. The complete Solid command catalog, repeat handling and command dispatch block match the previous CAD branch exactly; `command-preservation.json` records its source hash. This is patch integration, not an ancestry merge of main.

Runtime verification uses the built app with its new global command button. Material/wall checks pass with WebGPU by mouse and keyboard. The three-part 20-edit acceptance passes by mouse with the CPU fallback and by keyboard with the browser's default renderer. Screenshots were inspected. Local preview was started at http://127.0.0.1:5175/ from `/private/tmp/open-scad-viewer-cad-resume`.

## Controlled three-part acceptance

Each bracket, enclosure and flange receives 20 distinct edits: six rounds of Push/Pull, Boolean and transform, followed by the supported exact fillet and transform. Seven previews are cancelled for each part. All 20 Undo states, all 20 Redo states, unchanged cancelled previews and exact reloaded documents pass. Original topology identities remain unique and match the expected dimensions at every committed step. The compressed history archive includes 422 reviewed documents / 140 unique payloads.

`step/mouse` and `step/keyboard` contain actual final browser exports and their import/export cycle, six STEP files each. OpenCascade independently admits their solids, measures bounding boxes and integrates their volumes; maximum bounding error is approximately 1e-7 mm against a 1e-6 mm allowance.

Additional OpenCascade intersections with original STEP faces pass 16 gauges for each interaction's exports, using dimensions specified independently of the native B-rep:

- bracket webs: 5 mm; corner radius: 1 mm;
- flange bore: radius 6.5 mm; outside: radius 20 mm; fillet section: radius 1 mm;
- enclosure opposing walls: 1.4 mm; bottom: 2 mm; corner section: radius 1 mm.

The full 20-edit browser histories used WASM `38805d9eb86f32cde47b0ac19ad52cd068dfca6768d5a97229c5f53d63a8d0b6`. The final wall delivery adds placement-aware distance lower bounds. Final packaged wall mouse/keyboard evidence records its own exact WASM hash; the supported part histories are also repeated through the final packaged kernel's service tests.

## Verification and remaining scope

349 UI/history/STEP tests pass. Final targeted wall/search/theme tests: 29 pass. Part service histories: 17 pass. Native B-rep suite: 737 pass / 3 ignored; native bridge suite: 353 pass / 1 ignored. A subsequent bounded placed-volume refusal regression passes separately. Distribution byte limits remain enforced, with the prior small headroom retained after measuring each change.

This qualifies the declared controlled workflows and fillet classes. The complete 95-command interaction/error matrix, general curved fillets and complete arbitrary B-rep wall/volume coverage remain open. In particular, a rigidly placed partial-annular specimen has qualified distance bounds but unproven curved volume evidence; it cannot receive a material thickness certificate. See the material wall status and placed limitation evidence.
