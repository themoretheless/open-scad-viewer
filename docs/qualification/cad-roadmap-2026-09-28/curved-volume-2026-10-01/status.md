# Curved filled-volume qualification

Explicit native probes contradict complete curved-volume support:

- Sphere radius 2: exact=false, joins=true, trim=true, faces=false, pairs=false, nesting=None, orientations=[None].
- Cylinder radius 2, height 4: exact=true, joins=true, trim=true, faces=false, pairs=false, nesting=None, orientations=[None].

The required primitive volume-validity gate remains deliberately ignored in ordinary tests and fails when run explicitly with --ignored. No volume distance or absence-of-self-intersection claim is admitted by these probes.

A new native certify_linear_projection function proves a common global contraction after a fixed linear projection. It covers every knot rectangle using 4×4 sections, combines their projected Jacobian bounds into one hull and requires the same inverse contraction over the full chart. It returns None on incomplete work, periodic charts or failed proof. Section subdivision tightens enclosures; independent local certificates are not glued into a global claim. A diagonal XY projection with Z certifies a rational quarter-cylinder that coordinate-plane projection could not certify. All four authored cylinder sides pass. Collapsed/folded charts and partial coverage remain rejected.

Seven surface-injectivity tests and the authored-cylinder-side regression passed. Integration into face/self-intersection reports, curved face-pair classification, sphere boundary/injectivity evidence, volume orientation and final distance qualification remain open. No UI/WASM support expansion is claimed by this native stage.

## Native integration and protocol

The ordinary face-injectivity path now falls back to six fixed linear bases and accounts for every 4×4 rectangle subdivision in the shared work budget. Each successful report distinguishes linearProjection from coordinate projection and uses global-linear-projection-contraction. The actual cylinder dispatcher fixture linear-face-native.json reports allFacesInjective=true, absenceProven=false and 102 work units. The host checks the supported basis, candidate ordering, exact full-coverage work count and contraction bound; seven host protocol tests pass. Three B-rep face tests and the dispatcher regression pass. A collapsed face requires enough budget to visit later faces; exhausted entries remain null.

The explicit curved-volume gate still fails: cylinder exact=true, joins=true, trim=true, faces=true, pairs=false. Pair-contact classification remains open. WASM rebuilding is in progress; the packaged browser binary has not yet been qualified for this fallback. Legacy coordinate reports remain accepted.
