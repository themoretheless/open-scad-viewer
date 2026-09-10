//! Browser shell: exports the geometry linear-memory ABI as an import-free
//! wasm32 module. All logic lives in `geometry-bridge`; this crate only adds
//! the `extern "C"` surface the TypeScript host (`services/geometry`) uses.
//! Responses are returned as packed u64 (len << 32 | ptr), which is sound
//! because wasm32 linear memory pointers fit 32 bits.

#[no_mangle]
pub extern "C" fn abi_alloc(len: usize) -> usize {
    geometry_bridge::abi::abi_alloc(len)
}

/// # Safety
/// Pointers must reference live buffers allocated by this module, with their
/// exact lengths. Mesh pointers must come from operation 9; freeing consumes
/// them exactly once.
#[no_mangle]
pub unsafe extern "C" fn abi_free(ptr: usize, len: usize) {
    geometry_bridge::abi::abi_free(ptr, len)
}

/// # Safety
/// Pointers must reference live buffers allocated by this module, with their
/// exact lengths.
#[no_mangle]
pub unsafe extern "C" fn abi_request(op: u32, ptr: usize, len: usize) -> u64 {
    geometry_bridge::abi::abi_request(op, ptr, len)
}

/// # Safety
/// ptr must reference a live mesh handle returned by operation 9.
#[no_mangle]
pub unsafe extern "C" fn abi_mesh_field(ptr: usize, field: u32) -> usize {
    geometry_bridge::abi::abi_mesh_field(ptr, field)
}

/// # Safety
/// ptr must reference a live mesh handle returned by operation 9; consumed once.
#[no_mangle]
pub unsafe extern "C" fn abi_mesh_free(ptr: usize) {
    geometry_bridge::abi::abi_mesh_free(ptr)
}

/// # Safety
/// vp/vl and ip/il must reference live caller-owned buffers allocated by this
/// module; they are only read.
#[no_mangle]
pub unsafe extern "C" fn abi_import_mesh(
    stride: usize,
    vp: usize,
    vl: usize,
    ip: usize,
    il: usize,
) -> u64 {
    geometry_bridge::abi::abi_import_mesh(stride, vp, vl, ip, il)
}
