# Current UI history check

Bracket: applied ΔX = 2 mm through the exact numeric panel in the new UI, exported JSON, clicked Undo and exported again, clicked Redo, reloaded and exported again. Undo JSON equals the archived baseline structurally; reloaded JSON equals the applied document structurally. All four files and hashes are retained.

Scope: one translation and mouse history actions. The full 20-edit mouse/keyboard matrix, remaining parts, current-UI STEP chain and crash diagnosis remain open.

## STEP from current UI

Exported the restored, edited bracket through the current UI STEP command. Independent OpenCascade 8.0.1.0.0 confirms one valid solid, 25 faces, expected bounds [9,0,0]–[55,30,26] within 1.01e-7 mm and volume error 1.04e-6 mm³. All authored topology IDs are unchanged in the UI JSON; every original vertex moved exactly +2 mm on X within 1e-12 mm. See occt-report.json and occt-preview.png.

## Twenty current UI edits

Applied twenty +0.1 mm numeric X translations to the previously restored bracket. Twenty keyboard Meta+Z actions restore the starting JSON exactly; twenty Meta+Shift+Z actions followed by reload restore the final JSON exactly. Topology IDs are unchanged and every vertex translates +2 mm within 1e-10 mm. This checks one command class; cancellation and the complete command matrix remain open.

## Flange current UI chain

Imported the archived flange fixture in the isolated UI origin, applied twenty +0.1 mm Y translations, exported JSON, used twenty Meta+Z and exported baseline, used twenty Meta+Shift+Z and reloaded. Baseline and Undo documents match exactly; final and reloaded documents match exactly. Topology IDs are unchanged; every vertex translates +2 mm Y within 1e-10 mm. Exported STEP through the actual UI after reload. OpenCascade confirms one valid solid with 20 faces, expected bounds [-13,-18,0]–[27,22,12] within 1.01e-7 mm and volume error 3.16e-6 mm³. This checks translations and history, not the full Boolean/fillet construction chain or cancellation.

## Enclosure history failure

Twenty +0.1 mm Z edits produced the expected +2 mm result. Repeated keyboard Undo stopped with +0.3 mm remaining and the Undo button disabled. The baseline was not restored. DirectHistory evicts older full serialized snapshots above 16,000,000 retained characters, explaining the lost early edits for this larger document. This scenario fails acceptance; compact history storage is required. Artifacts preserve the baseline, edited document and observed Undo result. Redo/reload/STEP chain was not claimed for this failing case.

## Enclosure after compact history fix

Repeated the failing scenario in the actual new UI with shared immutable history text. Twenty +0.1 mm Z edits, twenty Meta+Z and twenty Meta+Shift+Z now complete. Undo JSON equals the baseline exactly; final JSON equals the reloaded document exactly. Topology IDs remain unchanged; each vertex moves +2 mm Z within 1e-10 mm. UI-exported STEP is one valid OpenCascade solid with 106 faces; expected bounds [7,0,2]–[47,30,28] match within 1.01e-7 mm, volume error 1.01e-6 mm³. Original failing artifacts remain for regression provenance. Full mixed-operation construction, preview cancellation and general geometry qualification remain open.

## Transform preview cancellation

In the actual UI opened Transform selection on the restored enclosure, entered X=5 and pressed Escape. Exported JSON exactly matches the prior document. Reopened Transform selection, entered X=1 and applied with Enter; topology IDs remain unchanged and all vertices move +1 mm X within 1e-10 mm. This does not force a late worker reply. Mouse Apply clicks left the card open in this run, while keyboard Enter succeeded; the mouse path remains unqualified and needs investigation. Existing hidden Rush/WebGPU alerts remain.

## Mouse Apply investigation

Reproduced the transform Apply button leaving the command card open in a second current UI run. An experimental shared mouse/keyboard request handler queuing apply against the current worker generation did not resolve the symptom and was reverted. No speculative fix remains. Keyboard Enter is proven by the preceding JSON check; mouse event routing remains open. Type checking passed for the experiment, which is not acceptance evidence.

## Mouse Apply fixed

Screenshot inspection showed floating command guidance overlapping the Apply button. Moved guidance into its own compact layout row. A single mouse click now closes the transform card and commits +2 mm X; saved JSON proves all vertex deltas within 1e-10 mm and unchanged topology IDs. Earlier worker-queue hypothesis was rejected; no experimental handler remains.

## Async/history regression after integration

Ran directModeling, directHistory, historyText, enclosureHistoryRetention, solidDraftStore, solidDraftHeadStore, sourceExpressionArchive and expressionInspectionController suites: 37 checks across eight files pass (4.50 s). Inspected tests cover cancelled/failed/mismatched restore, superseding edits, competing requests, ownership of returned geometry, recovery cancellation, draft-head coordination and inspection late success/error after cancellation/disposal. These are deterministic controller/storage tests, not the full command matrix or physical multitab/browser crash qualification. vue-tsc --noEmit also passes.

## Geometry-kernel startup retry

DirectModeler distinguishes failed WASM warm-up from failed document restoration. A failed warm-up leaves the stored draft untouched and displays a localized kernel loading error with Retry. Retry resumes the initial recovery intent with the existing generation/document/storage guards; cancellation clears the loading state and stale completions cannot overwrite it. Two component controls pass: ordinary deferred warm-up with restoration, and failed warm-up followed by successful Retry with one restored body and no extra undo state. Type checking passes. Full component suite is running; this does not diagnose or repair the browser renderer crash, and no browser fault-injection check is claimed.

Full DirectModeler component suite completed: 318/318 passed in 62.50 seconds, including the new retry scenario. Type checking and whitespace validation pass. This is component-level coverage, not the full real-browser command matrix.

## Kernel cancellation and repeated failure controls

Added component fault controls for both late success and late failure after recovery cancellation, and repeated warm-up failure after Retry. They verify the saved serialized draft remains unchanged, cancellation remains visible, stale failures do not introduce a new kernel error, and Retry remains available after another failure. Disabled primitive commands now explain whether the kernel is loading or requires Retry. These are deterministic component fault controls; browser renderer crash recovery and the full command matrix remain open.
