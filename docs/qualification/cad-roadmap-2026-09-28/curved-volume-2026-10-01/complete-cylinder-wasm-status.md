# Complete cylinder WASM and browser qualification

The geometry WASM now includes the native exact cap and opposite-side boundary certificates. Packaged size: 9,541,905 bytes. SHA256: `e895860b62ee9002144ea46296599dfe6973ce0c4ca1b9064bd69000296d04ae`.

All 19 tests in `solidDistance.test.ts`, `solidSelfIntersection.test.ts` and `solidVolumeDistanceUi.test.ts` pass, including the actual WASM complete cylinder face/pair proof and filled-volume separation. Vue type checking, Vite production build and distribution verification pass (137 artifacts).

The Chromium 151.0.7922.34 keyboard qualification passes six control models. It uses 2930 Tab actions, preserves exported documents, localizes invalid target selection and terminates the held worker on Esc. The cylinder case returns a 3 mm axial gap interval and original-face witnesses. Its displayed meshes come from native NURBS tessellation with 16 divisions; the reviewed screenshot shows both cylinders, two witness markers and their connecting segment. Results and screenshot are archived alongside this file.

The initial browser attempt overlapped a production rebuild and timed out finding the command. The successful retry ran after the build finished. Radial cylinder distance has native proof only; arbitrarily transformed cylinders, spheres and general curved volume distances are not qualified by this stage.
