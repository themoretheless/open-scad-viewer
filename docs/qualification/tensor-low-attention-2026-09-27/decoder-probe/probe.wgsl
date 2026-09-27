@group(0) @binding(0) var<storage, read> packed: array<u32>;
@group(0) @binding(1) var<storage, read> addresses: array<u32>;
@group(0) @binding(2) var<storage, read_write> output: array<u32>;
fn exact(bits: u32) -> u32 {
    let sign = (bits & 0x8000u) << 16u;
    let exponent = (bits >> 10u) & 31u;
    let mantissa = bits & 1023u;
    if exponent == 31u { return sign | 0x7f800000u | (mantissa << 13u); }
    if exponent != 0u { return sign | ((exponent + 112u) << 23u) | (mantissa << 13u); }
    if mantissa == 0u { return sign; }
    let highest = 31u - countLeadingZeros(mantissa);
    return sign | ((highest + 103u) << 23u) | ((mantissa << (23u - highest)) & 0x7fffffu);
}
fn guarded(bits: u32) -> u32 {
    let exponent = bits & 0x7c00u;
    if exponent == 0u || exponent == 0x7c00u { return exact(bits); }
    return bitcast<u32>(unpack2x16float(bits).x);
}
@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if i >= arrayLength(&addresses) { return; }
    let address = addresses[i];
    let word = packed[address / 2u];
    let bits = (word >> ((address % 2u) * 16u)) & 0xffffu;
    output[i*4u] = bitcast<u32>(unpack2x16float(word)[address % 2u]);
    output[i*4u+1u] = bitcast<u32>(unpack2x16float(bits).x);
    output[i*4u+2u] = guarded(bits);
    output[i*4u+3u] = exact(bits);
}
