/* photogrammetry-kernel native ABI (libphotogrammetry_wasm.dylib/.so/.dll).
 *
 * The same import-free ABI serves the browser WASM build and native hosts.
 * Responses are MGV1 binary envelopes ({ok, value} / {ok:false, message})
 * returned as packed (length << 32) | pointer into a heap allocation that the
 * caller releases with photo_free.
 */
#ifndef PHOTOGRAMMETRY_H
#define PHOTOGRAMMETRY_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Allocates `len` bytes; 0 on rejection (len == 0 or over the transport
 * limit). The host fills the buffer before passing it to photo_add*. */
uintptr_t photo_alloc(size_t len);

/* Frees a buffer returned by photo_alloc or a response pointer from
 * photo_run/photo_dense/... (pass the packed pointer and length). */
void photo_free(uintptr_t ptr, size_t len);

/* Adds an interleaved RGB photo (width*height*3 bytes at ptr). The buffer is
 * consumed on every outcome; never free it after a call. */
uintptr_t photo_add(size_t width, size_t height, double focal, uintptr_t ptr, size_t len);
uintptr_t photo_add_calibrated(size_t width, size_t height, double focal, uintptr_t ptr,
    size_t len, uintptr_t calibration_ptr, size_t calibration_len);

/* Backend selection for subsequent runs: 0 = CPU (default), 1 = GPU (opt-in,
 * qualified separately; CPU fallback without an adapter or the feature). */
uintptr_t photo_set_acceleration(uintptr_t value);

/* Native hosts read responses through these two (the packed u64 return value
 * truncates the pointer to 32 bits, which only wasm32 linear memory allows). */
uintptr_t photo_response_ptr(void);
uintptr_t photo_response_len(void);

/* Runs a stage: 0 = clear session, 1 = sparse reconstruction,
 * 2 = dense surface, 3 = compact surface, 4 = report. */
uintptr_t photo_run(uintptr_t action, size_t resolution);

/* Explicit dense presets: 0 = baseline, 1 = slanted plane, 2 = dual-scale volume. */
uintptr_t photo_dense(size_t resolution, uintptr_t preset);

/* Browser WebGPU sweep split (also usable by native hosts driving their own
 * GPU): prepare returns the payload pointer/length plus the WGSL text in its
 * envelope; finish consumes the score buffer and returns the surface. */
uintptr_t photo_dense_prepare(size_t resolution, uintptr_t preset);
uintptr_t photo_dense_finish(uintptr_t ptr, size_t len);

#ifdef __cplusplus
}
#endif

#endif
