# Original-face separation witnesses: native stage

The filled-volume native API now includes separationWitness with original face indices, UV parameters, evaluated points and coordinate enclosures. It selects the admitted pair that supplies the published upper bound after material separation is proven. Shell-local face numbering is mapped through sorted original face indices, matching isolated_outward_shell. Containment and boundary contact return no separation witness.

Five solid-distance native tests and two geometry-bridge tests passed. The new regression evaluates returned UVs on the original authored surfaces, checks a 3 mm analytic gap, reverses shell face order, and verifies no separation witness for containment. native-witness-fixtures.json records actual dispatcher output.

WASM packaging, host protocol validation, UI visualization and broader curved/cavity witness qualification remain to be completed. This does not close the full distance requirement.

The cavity regression also verifies both input orders: the nearest witness belongs to the inner shell of the hollow body, evaluates on its original face, and has an independently known 1 mm gap. This passed in native Rust; curved cavity cases remain open.
