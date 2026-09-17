// Frontoparallel NCC depth sweep for photogrammetry: a line-by-line port of the
// `sweep_select` WGSL entry (`dense::SWEEP_WGSL`). One thread per depth-map
// pixel scores every inverse-depth hypothesis against every source view and
// selects the winning bin in registers, so only 16 bytes per pixel return to
// the host. f32 throughout, exactly as the shader; the f64 depth
// reconstruction and all downstream geometry stay on the CPU.

#include <cuda_runtime.h>

#define MAX_TAPS 25
#define MAX_HYP 128

// Gray levels live in [0, 1]; -1 marks an invalid (out-of-frame) sample,
// matching the CPU's Option-returning bilinear sampler.
static __device__ __forceinline__ float sample_gray(
    const float* __restrict__ grays,
    unsigned offset,
    unsigned w,
    unsigned h,
    float x,
    float y
) {
    if (!(x >= 0.0f) || !(y >= 0.0f) || x >= (float)w - 1.0f || y >= (float)h - 1.0f) {
        return -1.0f;
    }
    unsigned ix = (unsigned)x;
    unsigned iy = (unsigned)y;
    float a = x - (float)ix;
    float b = y - (float)iy;
    unsigned i = offset + iy * w + ix;
    float p00 = grays[i];
    float p10 = grays[i + 1u];
    float p01 = grays[i + w];
    float p11 = grays[i + w + 1u];
    return (1.0f - b) * ((1.0f - a) * p00 + a * p10) + b * ((1.0f - a) * p01 + a * p11);
}

// Centered reference patch and its energy; returns false for flat patches.
static __device__ __forceinline__ bool reference_patch(
    const float* __restrict__ grays,
    unsigned plen,
    unsigned radius,
    unsigned ref_goff,
    unsigned ref_w,
    unsigned ref_h,
    float step,
    float px,
    float py,
    float* centered,
    float* energy
) {
    unsigned ps = radius * 2u + 1u;
    float refv[MAX_TAPS];
    for (unsigned i = 0; i < plen; i++) {
        float ox = (float)(i % ps) - (float)radius;
        float oy = (float)(i / ps) - (float)radius;
        // Interior pixels keep the reference patch fully in frame (CPU invariant).
        refv[i] = sample_gray(grays, ref_goff, ref_w, ref_h, px + ox * step, py + oy * step);
    }
    float mean = 0.0f;
    for (unsigned i = 0; i < plen; i++) {
        mean += refv[i];
    }
    mean = mean / (float)plen;
    float e = 0.0f;
    for (unsigned i = 0; i < plen; i++) {
        centered[i] = refv[i] - mean;
        e += centered[i] * centered[i];
    }
    *energy = e;
    return e >= 0.0001f;
}

// Mean NCC over the sources supporting depth z at reference pixel (px, py);
// -1 when fewer than `needed` sources correlate above 0.4.
static __device__ __forceinline__ float score_hypothesis(
    const float* __restrict__ srcf,
    const unsigned* __restrict__ srcm,
    const float* __restrict__ grays,
    unsigned n_src,
    unsigned plen,
    unsigned radius,
    unsigned needed,
    float step,
    float ref_f,
    float ref_cx,
    float ref_cy,
    float px,
    float py,
    float z,
    const float* centered,
    float energy
) {
    unsigned ps = radius * 2u + 1u;
    float sum = 0.0f;
    unsigned count = 0u;
    for (unsigned s = 0; s < n_src; s++) {
        unsigned fb = s * 16u;
        unsigned mb = s * 4u;
        unsigned goff = srcm[mb];
        unsigned gw = srcm[mb + 1u];
        unsigned gh = srcm[mb + 2u];
        float focal = srcf[fb + 12u];
        float cx = srcf[fb + 13u];
        float cy = srcf[fb + 14u];
        float ssum = 0.0f;
        float sq = 0.0f;
        float cov = 0.0f;
        bool ok = true;
        for (unsigned i = 0; i < plen; i++) {
            float ox = ((float)(i % ps) - (float)radius) * step;
            float oy = ((float)(i / ps) - (float)radius) * step;
            float rx = (px - ref_cx + ox) / ref_f;
            float ry = (py - ref_cy + oy) / ref_f;
            float vx = srcf[fb] * rx + srcf[fb + 1u] * ry + srcf[fb + 2u];
            float vy = srcf[fb + 3u] * rx + srcf[fb + 4u] * ry + srcf[fb + 5u];
            float vz = srcf[fb + 6u] * rx + srcf[fb + 7u] * ry + srcf[fb + 8u];
            float wx = vx * z + srcf[fb + 9u];
            float wy = vy * z + srcf[fb + 10u];
            float wz = vz * z + srcf[fb + 11u];
            if (wz <= 0.0f) {
                ok = false;
                break;
            }
            float value = sample_gray(grays, goff, gw, gh, focal * wx / wz + cx, focal * wy / wz + cy);
            if (value < 0.0f) {
                ok = false;
                break;
            }
            ssum += value;
            sq += value * value;
            cov += centered[i] * value;
        }
        if (!ok) {
            continue;
        }
        float energy_s = sq - ssum * ssum / (float)plen;
        if (energy_s < 0.0001f) {
            continue;
        }
        float ncc = fminf(fmaxf(cov / sqrtf(energy * energy_s), -1.0f), 1.0f);
        if (ncc > 0.4f) {
            sum += ncc;
            count++;
        }
    }
    return count >= needed ? sum / (float)count : -1.0f;
}

// Last index wins ties, matching the CPU's Iterator::max_by on total order.
static __device__ __forceinline__ unsigned argmax(const float* sc, unsigned lo, unsigned hi) {
    unsigned best = lo;
    for (unsigned d = lo; d <= hi; d++) {
        if (sc[d] >= sc[best]) {
            best = d;
        }
    }
    return best;
}

// Scores stay in registers/local memory and the CPU's pick_depth logic runs here
// per pixel (range clamp, uniqueness margin, parabolic sub-bin offset).
// results = (valid, best bin, offset, score); depth is rebuilt in f64 on the CPU.
extern "C" __global__ void sweep_select(
    unsigned width,
    unsigned height,
    unsigned n_hyp,
    unsigned n_src,
    unsigned plen,
    unsigned radius,
    unsigned needed,
    unsigned ref_w,
    unsigned ref_h,
    unsigned ref_goff,
    float min_corr,
    float margin,
    float step,
    float ref_f,
    float ref_cx,
    float ref_cy,
    const float* __restrict__ hyps,
    const float* __restrict__ srcf,
    const unsigned* __restrict__ srcm,
    const float* __restrict__ grays,
    const unsigned* __restrict__ bins,
    float* __restrict__ results
) {
    unsigned x = blockIdx.x * blockDim.x + threadIdx.x;
    unsigned y = blockIdx.y * blockDim.y + threadIdx.y;
    if (x >= width || y >= height) {
        return;
    }
    unsigned pixel = y * width + x;
    results[pixel * 4u] = 0.0f;
    results[pixel * 4u + 1u] = 0.0f;
    results[pixel * 4u + 2u] = 0.0f;
    results[pixel * 4u + 3u] = 0.0f;
    float centered[MAX_TAPS];
    float energy = 0.0f;
    float px = (float)x * step;
    float py = (float)y * step;
    if (x < 3u || y < 3u || x + 3u >= width || y + 3u >= height
        || !reference_patch(grays, plen, radius, ref_goff, ref_w, ref_h, step, px, py, centered, &energy)) {
        return;
    }
    float sc[MAX_HYP];
    for (unsigned d = 0; d < n_hyp; d++) {
        sc[d] = score_hypothesis(
            srcf, srcm, grays, n_src, plen, radius, needed, step, ref_f, ref_cx, ref_cy,
            px, py, hyps[d], centered, energy);
    }
    unsigned last = n_hyp - 1u;
    unsigned bin_lo = bins[pixel * 2u];
    unsigned bin_hi = bins[pixel * 2u + 1u];
    bool ranged = bin_lo > 0u || bin_hi < last;
    unsigned best_ranged = argmax(sc, bin_lo, bin_hi);
    unsigned best = 0u;
    float alternative = -1.0f;
    if (ranged && sc[best_ranged] >= min_corr) {
        best = best_ranged;
        for (unsigned d = 0; d < n_hyp; d++) {
            if (d + 3u < bin_lo || d > bin_hi + 3u) {
                alternative = fmaxf(alternative, sc[d]);
            }
        }
    } else {
        best = argmax(sc, 0u, last);
        for (unsigned d = 0; d < n_hyp; d++) {
            if (max(d, best) - min(d, best) > 3u) {
                alternative = fmaxf(alternative, sc[d]);
            }
        }
    }
    if (sc[best] < min_corr || sc[best] - alternative < margin) {
        return;
    }
    float offset = 0.0f;
    if (best > 0u && best < last) {
        float a = sc[best - 1u];
        float b = sc[best];
        float c = sc[best + 1u];
        float denom = a - 2.0f * b + c;
        if (a > 0.0f && c > 0.0f && fabsf(denom) > 1e-8f) {
            offset = fminf(fmaxf(0.5f * (a - c) / denom, -0.5f), 0.5f);
        }
    }
    results[pixel * 4u] = 1.0f;
    results[pixel * 4u + 1u] = (float)best;
    results[pixel * 4u + 2u] = offset;
    results[pixel * 4u + 3u] = sc[best];
}
