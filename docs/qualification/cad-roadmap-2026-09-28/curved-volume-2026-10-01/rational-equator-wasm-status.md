# Rational sphere equator WASM stage

The package now includes the rational sphere equator and traversal-aware Boolean seam split from `e0fd1cea`. Size: 9,542,877 bytes. SHA256: `771c7eb6cdd646204940ab0952b825e7b3c1d06607850cc7e306ac2049b723bd`.

An actual WASM sphere constructor regression verifies all eight equator pcurves have weights `[1,1,2]` and that every original edge has the same weights in coedge traversal direction. Stored edge orientation is explicitly accounted for. The 21-test protocol/UI/distance suite and Vue type checking pass against this package; the suite includes native-fixture validation for the next perspective stage, not actual WASM perspective qualification.

The package does not include the new perspective face proof. That integration is being compiled separately. General sphere volume validity, arbitrary radii with rounded stereographic controls, and complete sphere face-pair classification remain unqualified.
