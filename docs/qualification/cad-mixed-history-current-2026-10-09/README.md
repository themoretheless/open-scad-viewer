# Current mixed CAD history qualification

Replayed three service/WASM chains on current code: bracket (Boolean union, planar cap edits, translations, exact longitudinal fillet), flange (hole Boolean, cap edits, translations, circular fillet), enclosure (hole Boolean, cap edits, translations, layered fillet). Each chain performs 20 distinct edits, verifies analytical dimensions/volume and topology contracts, traverses all Undo/Redo states, and restores the serialized final document. Three tests pass in 9.09 seconds.

Exported each actual final body, imported its STEP and re-exported: six files pass independent OpenCascade validity, dimensions and volume checks. Kernel SHA256 is recorded in qualification.json.

Limits: this run uses the service/WASM path and synchronous history; it does not replace browser replay of mixed operations, nor prove general NURBS fillets, arbitrary corners, global thickness/tangency or all self-intersections. UI replay inputs are staged in /tmp/cad-mixed-ui-current-20261009.
