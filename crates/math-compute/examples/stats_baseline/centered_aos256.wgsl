// Second pass: residual sums and products about the first pass's centroid.
// The first six zero fields let the shared 15-f32 statistics fold be reused.
struct Params { count: u32, _pad0: u32, _pad1: u32, _pad2: u32 };
@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> points: array<f32>;
@group(0) @binding(2) var<storage, read> statistics: array<f32>;
@group(0) @binding(3) var<storage, read_write> partials: array<f32>;
const WG: u32 = 256;
var<workgroup> values: array<array<f32, 9>, WG>;
@compute @workgroup_size(WG)
fn main(
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(local_invocation_id) lid: vec3<u32>,
    @builtin(workgroup_id) wid: vec3<u32>,
) {
    var d = vec3<f32>(0.0);
    if gid.x < params.count {
        let base = gid.x * 3u;
        d = vec3<f32>(points[base], points[base + 1u], points[base + 2u])
            - vec3<f32>(statistics[15], statistics[16], statistics[17]);
    }
    values[lid.x] = array<f32, 9>(d.x, d.y, d.z,
        d.x * d.x, d.x * d.y, d.x * d.z, d.y * d.y, d.y * d.z, d.z * d.z);
    workgroupBarrier();
    var stride = WG / 2u;
    loop {
        if stride == 0u { break; }
        if lid.x < stride {
            for (var field = 0u; field < 9u; field++) {
                values[lid.x][field] += values[lid.x + stride][field];
            }
        }
        workgroupBarrier();
        stride /= 2u;
    }
    if lid.x == 0u {
        let base = wid.x * 15u;
        for (var field = 0u; field < 6u; field++) { partials[base + field] = 0.0; }
        for (var field = 0u; field < 9u; field++) {
            partials[base + 6u + field] = values[0][field];
        }
    }
}
