# Arc and NURBS profile preparation — native qualification

Six final browser scenarios pass: semicircle plus diameter, polynomial quadratic NURBS plus three sides, and rational quadratic NURBS with weights [1, 0.8, 1] plus three sides; each uses mouse and keyboard. Source/component/WASM hashes pin the current production build, Chrome Canary 157.0.8081.0. Native worker provenance accounts for every retained span and no artificial connector. Rational joined profiles retain the original weight array.

Scenarios exercise preview, complete-document cancellation, Apply, Undo, Repeat, 5 mm native extrusion, current STEP export, JSON import and actual page reload. GPU loss targets the Solid canvas device, displays CPU geometry and supports Retry through keyboard/mouse with complete unchanged documents and selected body. No page/console errors occur. Native rational preview screenshot was inspected.

OpenCascade independently verifies six STEP files as one valid solid each, prescribed bounds and volume. Semicircle volume 10*pi = 31.41592654 mm³; polynomial case 70/3 = 23.33333333 mm³; rational case 23.02779831 mm³. Rational area oracle evaluates the analytic rational Bezier derivative with 20,000-interval Simpson integration, independently of our geometry kernel. Rational minimum y = -0.8/1.8. Maximum bound error ~1e-7 mm; maximum volume error ~4.3e-10 mm³. This numerical integration is not a certified general NURBS area bound.

Initial arcs-keyboard run exported before asynchronous import completed because the first input name remains unchanged. The qualifier now waits for the consumed second input to disappear and the restoration status to finish. The terminal failed-run artifacts remain locally in arcs-keyboard; final proof uses arcs-keyboard-final. The other five final runs passed before that synchronization change and include their complete successful snapshots.

These fixtures qualify degree-two open curves in XY with positive weights. General degree/topology/planes, NURBS offsets and region Boolean operations, all parameter ranges, independent loft, whole-interval sweep and G1/G2 remain open. This does not complete P0–P3.
