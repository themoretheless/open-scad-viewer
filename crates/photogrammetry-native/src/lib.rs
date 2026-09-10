//! Native shell: exports the photogrammetry host ABI as a dynamic/static
//! library (dylib/staticlib) for non-browser hosts. All logic lives in
//! `photogrammetry-ffi`; this crate only adds the `extern "C"` surface.
//! Native callers fetch responses through photo_response_ptr/len, because the
//! packed u64 return truncates the pointer to 32 bits (only wasm32 linear
//! memory can promise that).

#[no_mangle]
pub extern "C" fn photo_alloc(len: usize) -> usize {
    photogrammetry_ffi::photo_alloc(len)
}

/// # Safety
/// ptr/len must be a live allocation returned by this module; consumed once.
#[no_mangle]
pub unsafe extern "C" fn photo_free(ptr: usize, len: usize) {
    photogrammetry_ffi::photo_free(ptr, len)
}

#[no_mangle]
pub extern "C" fn photo_response_ptr() -> usize {
    photogrammetry_ffi::photo_response_ptr()
}

#[no_mangle]
pub extern "C" fn photo_response_len() -> usize {
    photogrammetry_ffi::photo_response_len()
}

/// # Safety
/// ptr/len must reference a live caller-owned allocation returned by photo_alloc.
/// The buffer is consumed on every path, so the caller must not photo_free it.
#[no_mangle]
pub unsafe extern "C" fn photo_add(
    width: usize,
    height: usize,
    focal: f64,
    ptr: usize,
    len: usize,
) -> u64 {
    photogrammetry_ffi::photo_add(width, height, focal, ptr, len)
}

/// # Safety
/// Same buffer ownership rules as photo_add; the calibration buffer stays
/// caller-owned and is only read.
#[no_mangle]
pub unsafe extern "C" fn photo_add_calibrated(
    width: usize,
    height: usize,
    focal: f64,
    ptr: usize,
    len: usize,
    calibration_ptr: usize,
    calibration_len: usize,
) -> u64 {
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

/// Explicit bounded dense preset; photo_run(2, resolution) remains the legacy default.
#[no_mangle]
pub extern "C" fn photo_dense(resolution: usize, preset: u32) -> u64 {
    photogrammetry_ffi::photo_dense(resolution, preset)
}

/// Selects the compute backend for subsequent runs: 0 = CPU (default),
/// 1 = GPU (builds with the `gpu` feature; otherwise a recorded no-op
/// that keeps the CPU reference).
#[no_mangle]
pub extern "C" fn photo_set_acceleration(value: u32) -> u64 {
    photogrammetry_ffi::photo_set_acceleration(value)
}

/// Stage 1 of the WebGPU dense sweep; the response value carries the payload
/// pointer/length and the WGSL shader text, or null when the request is
/// ineligible (caller then uses photo_dense).
#[no_mangle]
pub extern "C" fn photo_dense_prepare(resolution: usize, preset: u32) -> u64 {
    photogrammetry_ffi::photo_dense_prepare(resolution, preset)
}

/// Stage 2: consumes the score buffer like photo_add consumes rgb.
/// # Safety
/// ptr/len must reference a live caller-owned photo_alloc buffer, which this
/// call takes over and frees on any outcome.
#[no_mangle]
pub unsafe extern "C" fn photo_dense_finish(ptr: usize, len: usize) -> u64 {
    photogrammetry_ffi::photo_dense_finish(ptr, len)
}

/// Runs in a disposable host context, so cancellation releases the whole session.
#[no_mangle]
pub extern "C" fn photo_run(action: u32, resolution: usize) -> u64 {
    photogrammetry_ffi::photo_run(action, resolution)
}
