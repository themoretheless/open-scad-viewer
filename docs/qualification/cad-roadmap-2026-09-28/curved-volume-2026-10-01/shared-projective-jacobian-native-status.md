# Shared projection derivative calculation

Coordinate and perspective injectivity proofs now use the same outward-interval homogeneous derivative routine. Perspective numerators occupy its first two coordinates and the transformed positive denominator occupies the homogeneous weight coordinate. The perspective caller still refuses a denominator enclosure containing zero before asking for quotient derivatives. The third projected coordinate is identically zero.

This removes a duplicate implementation of Bernstein derivative hulls and rational quotient bounds. All 220 NURBS unit tests and all 105 sphere-related B-rep tests pass after the change. The WASM package is currently being rebuilt; size reduction is not yet measured or claimed.

The preceding perspective WASM package (`f35b9f0276f32cc1058b5c02d8493a7e3b809e5c664063f48489fea81aa3e061`, 9,552,216 bytes) passed 22 actual WASM/protocol/UI tests and mouse/keyboard browser scenarios. Keyboard traversal used 2779 Tabs; page errors were empty and JSON exports remained unchanged. A full sphere face budget reports eight proven faces and 968 sections but cannot prove the distinct-face aggregate; budget 967 explicitly leaves face 8 unproved. These runs do not qualify this new derivative refactor or the later sphere-pole certificate in WASM.

Production Vite build and type checking passed for that preceding package, but distribution verification failed: 7,152,356 asset bytes exceeded the unchanged 7,152,000 budget by 356 bytes. Packaging the shared derivative routine and rechecking the distribution budget remains a required gate. Browser evidence is archived separately and must not be interpreted as a passing distribution-size gate.
