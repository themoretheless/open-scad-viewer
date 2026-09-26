//! Browser shell: exports the photogrammetry host ABI as an import-free
//! wasm32 module. All logic lives in `photogrammetry-ffi`; this crate only
//! adds the `extern "C"` surface the TypeScript host (`services/photogrammetry`)
//! instantiates. Responses are returned as packed u64 (len << 32 | ptr), which
//! is sound because wasm32 linear memory pointers fit 32 bits.
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
pub extern "C" fn photo_alloc(len: usize) -> usize {
    photogrammetry_ffi::photo_alloc(len)
}

/// # Safety
/// ptr/len must be a live allocation returned by this module; consumed once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn photo_free(ptr: usize, len: usize) {
    unsafe { photogrammetry_ffi::photo_free(ptr, len) }
}

/// # Safety
/// ptr/len must reference a live caller-owned allocation returned by photo_alloc.
/// The buffer is consumed on every path, so the caller must not photo_free it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn photo_add(
    width: usize,
    height: usize,
    focal: f64,
    ptr: usize,
    len: usize,
) -> u64 {
    unsafe { photogrammetry_ffi::photo_add(width, height, focal, ptr, len) }
}

/// # Safety
/// Same buffer ownership rules as photo_add; the calibration buffer stays
/// caller-owned and is only read.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn photo_add_calibrated(
    width: usize,
    height: usize,
    focal: f64,
    ptr: usize,
    len: usize,
    calibration_ptr: usize,
    calibration_len: usize,
) -> u64 {
    unsafe {
        photogrammetry_ffi::photo_add_calibrated(
            width,
            height,
            focal,
            ptr,
            len,
            calibration_ptr,
            calibration_len,
        )
    }
}

/// Explicit bounded dense preset; photo_run(2, resolution) remains the legacy default.
#[unsafe(no_mangle)]
pub extern "C" fn photo_dense(resolution: usize, preset: u32) -> u64 {
    photogrammetry_ffi::photo_dense(resolution, preset)
}

/// Stage 1 of the browser WebGPU dense sweep; the response value carries the
/// payload pointer/length and the WGSL shader text, or null when the request
/// is ineligible (caller then uses photo_dense).
#[unsafe(no_mangle)]
pub extern "C" fn photo_dense_prepare(resolution: usize, preset: u32) -> u64 {
    photogrammetry_ffi::photo_dense_prepare(resolution, preset)
}

/// Stage 2: consumes the score buffer like photo_add consumes rgb.
/// # Safety
/// ptr/len must reference a live caller-owned photo_alloc buffer, which this
/// call takes over and frees on any outcome.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn photo_dense_finish(ptr: usize, len: usize) -> u64 {
    unsafe { photogrammetry_ffi::photo_dense_finish(ptr, len) }
}

/// Stage 1 of the browser WebGPU sparse matching; the response value carries
/// the MAT1 payload pointer/length and the WGSL shader text, or null when the
/// request is ineligible (caller then uses the plain sparse action).
#[unsafe(no_mangle)]
pub extern "C" fn photo_sparse_prepare() -> u64 {
    photogrammetry_ffi::photo_sparse_prepare()
}

/// Stage 2: consumes the packed match response like photo_add consumes rgb.
/// # Safety
/// ptr/len must reference a live caller-owned photo_alloc buffer, which this
/// call takes over and frees on any outcome.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn photo_sparse_finish(ptr: usize, len: usize) -> u64 {
    unsafe { photogrammetry_ffi::photo_sparse_finish(ptr, len) }
}

/// Runs in a disposable Worker, so cancellation releases the whole session.
#[unsafe(no_mangle)]
pub extern "C" fn photo_run(action: u32, resolution: usize) -> u64 {
    photogrammetry_ffi::photo_run(action, resolution)
}
