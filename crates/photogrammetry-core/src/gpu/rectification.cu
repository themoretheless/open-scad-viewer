// Brown--Conrady output-pixel rectification. The host performs candidate
// search; this kernel only maps and bilinearly samples one accepted candidate.
#include <cuda_runtime.h>

extern "C" __global__ void rectify_brown(
    unsigned width, unsigned height, float focal, float fx, float fy, float cx, float cy,
    float k1, float k2, float k3, float p1, float p2,
    const unsigned* source, unsigned* output, unsigned* valid
) {
    unsigned x = blockIdx.x * blockDim.x + threadIdx.x;
    unsigned y = blockIdx.y * blockDim.y + threadIdx.y;
    if (x >= width || y >= height) return;
    float xn = (float(x) - float(width) * .5f) / focal;
    float yn = (float(y) - float(height) * .5f) / focal;
    float r2 = xn * xn + yn * yn;
    float radial = 1.f + r2 * (k1 + r2 * (k2 + r2 * k3));
    float derivative = k1 + r2 * (2.f * k2 + 3.f * r2 * k3);
    float cross = 2.f * xn * yn * derivative + 2.f * p1 * xn + 2.f * p2 * yn;
    float dx = radial + 2.f * xn * xn * derivative + 2.f * p1 * yn + 6.f * p2 * xn;
    float dy = radial + 2.f * yn * yn * derivative + 6.f * p1 * yn + 2.f * p2 * xn;
    float determinant = dx * dy - cross * cross;
    float u = fx * (xn * radial + 2.f * p1 * xn * yn + p2 * (r2 + 2.f * xn * xn)) + cx;
    float v = fy * (yn * radial + p1 * (r2 + 2.f * yn * yn) + 2.f * p2 * xn * yn) + cy;
    if (!isfinite(u) || !isfinite(v) || !isfinite(determinant) || dx <= 1e-6f || dy <= 1e-6f ||
        determinant <= 1e-6f || u < 0.f || v < 0.f || u > float(width - 1) || v > float(height - 1)) {
        atomicAnd(valid, 0u);
        return;
    }
    unsigned ix = unsigned(floorf(u)), iy = unsigned(floorf(v));
    unsigned nx = min(ix + 1u, width - 1u), ny = min(iy + 1u, height - 1u);
    float a = u - float(ix), b = v - float(iy);
    unsigned c[4] = {source[iy * width + ix], source[iy * width + nx], source[ny * width + ix], source[ny * width + nx]};
    unsigned packed = 0;
    for (unsigned shift = 0; shift < 24; shift += 8) {
        float c00 = float((c[0] >> shift) & 255u), c10 = float((c[1] >> shift) & 255u);
        float c01 = float((c[2] >> shift) & 255u), c11 = float((c[3] >> shift) & 255u);
        unsigned value = unsigned(fminf(255.f, fmaxf(0.f, roundf((1.f-b)*((1.f-a)*c00+a*c10)+b*((1.f-a)*c01+a*c11)))));
        packed |= value << shift;
    }
    output[y * width + x] = packed;
}
