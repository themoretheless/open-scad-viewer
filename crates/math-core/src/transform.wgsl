// Batch `q = M*p + t` compute shader. One thread per point; mirrors the CUDA
// port `transform_points.cu` (crates/math-core/src/transform.cu). f32
// arithmetic, like every other GPU/CUDA kernel in this workspace.
struct Params {
    m0: vec3<f32>,
    m1: vec3<f32>,
    m2: vec3<f32>,
    t: vec3<f32>,
    count: u32,
};

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> points_in: array<f32>;
@group(0) @binding(2) var<storage, read_write> points_out: array<f32>;

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i >= params.count) {
        return;
    }
    let base = i * 3u;
    let p = vec3<f32>(points_in[base], points_in[base + 1u], points_in[base + 2u]);
    let q = vec3<f32>(dot(params.m0, p), dot(params.m1, p), dot(params.m2, p)) + params.t;
    points_out[base] = q.x;
    points_out[base + 1u] = q.y;
    points_out[base + 2u] = q.z;
}
