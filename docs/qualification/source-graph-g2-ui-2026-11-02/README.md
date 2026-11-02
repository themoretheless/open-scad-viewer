# Source graph G2 preview UI

A collapsed panel appears next to the existing profile-transition inspection when exactly two NURBS surfaces are selected. It accepts half width, target radius and radius tolerance, and offers Preview/Retry and Cancel. Enter runs and Escape cancels inside its controls. A dedicated preview worker keeps cancellation isolated from other CAD clients.

On a valid response, the viewport shows a display wireframe (ten isocurves) and a red radius-witness marker. Isocurve and point evaluation remain native Rust calls on the rounded display patch. Normalized witness UV is mapped to that patch's actual knot domain. The marker is a display location of the existing native witness, not a fresh geometry certificate. No CAD document, surface or body is committed by this panel.

Generation checks discard cancelled results. Changes to active state, source surface control points/knots/weights or numeric parameters cancel pending work and clear old display. Closing/unmounting clears the overlay. The shared-client test injects a captured late reply after cancellation and verifies only the replacement request can settle.

## Evidence

Six focused tests pass: two panel/client lifecycle tests and four preview transport tests. Panel tests use a real previously qualified WASM receipt with mocked display sampling and a controlled worker; they do not prove geometry. Full `vue-tsc --noEmit --pretty false` passes. The preceding source-graph-g2-preview receipt separately qualifies real WASM and a real worker realm.

In the running application at http://127.0.0.1:5195/, selecting the existing surfaces Площадка and Торец makes the panel visible. Mouse Preview and Enter both reach the native unsupported-graph refusal; the UI shows a Russian next-step message and Retry. Escape clears it without changing the four-object document. The screenshot records this rejection path. A positive supported-pair preview has not yet been verified in the browser; actual edge selection and closed-body application remain open.
