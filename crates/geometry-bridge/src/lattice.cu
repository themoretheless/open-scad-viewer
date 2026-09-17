// CUDA port of LATTICE_WGSL. f32 arithmetic; extraction and NaN fallback stay
// on the CPU host path.

#include <cuda_runtime.h>

__device__ inline float3 add3(float3 a, float3 b) {
    return make_float3(a.x + b.x, a.y + b.y, a.z + b.z);
}

__device__ inline float3 sub3(float3 a, float3 b) {
    return make_float3(a.x - b.x, a.y - b.y, a.z - b.z);
}

__device__ inline float3 mul3(float3 a, float s) {
    return make_float3(a.x * s, a.y * s, a.z * s);
}

__device__ inline float dot3(float3 a, float3 b) {
    return a.x * b.x + a.y * b.y + a.z * b.z;
}

__device__ inline float3 cross3(float3 a, float3 b) {
    return make_float3(
        a.y * b.z - a.z * b.y,
        a.z * b.x - a.x * b.z,
        a.x * b.y - a.y * b.x
    );
}

__device__ inline float get3(float3 v, unsigned k) {
    return k == 0u ? v.x : (k == 1u ? v.y : v.z);
}

__device__ inline float clampf(float v, float lo, float hi) {
    return fminf(fmaxf(v, lo), hi);
}

__device__ inline float3 node_min(const float* nodes, unsigned n) {
    unsigned base = n * 10u;
    return make_float3(nodes[base], nodes[base + 1u], nodes[base + 2u]);
}

__device__ inline float3 node_max(const float* nodes, unsigned n) {
    unsigned base = n * 10u + 3u;
    return make_float3(nodes[base], nodes[base + 1u], nodes[base + 2u]);
}

__device__ inline unsigned node_left(const float* nodes, unsigned n) {
    return __float_as_uint(nodes[n * 10u + 6u]);
}

__device__ inline unsigned node_right(const float* nodes, unsigned n) {
    return __float_as_uint(nodes[n * 10u + 7u]);
}

__device__ inline unsigned node_start(const float* nodes, unsigned n) {
    return __float_as_uint(nodes[n * 10u + 8u]);
}

__device__ inline unsigned node_count(const float* nodes, unsigned n) {
    return __float_as_uint(nodes[n * 10u + 9u]);
}

__device__ inline float3 tri_point(const float* tris, unsigned t, unsigned k) {
    unsigned base = t * 9u + k * 3u;
    return make_float3(tris[base], tris[base + 1u], tris[base + 2u]);
}

__device__ float3 closest_triangle(float3 p, float3 a, float3 b, float3 c) {
    float3 ab = sub3(b, a);
    float3 ac = sub3(c, a);
    float3 n = cross3(ab, ac);
    float nn = dot3(n, n);
    if (nn > 0.0f) {
        float3 q = sub3(p, mul3(n, dot3(sub3(p, a), n) / nn));
        float3 aq = sub3(q, a);
        float v = dot3(cross3(aq, ac), n) / nn;
        float w = dot3(cross3(ab, aq), n) / nn;
        if (v >= 0.0f && w >= 0.0f && v + w <= 1.0f) {
            return q;
        }
    }
    float3 best = a;
    float best_dist = 3.402823466e38f;
    float3 starts[3] = {a, b, c};
    float3 ends[3] = {b, c, a};
    for (unsigned e = 0u; e < 3u; e++) {
        float3 ea = starts[e];
        float3 eb = ends[e];
        float3 d = sub3(eb, ea);
        float dd = dot3(d, d);
        float t = dd > 0.0f ? clampf(dot3(sub3(p, ea), d) / dd, 0.0f, 1.0f) : 0.0f;
        float3 q = add3(ea, mul3(d, t));
        float3 delta = sub3(p, q);
        float dist = sqrtf(dot3(delta, delta));
        if (dist < best_dist) {
            best = q;
            best_dist = dist;
        }
    }
    return best;
}

__device__ float nearest2(float3 p, const float* nodes, const float* tris) {
    float best = 3.402823466e38f;
    unsigned stack[64];
    unsigned sp = 0u;
    stack[sp++] = 0u;
    while (sp > 0u) {
        unsigned n = stack[--sp];
        float3 bmin = sub3(node_min(nodes, n), p);
        float3 bmax = sub3(p, node_max(nodes, n));
        float3 d2v = make_float3(
            fmaxf(fmaxf(bmin.x, bmax.x), 0.0f),
            fmaxf(fmaxf(bmin.y, bmax.y), 0.0f),
            fmaxf(fmaxf(bmin.z, bmax.z), 0.0f)
        );
        float bound = dot3(d2v, d2v);
        if (bound > best) {
            continue;
        }
        unsigned count = node_count(nodes, n);
        if (count > 0u) {
            for (unsigned i = 0u; i < count; i++) {
                unsigned t = node_start(nodes, n) + i;
                float3 q = closest_triangle(p, tri_point(tris, t, 0u), tri_point(tris, t, 1u), tri_point(tris, t, 2u));
                float3 dq = sub3(p, q);
                best = fminf(best, dot3(dq, dq));
            }
        } else {
            if (sp + 2u > 64u) {
                return -1.0f;
            }
            stack[sp] = node_left(nodes, n);
            stack[sp + 1u] = node_right(nodes, n);
            sp += 2u;
        }
    }
    return best;
}

__device__ uint2 ray_hits(float3 p, const float* nodes, const float* tris) {
    float3 dir = make_float3(1.0f, 0.3713906763541037f, 0.127831f);
    float hits[128];
    unsigned count = 0u;
    unsigned stack[64];
    unsigned sp = 0u;
    stack[sp++] = 0u;
    while (sp > 0u) {
        unsigned n = stack[--sp];
        float3 nmin = node_min(nodes, n);
        float3 nmax = node_max(nodes, n);
        float low = 0.0f;
        float high = 3.402823466e38f;
        for (unsigned k = 0u; k < 3u; k++) {
            float a = (get3(nmin, k) - get3(p, k)) / get3(dir, k);
            float b = (get3(nmax, k) - get3(p, k)) / get3(dir, k);
            low = fmaxf(low, fminf(a, b));
            high = fminf(high, fmaxf(a, b));
        }
        if (high < low) {
            continue;
        }
        unsigned tc = node_count(nodes, n);
        if (tc > 0u) {
            for (unsigned i = 0u; i < tc; i++) {
                unsigned t = node_start(nodes, n) + i;
                float3 a = tri_point(tris, t, 0u);
                float3 b = tri_point(tris, t, 1u);
                float3 c = tri_point(tris, t, 2u);
                float3 e1 = sub3(b, a);
                float3 e2 = sub3(c, a);
                float3 h = cross3(dir, e2);
                float det = dot3(e1, h);
                if (fabsf(det) < 1e-13f) {
                    continue;
                }
                float3 sv = sub3(p, a);
                float u = dot3(sv, h) / det;
                if (u < -1e-10f || u > 1.0f + 1e-10f) {
                    continue;
                }
                float3 q = cross3(sv, e1);
                float v = dot3(dir, q) / det;
                if (v < -1e-10f || u + v > 1.0f + 1e-10f) {
                    continue;
                }
                float along = dot3(e2, q) / det;
                if (along > 1e-10f) {
                    if (count >= 128u) {
                        return make_uint2(0u, 1u);
                    }
                    hits[count++] = along;
                }
            }
        } else {
            if (sp + 2u > 64u) {
                return make_uint2(0u, 1u);
            }
            stack[sp] = node_left(nodes, n);
            stack[sp + 1u] = node_right(nodes, n);
            sp += 2u;
        }
    }
    for (unsigned i = 1u; i < count; i++) {
        float key = hits[i];
        unsigned j = i;
        while (j > 0u && hits[j - 1u] > key) {
            hits[j] = hits[j - 1u];
            j--;
        }
        hits[j] = key;
    }
    unsigned unique = 0u;
    for (unsigned i = 0u; i < count; i++) {
        if (i == 0u || fabsf(hits[i] - hits[i - 1u]) >= 1e-8f * (1.0f + fmaxf(fabsf(hits[i]), fabsf(hits[i - 1u])))) {
            unique++;
        }
    }
    return make_uint2(unique, 0u);
}

__device__ float signed_distance(float3 p, const float* nodes, const float* tris) {
    float d2 = nearest2(p, nodes, tris);
    if (d2 < 0.0f) {
        return __uint_as_float(0x7fc00000u);
    }
    float distance = sqrtf(d2);
    if (distance < 1e-12f) {
        return 0.0f;
    }
    uint2 h = ray_hits(p, nodes, tris);
    if (h.y > 0u) {
        return __uint_as_float(0x7fc00000u);
    }
    return (h.x % 2u == 1u) ? -distance : distance;
}

extern "C" __global__ void lattice_sample(
    unsigned nx,
    unsigned ny,
    unsigned nz,
    unsigned n_segments,
    unsigned n_nodes,
    unsigned organic,
    unsigned open_top,
    unsigned keep_core,
    float min_x,
    float min_y,
    float min_z,
    float step_x,
    float step_y,
    float step_z,
    float skin,
    float radius_blend,
    float wall_depth,
    float top_z,
    const float* __restrict__ nodes,
    const float* __restrict__ tris,
    const float* __restrict__ segments,
    float* __restrict__ values
) {
    unsigned id = blockIdx.x * blockDim.x + threadIdx.x;
    unsigned row = nx + 1u;
    unsigned total = row * (ny + 1u) * (nz + 1u);
    if (id >= total || n_nodes == 0u) {
        return;
    }
    unsigned x = id % row;
    unsigned y = (id / row) % (ny + 1u);
    unsigned z = id / (row * (ny + 1u));
    float3 p = make_float3(
        min_x + float(x) * step_x,
        min_y + float(y) * step_y,
        min_z + float(z) * step_z
    );
    float source = signed_distance(p, nodes, tris);
    if (isnan(source)) {
        values[id] = __uint_as_float(0x7fc00000u);
        return;
    }
    float graph = 3.402823466e38f;
    for (unsigned s = 0u; s < n_segments; s++) {
        unsigned base = s * 8u;
        float3 a = make_float3(segments[base], segments[base + 1u], segments[base + 2u]);
        float3 d = make_float3(segments[base + 3u], segments[base + 4u], segments[base + 5u]);
        float length2 = segments[base + 6u];
        float r = segments[base + 7u];
        float3 q = sub3(p, a);
        float t = clampf(dot3(q, d) / length2, 0.0f, 1.0f);
        float3 delta = sub3(q, mul3(d, t));
        float distance = sqrtf(dot3(delta, delta)) - r;
        if (organic > 0u) {
            float h = fmaxf((radius_blend - fabsf(graph - distance)) / radius_blend, 0.0f);
            graph = fminf(graph, distance) - h * h * radius_blend / 4.0f;
        } else {
            graph = fminf(graph, distance);
        }
    }
    float skin_field = 3.402823466e38f;
    if (skin > 0.0f) {
        float cap = -3.402823466e38f;
        if (open_top > 0u) {
            cap = p.z - (top_z - skin);
        }
        skin_field = fmaxf(-source - skin, cap);
    }
    float material = fminf(graph, skin_field);
    if (wall_depth > 0.0f) {
        float band = -source - wall_depth;
        float walls = fmaxf(material, band);
        if (keep_core > 0u) {
            walls = fminf(walls, source + wall_depth);
        }
        material = walls;
    }
    values[id] = fmaxf(source, material);
}
