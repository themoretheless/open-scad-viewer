# Supported source graph preview in the browser

Loaded the included two-surface fixture using the normal project import UI on a separate unique localhost origin. The working document at 127.0.0.1 was not replaced. Selected both surfaces and ran the new panel through the actual packaged WASM and preview worker.

The viewport renders ten isocurves and one native radius-witness location. The full-patch radius check refuses both tested target radii (1 and 2 mm); this is successful preview rendering with an explicit geometry refusal, not successful fillet admission. The scene stays at two source objects; the display-only panel has no document commit path.

Observed Esc removing the overlay, Enter rebuilding it, a committed radius edit removing the old overlay, and Preview rebuilding with the new input. A positive browser run uncovered a real marker bug: fixed SVG radius 6 was in model coordinates, filling most of the viewport on a 10 mm part. The marker now follows the existing view-scaled diagnostic-marker convention and uses a non-scaling stroke. Its measured diameter is about 11.29 pixels both before and after zoom. The screenshot shows the corrected result, not the oversized marker.

Six focused lifecycle/transport tests and full vue-tsc pass. Browser evidence is distinct from component tests with mocked display sampling. The project-download observation timed out; no downloaded-byte comparison or export qualification is claimed. Closed-body transfer/application, general NURBS cases, full radius admission and parts/STEP/history acceptance remain open.
