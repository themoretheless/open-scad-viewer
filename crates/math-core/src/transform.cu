// CUDA port of `TRANSFORM_WGSL` (crates/math-core/src/transform.wgsl). One
// thread per point computes `q = M*p + t`. f32 arithmetic, like the WGSL path.
//
// Regenerate the checked-in PTX with `npm run build:cuda-kernels` (nvcc).

#include <cuda_runtime.h>

extern "C" __global__ void transform_points(
    float m00, float m01, float m02,
    float m10, float m11, float m12,
    float m20, float m21, float m22,
    float tx, float ty, float tz,
    unsigned count,
    const float* __restrict__ points_in,
    float* __restrict__ points_out
) {
    unsigned i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i >= count) {
        return;
    }
    unsigned base = i * 3u;
    float px = points_in[base];
    float py = points_in[base + 1u];
    float pz = points_in[base + 2u];
    points_out[base] = m00 * px + m01 * py + m02 * pz + tx;
    points_out[base + 1u] = m10 * px + m11 * py + m12 * pz + ty;
    points_out[base + 2u] = m20 * px + m21 * py + m22 * pz + tz;
}
