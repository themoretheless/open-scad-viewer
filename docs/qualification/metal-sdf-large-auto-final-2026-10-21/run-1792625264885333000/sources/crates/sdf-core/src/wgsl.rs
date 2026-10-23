//! WGSL compute shader for SDF grid sampling, shared between native `wgpu` and browser WebGPU.

/// The grid-sampling compute shader (WGSL), shared by the native `gpu` feature
/// and the browser WebGPU host path; both must execute the identical text.
pub const SDF_WGSL: &str = r##"
struct Params {
    nx: u32,
    ny: u32,
    nz: u32,
    n_nodes: u32,
    min_x: f32,
    min_y: f32,
    min_z: f32,
    step_x: f32,
    step_y: f32,
    step_z: f32,
}
@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> kinds: array<u32>;
@group(0) @binding(2) var<storage, read> node_params: array<f32>;
@group(0) @binding(3) var<storage, read> aux: array<u32>;
@group(0) @binding(4) var<storage, read> tris: array<f32>;
@group(0) @binding(5) var<storage, read_write> values: array<f32>;

fn closest_triangle(p: vec3f, a: vec3f, b: vec3f, c: vec3f) -> vec3f {
    let ab = b - a;
    let ac = c - a;
    let n = cross(ab, ac);
    let nn = dot(n, n);
    if (nn > 0.0) {
        let q = p - n * (dot(p - a, n) / nn);
        let aq = q - a;
        let v = dot(cross(aq, ac), n) / nn;
        let w = dot(cross(ab, aq), n) / nn;
        if (v >= 0.0 && w >= 0.0 && v + w <= 1.0) {
            return q;
        }
    }
    var best = a;
    var best_dist = 3.402823466e+38;
    let edges = array<vec3f, 6>(a, b, b, c, c, a);
    for (var e = 0u; e < 3u; e++) {
        let ea = edges[e * 2u];
        let eb = edges[e * 2u + 1u];
        let d = eb - ea;
        var t = 0.0;
        if (dot(d, d) > 0.0) {
            t = clamp(dot(p - ea, d) / dot(d, d), 0.0, 1.0);
        }
        let q = ea + t * d;
        let dist_edge = distance(p, q);
        if (dist_edge < best_dist) {
            best = q;
            best_dist = dist_edge;
        }
    }
    return best;
}

// `WG` is the workgroup-size anchor: the compute-core runtime may substitute
// a per-backend tuned value (powers of two only) before compilation.
const WG: u32 = 256;

// Flat postorder field tree: leaves push, CSG/offset ops combine with a value
// stack. Node kinds match sdf-core/src/flat.rs.
@compute @workgroup_size(WG)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let row = params.nx + 1u;
    let total = row * (params.ny + 1u) * (params.nz + 1u);
    if (id.x >= total) {
        return;
    }
    let x = idx_decompose_x(id.x, row);
    let y = (id.x / row) % (params.ny + 1u);
    let z = id.x / (row * (params.ny + 1u));
    let p = vec3f(
        params.min_x + f32(x) * params.step_x,
        params.min_y + f32(y) * params.step_y,
        params.min_z + f32(z) * params.step_z,
    );
    var stack: array<f32, 33>;
    var sp = 0u;
    for (var n = 0u; n < params.n_nodes; n++) {
        let base = n * 8u;
        let kind = kinds[n];
        if (kind <= 2u || kind == 10u || (kind >= 12u && kind <= 15u)) {
            // Leaves: sphere / box / torus / capsule / cylinder / TPMS with translate-folded parameters.
            var v = 0.0;
            if (kind == 10u) {
                let a = vec3f(node_params[base], node_params[base + 1u], node_params[base + 2u]);
                let b = vec3f(node_params[base + 3u], node_params[base + 4u], node_params[base + 5u]);
                let ab = b - a;
                let ap = p - a;
                var t = 0.0;
                let denom = dot(ab, ab);
                if (denom > 0.0) {
                    t = clamp(dot(ap, ab) / denom, 0.0, 1.0);
                }
                v = length(ap - ab * t) - node_params[base + 6u];
            } else if (kind == 0u) {
                let c = vec3f(node_params[base], node_params[base + 1u], node_params[base + 2u]);
                v = length(p - c) - node_params[base + 3u];
            } else if (kind == 1u) {
                let c = vec3f(node_params[base], node_params[base + 1u], node_params[base + 2u]);
                let h = vec3f(node_params[base + 3u], node_params[base + 4u], node_params[base + 5u]);
                let q = abs(p - c) - h;
                v = length(max(q, vec3f(0.0))) + min(max(q.x, max(q.y, q.z)), 0.0);
            } else if (kind == 2u) {
                let c = vec3f(node_params[base], node_params[base + 1u], node_params[base + 2u]);
                let q = p - c;
                v = length(vec2f(length(q.xy) - node_params[base + 3u], q.z)) - node_params[base + 4u];
            } else if (kind == 12u) {
                let c = vec3f(node_params[base], node_params[base + 1u], node_params[base + 2u]);
                let q = p - c;
                let d = vec2f(length(q.xy) - node_params[base + 3u], abs(q.z) - node_params[base + 4u]);
                v = min(max(d.x, d.y), 0.0) + length(max(d, vec2f(0.0)));
            } else {
                let c = vec3f(node_params[base], node_params[base + 1u], node_params[base + 2u]);
                let period = node_params[base + 3u];
                let thickness = node_params[base + 4u];
                let k = 6.283185307179586 / period;
                let q = (p - c) * k;
                let sx = sin(q.x);
                let sy = sin(q.y);
                let sz = sin(q.z);
                let cx = cos(q.x);
                let cy = cos(q.y);
                let cz = cos(q.z);
                var iso = 0.0;
                if (kind == 13u) {
                    iso = sx * cy + sy * cz + sz * cx;
                } else if (kind == 14u) {
                    iso = cx + cy + cz;
                } else {
                    iso = sx * sy * sz + sx * cy * cz + cx * sy * cz + cx * cy * sz;
                }
                v = abs(iso) / k - 0.5 * thickness;
            }
            stack[sp] = v;
            sp++;
        } else if (kind == 8u || kind == 9u) {
            // Mesh distance: brute-force closest point (and solid angle for
            // signed) over the referenced triangle window, in scan order.
            let start = aux[n * 2u];
            let count = aux[n * 2u + 1u];
            var best = 3.402823466e+38;
            var angle = 0.0;
            for (var t = 0u; t < count; t++) {
                let base_t = (start + t) * 9u;
                let a = vec3f(tris[base_t], tris[base_t + 1u], tris[base_t + 2u]);
                let b = vec3f(tris[base_t + 3u], tris[base_t + 4u], tris[base_t + 5u]);
                let c = vec3f(tris[base_t + 6u], tris[base_t + 7u], tris[base_t + 8u]);
                best = min(best, distance(p, closest_triangle(p, a, b, c)));
                if (kind == 9u) {
                    let qa = a - p;
                    let qb = b - p;
                    let qc = c - p;
                    let la = length(qa);
                    let lb = length(qb);
                    let lc = length(qc);
                    angle += 2.0 * atan2(
                        dot(qa, cross(qb, qc)),
                        la * lb * lc + dot(qa, qb) * lc + dot(qb, qc) * la + dot(qc, qa) * lb,
                    );
                }
            }
            var v = best;
            if (kind == 9u && best > 0.0 && abs(angle) > 6.283185307) {
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
            let b = stack[sp];
            sp--;
            let a = stack[sp];
            var v = 0.0;
            if (kind == 3u) {
                v = min(a, b);
            } else if (kind == 4u) {
                v = max(a, b);
            } else if (kind == 5u) {
                v = max(a, -b);
            } else if (kind == 11u) {
                let k = node_params[base];
                let na = -a;
                let h = clamp(0.5 + 0.5 * (b - na) / k, 0.0, 1.0);
                v = -(b * (1.0 - h) + na * h - k * h * (1.0 - h));
            } else {
                let k = node_params[base];
                let h = clamp(0.5 + 0.5 * (b - a) / k, 0.0, 1.0);
                v = b * (1.0 - h) + a * h - k * h * (1.0 - h);
            }
            stack[sp] = v;
            sp++;
        }
    }
    values[id.x] = stack[0];
}
fn idx_decompose_x(idx: u32, row: u32) -> u32 {
    return idx % row;
}
"##;
