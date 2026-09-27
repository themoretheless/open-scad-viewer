F16 attention decode investigation, 2026-09-27

Primary sources:
https://www.w3.org/TR/WGSL/#unpack2x16float-builtin (17.10.7)
https://www.w3.org/TR/WGSL/#floating-point-evaluation (15.7.2, 15.7.6)
WGSL decodes binary16 then converts to f32; intermediate subnormal values in
unpacking builtins may be flushed. Therefore normal f32 output range alone does
not establish preservation of half subnormals on every conforming backend.

https://developer.apple.com/metal/Metal-Shading-Language-Specification.pdf
Apple Metal Shading Language4.1, 2026-06-04, section8.6, printedpage376:
half and bfloat to float are lossless; fast math does not alter conversion
accuracy. This supports testing the actual Metal implementation, not assuming
portable WGSL preservation.

Local Naga30.0.1 src/back/msl/writer.rs:2827-2831 emits:
float2(as_type<half2>(operand))
for Unpack2x16float.

Candidate: extract selected16bits first, use unpack2x16float(bits16).x only if
exponent!=0 && exponent!=31. For exponent0/31 use exact integer decoder.
Upper16bits arezero, avoiding effects from unused adjacent raw payloads.

Probe: all65,536binary16 encodings, both selected halfword parities,9neighbor
payloads (zeros, smallestsubnormal,1,maxfinite,±Inf,qNaN,sNaN), nonzero physical
word offset with sentinels. Outputs: originalpacked builtin, selected builtin,
guarded selectedbuiltin, currentinteger decoder. CPUreference uses independent
f64 mathematical half interpretation. Finite results exactbits (incl±0), special
results classified separately. Three GPUreplays; source storage unchanged.
Rawbuiltin special mismatches are diagnostic; guardedfinite mismatches fail.
This is conversion qualification, not attention correctness or performance.

Files SHA256:
/tmp/low-attention-unpack-probe/probe.wgsl: db8c2f623a566e7afe08884d0451e8559f7393c59b070fc7425e96448837f454
/tmp/low-attention-unpack-probe/src/main.rs: 53b613251d872b6106c8a597024a8369a44ed5475cd0d4ef01b10d3dff2e7ff0
/Users/themoretheless/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/naga-30.0.1/src/back/msl/writer.rs: e7c9c2aafb3f0175eb18174fd2a06044a980947ed1069ce20709f07e962122d2
