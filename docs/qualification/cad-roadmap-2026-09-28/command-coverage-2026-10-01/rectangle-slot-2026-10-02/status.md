# Numeric rectangle and retained slot — 2026-10-02

Rectangle authoring accepts a lower-left local point and positive width/height. Slot authoring accepts two end-cap centers and full width. Quantities use the existing unit parser. Preview does not modify the document. Create commits once, Escape cancels, and invalid parameters block creation.

Slots now commit two retained lines and four rational quarter-circle spans, including after a mouse drag. The temporary line/arc document is prepared by a dedicated CAD worker. Numeric Create consumes the prepared result without another profile calculation. Mouse release requests automatic application; cancellation, parameter changes, changed document and closed workspace invalidate responses. Errors are localized and provide Retry. A sticky command header keeps worker errors and Retry visible; native browser checks the error's bounds are inside the panel. All numeric curve previews use the green preview legend color and a 2px dashed stroke.

The authoring helper uses native arc endpoints for adjacent lines. Mixing independently computed JS normals with native trigonometry had introduced sub-ulp connector edges for diagonal slots; the final implementation avoids those artificial edges. All modeling uses the retained loops; display tessellation is not used as the modeling profile.

## Local verification

- 304 DirectModeler UI tests + 6 native authoring tests: **310 passed**, functional source before blank-line cleanup, 112.64 seconds.
- Unit scenarios include rectangle bounds, horizontal/diagonal/reversed slots and analytic area; unchanged draft, Undo/Redo, localized failure/retry, cancelled/superseded replies, mouse-drag auto-apply cancellation.
- TypeScript, Vite production build, dist verifier and diff whitespace check pass.
- DirectModeler: 399,746 bytes. Asset total: 7,218,044 bytes. Geometry WASM unchanged (SHA256 07cd41b96faa8f5bb05507c461845711c4355613c0fb856eb4f5fb20d736b5fe).

## Final native browser scenarios

Chrome Canary 157.0.8081.0, WebGPU active, no page/console errors. All seven reports pin the same final DirectModeler artifact and source hash. File fixtures use the native JSON import input. Keyboard actions use actual Tab/Enter/text events.

| Scenario | Input | Plane | Slot direction | Tabs |
| --- | --- | --- | --- | --- |
| keyboard | keyboard | XY | diagonal | 3635 |
| keyboard-face | keyboard | body face Y=-10 | horizontal | 4532 |
| mouse | mouse | XY | horizontal | 0 |
| mouse-face | mouse | body face Y=-10 | diagonal | 0 |
| mouse-gesture | mouse drag | XY | diagonal | 0 |

Every row performs 10 rectangle + 10 slot creations, checks distinct sketch identities and exact profiles, 20 Undo and 20 Redo against complete documents, extrudes the last two profiles by 7 mm, exports their CURRENT STEP, and restores the complete final document after reload. Body-face rows retain supportBodyId=base and Y=-10 supporting-plane geometry. The source body remains unchanged. Additional final-build circle/arc keyboard regression reports cover their previous numeric workflows.

Each row also verifies invalid dimensions/width/coincident centers, invalid slot input without worker dispatch, private worker error replaced by a localized message, recovery through Retry, and forced successful stale replies after supersession and Escape. Numeric rows prove Create does not recompute its prepared profile.

Mouse drag coordinates are compared to the actual captured preparation request. Browser pointer coordinates differ from ideal fractional screen targets by up to 0.000007073332 mm; this is recorded, not rounded away. Numeric coordinates remain strict. Gesture area and STEP expectations are derived independently from actual requested centers and width. An initial failing nominal-target assertion and screenshot remain in mouse-gesture/failure.* as diagnostic evidence.

## Independent STEP qualification

OpenCascade 8.0.1.0.0 reads all ten final STEP files: one valid solid each, prescribed dimensions/bounds and expected volume. Max bound error is approximately 1e-7 mm; max volume error approximately 3.12e-8 mm³. Rectangles are 10×6×7 mm (420 mm³). Numeric horizontal slots have center spacing 10 mm, width 4 mm and height 7 mm (367.9645943 mm³); diagonal slots have spacing 5 mm (227.9645943 mm³). The gesture variant uses its measured centers. Independent previews and all report hashes are stored alongside the manifests.

Actual rectangle/body-face draft, localized slot-failure, and independent OCCT preview screenshots were inspected.

## Remaining full-roadmap requirements

This closes the stated numeric rectangle/slot scenarios, not the complete 95-command mouse/keyboard/error matrix. The source inventory is a source-only snapshot and deliberately carries unverified execution fields. Arbitrary fillets/corners, general B-rep diagnosis/STEP inputs, continuous surface qualification, compound-profile editing, large-scene performance and P3 integrations remain open. The tests do not certify every parameter within the input range or all workplane orientations.
