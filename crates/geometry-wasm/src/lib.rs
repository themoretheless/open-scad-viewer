//! Browser shell: exports the geometry linear-memory ABI as an import-free
//! wasm32 module. All logic lives in `geometry-bridge`; this crate only adds
//! the `extern "C"` surface the TypeScript host (`services/geometry`) uses.
//! Responses are returned as packed u64 (len << 32 | ptr), which is sound
//! because wasm32 linear memory pointers fit 32 bits.
#![feature(
    try_blocks,
    gen_blocks,
    yield_expr,
    super_let,
    deref_patterns,
    yeet_expr
)]
#![allow(unused_features)]

#[unsafe(no_mangle)]
pub extern "C" fn abi_alloc(len: usize) -> usize {
    geometry_bridge::abi::abi_alloc(len)
}

/// # Safety
/// Pointers must reference live buffers allocated by this module, with their
/// exact lengths. Mesh pointers must come from operation 9; freeing consumes
/// them exactly once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn abi_free(ptr: usize, len: usize) {
    unsafe { geometry_bridge::abi::abi_free(ptr, len) }
}

/// # Safety
/// Pointers must reference live buffers allocated by this module, with their
/// exact lengths.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn abi_request(op: u32, ptr: usize, len: usize) -> u64 {
    unsafe { geometry_bridge::abi::abi_request(op, ptr, len) }
}

/// # Safety
/// ptr must reference a live mesh handle returned by operation 9.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn abi_mesh_field(ptr: usize, field: u32) -> usize {
    unsafe { geometry_bridge::abi::abi_mesh_field(ptr, field) }
}

/// # Safety
/// ptr must reference a live mesh handle returned by operation 9; consumed once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn abi_mesh_free(ptr: usize) {
    unsafe { geometry_bridge::abi::abi_mesh_free(ptr) }
}

/// # Safety
/// vp/vl and ip/il must reference live caller-owned buffers allocated by this
/// module; they are only read.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn abi_import_mesh(
    stride: usize,
    vp: usize,
    vl: usize,
    ip: usize,
    il: usize,
) -> u64 {
    unsafe { geometry_bridge::abi::abi_import_mesh(stride, vp, vl, ip, il) }
}

/// # Safety
/// vp/vl and ip/il must reference live caller-owned buffers allocated by this
/// module; they are only read.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn abi_bvh_build(
    stride: usize,
    leaf: usize,
    vp: usize,
    vl: usize,
    ip: usize,
    il: usize,
) -> u64 {
    unsafe { geometry_bridge::abi::abi_bvh_build(stride, leaf, vp, vl, ip, il) }
}

/// # Safety
/// All buffer pointers must reference live caller-owned buffers allocated by
/// this module; they are only read.
#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn abi_semantic_edges(
    vp: usize,
    vl: usize,
    ip: usize,
    il: usize,
    mfp: usize,
    mfl: usize,
    mtp: usize,
    mtl: usize,
    weld: u32,
    crease_dot_threshold: f64,
) -> u64 {
    unsafe {
        geometry_bridge::abi::abi_semantic_edges(
            vp,
            vl,
            ip,
            il,
            mfp,
            mfl,
            mtp,
            mtl,
            weld,
            crease_dot_threshold,
        )
    }
}

/// # Safety
/// The handle must reference a live result from `abi_bvh_build` or
/// `abi_semantic_edges`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn abi_array_field(handle: usize, slot: u32) -> usize {
    unsafe { geometry_bridge::abi::abi_array_field(handle, slot) }
}

/// # Safety
/// The handle must reference a live analysis result; consumed once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn abi_array_free(handle: usize) {
    unsafe { geometry_bridge::abi::abi_array_free(handle) }
}
