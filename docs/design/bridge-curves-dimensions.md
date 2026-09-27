# Bridge curves and sketch dimensions

The Solid editor supports a G2 bridge between two selected native NURBS curves.
Shift-select them in the scene list, choose each endpoint and tension in the CV
panel, then create the bridge. The result is a separate editable degree-5 NURBS.
Its endpoint derivatives match the source derivatives after local parameter
scaling, including curvature. Source curves are retained. Changes to the sources
do not regenerate existing bridges. Surface bridging is not included.

Select a sketch and click Dimensions (or open Properties) to add linear or signed angular dimensions by
point number. Lengths use millimetres. Angles use degrees in [-180,180], with B
as the vertex in A-B-C. Editing length holds A and moves B along AB; editing
angle holds A/B and rotates C at its existing distance from B. Other dimensions
are remeasured rather than imposed as simultaneous constraints. Zero-length
segments cannot be resized or used to define an angle. Analytic arc/circle
samples can be measured, but their dimensions are read-only.

Annotations are saved in the Solid document and restored by undo/redo. Native
Rust computes geometry, measurements, label positions, and dimension lines.
The Vue panel only supplies references and values and renders the result.
Trimming or resampling a contour removes annotations because point identities
change; ordinary vertex edits and affine transformations retain them.

Example: open `examples/solid/bridge-dimensions.json` through the Solid File menu.
It includes an annotated polyline and two curves ready for G2 bridging.
