# Exact sphere distance native contract stage

The canonical filled-volume distance fixture now includes a seventh case: two radius-3 spheres separated by translation X=8. Both native validity summaries are proven; the reported material gap interval is `[1.9999999999999991, 2.0000093171908517]` mm, contains the independently known 2 mm gap and is narrower than the requested 1e-5 mm tolerance. Original-face witnesses and native NURBS display meshes (16 divisions) are included.

The native dispatcher/export regression passes and asserts proven volume validity for both inputs. TypeScript contract validation accepts the spherical result and rejects it if either body's self-intersection-absence evidence is false. The focused native/protocol/UI selection passes 16 tests; seven tests are skipped by its name filter, including the new actual-WASM path pending packaging. Vue type checking passes.

The existing seven-case browser driver already reads each fixture's explicit expected gap and original display mesh. Its sphere scenario and full 23-test WASM suite remain to be run on the live exact-sphere package once it finishes. This native contract evidence does not qualify the currently packaged older WASM or close the production distribution-size gate.
