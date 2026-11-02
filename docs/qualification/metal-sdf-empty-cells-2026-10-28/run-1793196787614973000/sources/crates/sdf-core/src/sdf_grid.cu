// CUDA port of `SDF_WGSL` (crates/sdf-core/src/lib.rs). Both kernels must
// stay semantically identical: one thread per grid node evaluates the flat
// postorder field tree (node kinds in sdf-core/src/flat.rs) with a value
// stack. f32 arithmetic, like the WGSL path.
//
// Regenerate the checked-in PTX with `npm run build:cuda-kernels` (nvcc).

#include <cuda_runtime.h>

#define STACK_DEPTH 33
#define F32_MAX 3.402823466e+38f

__device__ __forceinline__ float3 v3(float x, float y, float z) {
    return make_float3(x, y, z);
}
__device__ __forceinline__ float3 sub3(float3 a, float3 b) {
    return make_float3(a.x - b.x, a.y - b.y, a.z - b.z);
}
__device__ __forceinline__ float3 add3(float3 a, float3 b) {
    return make_float3(a.x + b.x, a.y + b.y, a.z + b.z);
}
__device__ __forceinline__ float3 scale3(float3 a, float s) {
    return make_float3(a.x * s, a.y * s, a.z * s);
}
__device__ __forceinline__ float dot3(float3 a, float3 b) {
    return a.x * b.x + a.y * b.y + a.z * b.z;
}
__device__ __forceinline__ float3 cross3(float3 a, float3 b) {
    return make_float3(a.y * b.z - a.z * b.y, a.z * b.x - a.x * b.z, a.x * b.y - a.y * b.x);
}
__device__ __forceinline__ float length3(float3 a) {
    return sqrtf(dot3(a, a));
}
__device__ __forceinline__ float distance3(float3 a, float3 b) {
    return length3(sub3(a, b));
}
__device__ __forceinline__ float clampf(float v, float lo, float hi) {
    return fminf(fmaxf(v, lo), hi);
}

__device__ float3 closest_triangle(float3 p, float3 a, float3 b, float3 c) {
    float3 ab = sub3(b, a);
    float3 ac = sub3(c, a);
    float3 n = cross3(ab, ac);
    float nn = dot3(n, n);
    if (nn > 0.0f) {
        float3 q = sub3(p, scale3(n, dot3(sub3(p, a), n) / nn));
        float3 aq = sub3(q, a);
        float v = dot3(cross3(aq, ac), n) / nn;
        float w = dot3(cross3(ab, aq), n) / nn;
        if (v >= 0.0f && w >= 0.0f && v + w <= 1.0f) {
            return q;
        }
    }
    float3 best = a;
    float best_dist = F32_MAX;
    float3 edges[6] = {a, b, b, c, c, a};
    for (unsigned e = 0; e < 3; e++) {
        float3 ea = edges[e * 2];
        float3 eb = edges[e * 2 + 1];
        float3 d = sub3(eb, ea);
        float t = 0.0f;
        float dd = dot3(d, d);
        if (dd > 0.0f) {
            t = clampf(dot3(sub3(p, ea), d) / dd, 0.0f, 1.0f);
        }
        float3 q = add3(ea, scale3(d, t));
        float dist_edge = distance3(p, q);
        if (dist_edge < best_dist) {
            best = q;
            best_dist = dist_edge;
        }
    }
    return best;
}

extern "C" __global__ void sdf_grid(
    unsigned nx,
    unsigned ny,
    unsigned nz,
    unsigned n_nodes,
    float min_x,
    float min_y,
    float min_z,
    float step_x,
    float step_y,
    float step_z,
    const unsigned* __restrict__ kinds,
    const float* __restrict__ node_params,
    const unsigned* __restrict__ aux,
    const float* __restrict__ tris,
    float* __restrict__ values
) {
    unsigned row = nx + 1u;
    unsigned total = row * (ny + 1u) * (nz + 1u);
    unsigned id = blockIdx.x * blockDim.x + threadIdx.x;
    if (id >= total) {
        return;
    }
    unsigned x = id % row;
    unsigned y = (id / row) % (ny + 1u);
    unsigned z = id / (row * (ny + 1u));
    float3 p = v3(
        min_x + (float)x * step_x,
        min_y + (float)y * step_y,
        min_z + (float)z * step_z
    );
    float stack[STACK_DEPTH];
    unsigned sp = 0u;
    for (unsigned n = 0u; n < n_nodes; n++) {
        unsigned base = n * 8u;
        unsigned kind = kinds[n];
        if (kind <= 2u) {
            // Leaves: sphere / box / torus with translate-folded parameters.
            float v = 0.0f;
            float3 c = v3(node_params[base], node_params[base + 1u], node_params[base + 2u]);
            if (kind == 0u) {
                v = length3(sub3(p, c)) - node_params[base + 3u];
            } else if (kind == 1u) {
                float3 h = v3(node_params[base + 3u], node_params[base + 4u], node_params[base + 5u]);
                float3 d = sub3(p, c);
                float3 q = v3(fabsf(d.x) - h.x, fabsf(d.y) - h.y, fabsf(d.z) - h.z);
                float3 qp = v3(fmaxf(q.x, 0.0f), fmaxf(q.y, 0.0f), fmaxf(q.z, 0.0f));
                v = length3(qp) + fminf(fmaxf(q.x, fmaxf(q.y, q.z)), 0.0f);
            } else {
                float3 q = sub3(p, c);
                float ring = sqrtf(q.x * q.x + q.y * q.y) - node_params[base + 3u];
                v = sqrtf(ring * ring + q.z * q.z) - node_params[base + 4u];
            }
            stack[sp] = v;
            sp++;
        } else if (kind == 8u || kind == 9u) {
            // Mesh distance: brute-force closest point (and solid angle for
            // signed) over the referenced triangle window, in scan order.
            unsigned start = aux[n * 2u];
            unsigned count = aux[n * 2u + 1u];
            float best = F32_MAX;
            float angle = 0.0f;
            for (unsigned t = 0u; t < count; t++) {
                unsigned base_t = (start + t) * 9u;
                float3 a = v3(tris[base_t], tris[base_t + 1u], tris[base_t + 2u]);
                float3 b = v3(tris[base_t + 3u], tris[base_t + 4u], tris[base_t + 5u]);
                float3 c = v3(tris[base_t + 6u], tris[base_t + 7u], tris[base_t + 8u]);
                best = fminf(best, distance3(p, closest_triangle(p, a, b, c)));
                if (kind == 9u) {
                    float3 qa = sub3(a, p);
                    float3 qb = sub3(b, p);
                    float3 qc = sub3(c, p);
                    float la = length3(qa);
                    float lb = length3(qb);
                    float lc = length3(qc);
                    angle += 2.0f * atan2f(
                        dot3(qa, cross3(qb, qc)),
                        la * lb * lc + dot3(qa, qb) * lc + dot3(qb, qc) * la + dot3(qc, qa) * lb
                    );
                }
            }
            float v = best;
            if (kind == 9u && best > 0.0f && fabsf(angle) > 6.283185307f) {
                v = -best;
            }
            stack[sp] = v;
            sp++;
        } else if (kind == 7u) {
            sp--;
            stack[sp] = stack[sp] - node_params[base];
            sp++;
        } else {
            sp--;
            float b = stack[sp];
            sp--;
            float a = stack[sp];
            float v = 0.0f;
            if (kind == 3u) {
                v = fminf(a, b);
            } else if (kind == 4u) {
                v = fmaxf(a, b);
            } else if (kind == 5u) {
                v = fmaxf(a, -b);
            } else {
                float k = node_params[base];
                float h = clampf(0.5f + 0.5f * (b - a) / k, 0.0f, 1.0f);
                v = b * (1.0f - h) + a * h - k * h * (1.0f - h);
            }
            stack[sp] = v;
            sp++;
        }
    }
    values[id] = stack[0];
}
