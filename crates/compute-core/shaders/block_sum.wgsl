// Workgroup-strided sum reduction: partials[wid] = sum of input[wid*WG .. wid*WG+WG).
// Callers reduce the partials array (a second dispatch, or a tiny CPU fold)
// to obtain the full sum; chaining dispatches scales to arbitrary lengths.
struct Params {
    count: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
};

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> input: array<f32>;
@group(0) @binding(2) var<storage, read_write> partials: array<f32>;

// WG must stay a power of two: the tree reduction halves the stride.
const WG: u32 = 256;

var<workgroup> scratch: array<f32, WG>;

@compute @workgroup_size(WG)
fn main(
    @builtin(local_invocation_id) lid: vec3<u32>,
    @builtin(workgroup_id) wid: vec3<u32>,
) {
    let i = wid.x * WG + lid.x;
    var sum = 0.0;
    if (i < params.count) {
        sum = input[i];
    }
    scratch[lid.x] = sum;
    for (var stride = WG / 2u; stride > 0u; stride = stride >> 1u) {
        workgroupBarrier();
        if (lid.x < stride) {
            scratch[lid.x] = scratch[lid.x] + scratch[lid.x + stride];
        }
    }
    if (lid.x == 0u) {
        partials[wid.x] = scratch[0];
    }
}
