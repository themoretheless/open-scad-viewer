struct Params { count: u32, op: u32, _pad0: u32, _pad1: u32 };
@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> input: array<f32>;
@group(0) @binding(2) var<storage, read_write> output: array<f32>;
const WG: u32 = 256;
@compute @workgroup_size(WG)
fn main(@builtin(global_invocation_id) gid: vec3<u32>, @builtin(num_workgroups) groups: vec3<u32>) {
    let stride = groups.x * WG;
    let count = params.count;
    switch params.op {
        case 0u: {
            for (var i = gid.x; i < count; i += stride) { output[i] = -input[i]; }
        }
        case 1u: {
            for (var i = gid.x; i < count; i += stride) { output[i] = abs(input[i]); }
        }
        case 2u: {
            for (var i = gid.x; i < count; i += stride) {
                let x = input[i];
                output[i] = x * x;
            }
        }
        case 3u: {
            for (var i = gid.x; i < count; i += stride) { output[i] = sqrt(input[i]); }
        }
        case 4u: {
            for (var i = gid.x; i < count; i += stride) { output[i] = 1.0 / input[i]; }
        }
        case 5u: {
            for (var i = gid.x; i < count; i += stride) { output[i] = exp(input[i]); }
        }
        case 6u: {
            for (var i = gid.x; i < count; i += stride) { output[i] = log(input[i]); }
        }
        case 7u: {
            for (var i = gid.x; i < count; i += stride) { output[i] = sin(input[i]); }
        }
        case 8u: {
            for (var i = gid.x; i < count; i += stride) { output[i] = cos(input[i]); }
        }
        default: {
            for (var i = gid.x; i < count; i += stride) { output[i] = 0.0; }
        }
    }
}
