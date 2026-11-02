# Source seam curvature compatibility

Added `Patch::inspect_source_curvature_seams`, inspecting the entire U interval at each exact V=0 and V=1 source contact. The area inspection API still rejects degenerate rectangles; only this explicit seam API admits them. It uses the same source-bound reference, actual polynomial derivatives, outward interval operations and shared coefficient budget. Zero work remains unresolved.

On the constant graph control fixture, both source seams have inverse-curvature error lower bound greater than 0.99 mm^-1 against the approximately 1 mm^-1 input-derived law. Both are proven violations at tolerance 0.001 mm^-1. This is not a floating point sample near the seam: the bound covers the entire seam.

The G2 profile has zero transverse normal curvature at these contacts, matching the ruled source direction. For the planar fixture, imposing a positive constant transverse inverse radius all the way to the seam conflicts with G2 contact. This does not refute variable-curvature G2 blends. The next construction must explicitly prescribe curvature rolloff toward the source, separately from the nominal core radius; accepting the central radius band cannot certify those contact regions.

No shape, tolerance or production admission was changed to hide this conflict. General B-rep trimming, node construction and packaged application acceptance remain open.
