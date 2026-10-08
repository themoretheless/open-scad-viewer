# Dimension input beside the manipulator

The earlier SVG foreignObject rendered a thick error border and clipped text.
The updated browser regression fails on that distribution. The editor now lives
in a normal HTML overlay, positioned from the original projected anchor and SVG
view center. A ResizeObserver follows the canvas and field dimensions; the field
is clamped inside the viewport and retains normal 13px text.

Both mouse and keyboard corner scenarios pass. Original-region error highlights
remain; the value and unit fit inside the canvas. Editing 1 mm in the overlay
updates the command panel and applies the corner. Undo, Redo and reload preserve
the exact document. The screenshots were reviewed. This qualifies the retained
rotated cuboid corner, not every command or arbitrary fillet geometry.

The receipt binds source hashes, the prior kernel and individual reports. The
factored-normal kernel was still optimizing; these runs do not qualify it.

Extended mouse and keyboard checks preserve 13px input text across camera zoom
and a 1100×800 viewport resize. Invalid text reports a local numeric error and
blocks apply without changing the document; valid unit input recovers and applies.
