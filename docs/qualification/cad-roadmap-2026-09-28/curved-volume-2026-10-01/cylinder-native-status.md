# Native cylinder volume and distance stage

Curved cap trims now obtain a shared-edge certificate after exact curve/pcurve/surface agreement with a bounded 32768-work predicate budget. Straight shared edges obtain an opposite-side certificate when original control coordinates lie strictly on opposite sides of an authored coordinate plane. Both surfaces must have that complete natural boundary equal to the edge. No fitted plane or proximity test authorizes the result.

Eight shared-boundary regressions pass, including all eight cylinder side/cap pairs, all four adjacent side pairs, a 1e-12 perturbation of the cap pcurve and a same-side counterexample. The original cylinder now passes complete native boundary embedding and outward orientation. Four volume-validity tests and six solid-distance tests pass. The sphere requirement remains explicitly unachieved in an ignored gate; running it with --ignored fails.

Two authored cylinders of radius 2 and height 4, separated by translation Z=7, have independently known filled-volume gap 3 mm. Native distance converges within 1e-5 mm, excludes overlap, and returns points that evaluate on the original faces at their reported UVs. The complete dispatcher fixture reports allFacesInjective=true, allPairsClassified=true and absenceProven=true.

The radial case is now independently qualified in native code: translating the second cylinder X=7 gives the known gap 3 mm. The interval contains 3 mm with width at most 1e-5 mm, excludes material overlap, and both reported witness UVs evaluate to their original surface points. All seven solid-distance tests pass (7 passed, 0 failed).

This stage is native. Its cap/straight-edge improvements still require WASM packaging and browser qualification. Arbitrarily oriented/modified cylinders, spheres and general curved solids remain open.
