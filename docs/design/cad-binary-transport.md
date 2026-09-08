# Binary CAD mesh transport

CAD mesh export uses a Rust-owned snapshot exposed through WASM pointers and
lengths: positions are `f64`, triangle indices and face IDs are `u32`. Keeping
positions in double precision preserves the previous JSON path's geometry.
`withCadMesh` reads this snapshot through typed array views, then frees it in a
`finally` block. The memory buffer is reacquired after allocation because WASM
memory growth can detach old views.

The reader must be synchronous and must not retain or mutate the views. Calls
back into the bridge are rejected during the borrow. Rendering buffers are
copied into JavaScript-owned storage before returning, so they remain valid
after Rust cleanup and can be transferred to workers. This removes JSON and
intermediate JavaScript number arrays; it does not eliminate every copy.

Import accepts packed `Float32Array` vertices and `Uint32Array` indices.
wasm-bindgen copies the inputs into WASM; Rust extracts positions and validates
the mesh. Subsequent changes to JavaScript inputs do not affect the solid.

Small CAD commands, compiler documents and error messages still use JSON.
The JSON mesh exporter remains available as a test and benchmark reference.
This change does not remove serde, wasm-bindgen or compression dependencies.

## Verification and measurement

`tests/cadBinaryTransport.test.ts` checks exact JSON/binary equivalence, owned
renderer buffers, cleanup on exceptions, reentrancy, memory growth and import
ownership. Existing geometry oracle tests check that meshes remain unchanged.

Run `node --import tsx scripts/bench-cad-transport.mts` after building WASM.
It verifies equal outputs, alternates both paths, discards five warmup rounds
and reports 30 measured rounds. On Node 22.23.2, arm64, a 16,128-triangle mesh
took a median 10.99 ms through JSON and 4.14 ms through binary (2.65x).
This measures snapshot creation and transport, not geometry construction.
The SKADIS box build remained approximately 225 ms end to end; its geometry
operations dominate. These are local measurements, not a browser benchmark.
