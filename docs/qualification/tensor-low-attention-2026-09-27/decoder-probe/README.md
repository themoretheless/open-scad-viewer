# F16 decoder probe

Actual Metal run on 2026-09-27: all four decoder variants passed three replays
with no finite, subnormal, infinity or NaN-classification mismatches. The input
allocation remained unchanged. This checks conversion correctness on this
machine; it contains no performance measurement.

## Preserved execution

- [probe.wgsl](probe.wgsl) and [src/main.rs](src/main.rs) are the exact executed source.
- [Cargo.executed.toml](Cargo.executed.toml) is the exact manifest used under
  `/tmp/low-attention-unpack-probe`, including its original absolute dependency.
- [Cargo.toml](Cargo.toml) changes only that dependency path to the equivalent
  relative workspace path. This portable manifest was not used for the retained run.
- [Cargo.lock](Cargo.lock) and [build.txt](build.txt) preserve dependency resolution
  and the successful build output.
- [run.txt](run.txt) is the initial sandbox attempt: GPU unavailable, exit 1.
  It is **not** a numerical test result.
- [run-metal.txt](run-metal.txt) is the actual Metal execution outside the sandbox,
  exit 0. [README.txt](README.txt) preserves the original investigation notes and hashes.
- [artifact-hashes.json](artifact-hashes.json) fingerprints these preserved files.

## Coverage

Each replay checks **65,536 input encodings × 9 adjacent payloads × 2 selected
halfword positions = 1,179,648 cases**, through four decoder variants:
packed builtin, selected-half builtin, guarded selected-half builtin, integer codec.
The selected halfword lies at a nonzero physical offset, with sentinel words
around its packed word. Neighbors include signed zero, a subnormal, finite
values, infinities, and quiet/signaling NaNs.

Of the 65,536 encodings, **63,488 are finite** (61,440 normal, 2,046 nonzero
subnormal, 2 signed zeros); 2 are infinities and 2,046 are NaNs. Therefore each
replay exercises 1,142,784 finite cases, including 36,828 nonzero subnormal
cases. Finite outputs are compared bit-for-bit against an independent f64
mathematical decoder. NaNs require classification only. Guarded/integer errors
fail the probe; raw builtin outcomes are reported separately.

## Why the guarded path

The W3C Candidate Recommendation Draft of 21 September 2026 defines binary16 unpacking followed by f32 conversion, but permits flushing
subnormal intermediate values in unpacking operations. A normal f32 result does
not itself exclude an earlier half flush. See the W3C sections on
[unpacking](https://www.w3.org/TR/2026/CRD-WGSL-20260921/#unpack2x16float-builtin) and
[floating-point behavior](https://www.w3.org/TR/2026/CRD-WGSL-20260921/#floating-point-evaluation).

The [Metal Shading Language 4.1 specification](https://developer.apple.com/metal/Metal-Shading-Language-Specification.pdf),
§8.6, printed page 376, states that half-to-float conversion is lossless and
conversion accuracy is unaffected by fast math. Local Naga 30.0.1
`src/back/msl/writer.rs:2827–2831` lowers `unpack2x16float` to
`float2(as_type<half2>(operand))`.

The portable candidate extracts the selected 16 bits first. It uses
`unpack2x16float(bits16).x` for ordinary finite half values; exponent 0/31 uses
the integer codec. The builtin's unused upper half is zero. Thus subnormals,
zeros and special classification do not depend on the unpack implementation.
The successful unguarded Metal result does not establish a cross-backend guarantee.

## Reproduce

From the repository root, with a working GPU adapter:

```sh
cargo run --locked --manifest-path docs/qualification/tensor-low-attention-2026-09-27/decoder-probe/Cargo.toml --target-dir /tmp/low-attention-decoder-probe-target
```

The probe requires a GPU and exits with an error when none is available. On
this macOS host, the sandbox denied adapter access, so the retained native
execution required the approved unsandboxed GPU window. Runtime/build artifacts
should remain outside this qualification directory. Later workspace revisions
can change dependencies or runtime behavior; preserve new results separately.
