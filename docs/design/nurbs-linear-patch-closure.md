# Automatic straight-boundary patch image closure

`brep_core::watertight::inspect_linear_patch_set` partitions complete original boundary images at all authored endpoints. It supports one-to-many partial overlap and nonlinear rational parameterizations of axis-aligned straight images. It does not substitute rounded trimmed curves for original geometry.

Each eligible curve is clamped, nonperiodic and positive-weight. Both endpoints differ on exactly one coordinate; every control lies on the same support line and inside the endpoint coordinate range. Positive rational convex-hull containment proves image inclusion, while continuity and the exact endpoint values prove surjectivity onto the entire segment by the intermediate value theorem. No derivative, inverse parameter approximation or sampled mesh is needed for this image-set proof.

All source faces must occur exactly once in one authored shell per model. Complete original edge/pcurve/surface agreement is independently verified with a shared budget. The physical coordinate ranges are partitioned by exact comparisons of original binary64 endpoints. Every positive-length interval must have exactly two opposite face uses from distinct models. Missing intervals, duplicate uses, wrong orientations, unsupported images and incomplete source checks prevent admission. A point contact does not constitute a shared interval.

The result certifies closure of the matched boundary images within twice the per-edge model tolerance. It does not author trim fractions or topology, prove a parameter bijection, or certify face/trim injectivity, vertex links, manifold embedding, outwardness or volume. General curved automatic partial matching and closed periodic seams remain unsupported. The existing full-boundary affine and nonlinear correspondence APIs retain their independent contracts.

Limits: 1..256 models with equal tolerances, one nonempty shell per model, at most 1024 free boundaries, at most 100000 positive image intervals, and 1..1000000 shared original-agreement cells. Resource overflow is an explicit error; insufficient source-agreement work returns an unqualified report. Input geometry is immutable.
