# Retained profile offset — native browser qualification

Mouse and keyboard scenarios pass in Chrome Canary 157.0.8081.0 on the current pinned production source/component/WASM. Keyboard command search, offset distance and extrusion height now use actual keyboard events; earlier script fill calls did not establish keyboard input coverage. Keyboard run uses 937 Tab events.

Fixture: 8x6 mm rectangle with a 1 mm-radius circular hole. Offset +0.5 mm (entered as 0.05 cm) rounds the outer corners and reduces the hole radius to 0.5 mm, preserving two retained loops and the profile identity. The independently expected area is 48+28*0.5+pi*0.25-pi*0.25 = 62 mm². Excessive inward offset -10 mm is refused with a localized corrective action.

Scenarios compare complete exported documents through cancel, Apply, Undo, Redo, Undo then Repeat, JSON import and actual page reload. They extrude 5 mm, destroy the Solid canvas GPU device, require CPU fallback, save the unchanged document, retry WebGPU and compare the restored document. Zero page/console errors. The native extruded rounded plate screenshot was inspected.

Independent OpenCascade 8.0.1.0.0 accepts both current STEP files as one valid 14-face solid, expected bounds [-0.5,-0.5,0] to [8.5,6.5,5], volume 310 mm³. Maximum bounds error ~1e-7 mm, volume error ~5.96e-9 mm³.

Initial local runs had an outdated error-text expectation. A subsequent run read the worker counter after page reload, which reset that instrumentation; the counter is now captured before reload. Verified runs in verified/ record five profileEdit worker requests each. Failed runs remain locally outside committed evidence.

Scope is this retained line/arc profile and offset amount in XY. General NURBS offsets, other topology/planes, stale-reply/error-code matrices, Curve parameters browser qualification and full P0–P3 completion remain open. Product source is unchanged.
