# Curved filled-volume qualification

Explicit native probes contradict complete curved-volume support:

- Sphere radius 2: exact=false, joins=true, trim=true, faces=false, pairs=false, nesting=None, orientations=[None].
- Cylinder radius 2, height 4: exact=true, joins=true, trim=true, faces=false, pairs=false, nesting=None, orientations=[None].

The required primitive volume-validity gate remains deliberately ignored in ordinary tests and fails when run explicitly with --ignored. No volume distance or absence-of-self-intersection claim is admitted by these probes.

A new native certify_linear_projection function proves a common global contraction after a fixed linear projection. It covers every knot rectangle using 4×4 sections, combines their projected Jacobian bounds into one hull and requires the same inverse contraction over the full chart. It returns None on incomplete work, periodic charts or failed proof. Section subdivision tightens enclosures; independent local certificates are not glued into a global claim. A diagonal XY projection with Z certifies a rational quarter-cylinder that coordinate-plane projection could not certify. All four authored cylinder sides pass. Collapsed/folded charts and partial coverage remain rejected.

Seven surface-injectivity tests and the authored-cylinder-side regression passed. Integration into face/self-intersection reports, curved face-pair classification, sphere boundary/injectivity evidence, volume orientation and final distance qualification remain open. No UI/WASM support expansion is claimed by this native stage.
