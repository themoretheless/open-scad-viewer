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
The direct ABI copies the inputs into WASM; Rust extracts positions and validates
the mesh. Subsequent changes to JavaScript inputs do not affect the solid.

Commands, compiler inputs/results, surface evaluation and errors use the bounded
MGV1 binary value codec. No JSON stringify/parse occurs at the WASM boundary.
The native JSON entry points remain for document files and CLI consumers, using
our own parser/writer. The Cargo workspace has no external packages; serde,
serde_json, wasm-bindgen and indexmap have been removed, as has runtime fflate.
The build uses Node's built-in compressor and our checked DEFLATE decoder.

## Verification and measurement

`tests/cadBinaryTransport.test.ts` checks exact value/view equivalence, owned
renderer buffers, cleanup on exceptions, reentrancy, memory growth and import
ownership. Existing geometry oracle tests check that meshes remain unchanged.

Run `node --import tsx scripts/bench-cad-transport.mts` after building WASM.
It compares the binary value path with direct mesh views, checks equal outputs,
alternates both paths, discards five warmup rounds and reports 30 measured rounds.
On Node 22.23.2, arm64, a 16,128-triangle mesh took a median 7.86 ms through
binary values and 4.32 ms through direct views. This measures snapshot creation
and transport, not geometry construction. The current SKADIS build measured
approximately 250 ms end to end, versus about 225 ms before removing the
remaining dependencies. These are local measurements, not browser benchmarks.
Geometry operations dominate; dependency removal is not an overall speedup.
