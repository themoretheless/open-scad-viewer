# Body diagnostic interaction and focus recovery, 2026-10-02

## Changes

The command palette restores prior focus on close only when focus remains inside the palette or returns to the document body. An executed command can keep its deliberate destination focus. This fixes the actual browser failure where body diagnostics focused its panel and palette teardown subsequently moved focus back to the launcher. Canceling the palette still restores the launcher.

Section input errors and the XY plane recovery button now appear immediately under the plane normal fields. The previous ordering placed correction text below mesh report rows and outside the initial visible panel. The final keyboard screenshot was inspected: the invalid group, explanation and recovery button are all visible together.

## Proof

- Final native Apple Metal / Chrome Canary 157.0.8081.0 mouse and sequential keyboard scenarios pass with WebGPU active and strict page/console error gates. Keyboard mode uses 821 Tab presses; all actions and numeric inputs use real Tab/Enter/typing. Fixture file upload uses Playwright setInputFiles in both modes.
- Both modes fail under the original palette focus behavior; those logs are retained. Both pass with the fix and after moving the error text.
- Injected display-inspection worker failure shows a localized alert; the focused diagnostic panel makes retry reachable. Retry restores the actual section.
- Invalid numeric text prevents display worker dispatch and clears the section. Zero normal shows an associated alert; XY reset restores the section.
- Escape terminates held display work. Entire downloaded documents equal the baseline after cancellation, retry and reload.
- Injected mesh-contact worker failure reports incomplete inspection and an actionable retry; retry completes on the cube fixture.
- Palette command execution preserves panel focus, while palette Escape restores launcher focus.
- Typecheck, Vite, dist, 13 focused diagnostic UI tests and all 12 command search tests pass. Dist totals 7,199,516 asset bytes; the existing budget admits the measured 85-byte focus guard growth.

## Scope

This proves the body-diagnostics interaction on one closed cube mesh. It is not general B-rep self-intersection certification or full command coverage. English/Russian diagnostic unit cases already exercise localization, while this browser fixture uses Russian. General geometry and the remaining P0–P3 work remain open. No push or RAG upload is claimed.
