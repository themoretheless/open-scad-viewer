# Metal SIMD-group float matrix probe

Executed on MLX 0.32.1, Apple M4 Max, Metal, 2026-09-27.
The source constructs fresh custom-kernel outputs for three replays and checks
an 8×8 float matrix product against independent host f64 dots. Inputs are raw
u32 representations of f32 values, loaded by bitcast into shared float tiles.
This isolates the float SIMD-group matrix path from native low-array decoding.

| Case | Checks per replay | Observed result |
| --- | ---: | --- |
| Ordinary exact dyadic values | 64 | Exact |
| Minimum positive f16 × maximum finite f16 | 64 | Exact |
| Minimum BF16 subnormal on the left × maximum finite BF16 | 64 | All zero; expected 0.2490234375 |
| Minimum BF16 subnormal on the right × maximum finite BF16 | 64 | All zero; expected 0.2490234375 |
| Maximum BF16 subnormal on the left × maximum finite BF16 | 64 | All zero; expected 31.6259765625 |

All three replays had the same outcome. The ordinary checks are pass gates;
BF16 subnormal cases are observations and do not make the probe exit nonzero.
The tested SIMD-group path therefore needs protection before use for the
MLX backend's tiny-BF16 normal-product cases. The probe does not distinguish
which individual load/multiply/compiler stage drops the tiny input, and it
makes no timing or machine-instruction claim. Maximum-subnormal BF16 was
probed on the left only; minimum subnormal was probed in both operand orders.

The installed MLX 0.32.1 GEMM header uses float SIMD-group fragments for its
low-input accumulation: `BlockMMA` defaults `AccumType` to float, casts tiled
input elements into that type, then calls `simdgroup_multiply_accumulate`.
This suggested the candidate under investigation.
[Pinned upstream source](https://github.com/ml-explore/mlx/blob/v0.32.1/mlx/backend/metal/kernels/steel/gemm/mma.h).
The inspected local file was `/opt/homebrew/Cellar/mlx/0.32.1/include/mlx/backend/metal/kernels/steel/gemm/mma.h`.

## Reproduce

Run from the repository root with the external Homebrew MLX-C SDK available:

```sh
clang -std=c11 -O2 -Wall -Wextra -Werror \
  -I/opt/homebrew/opt/mlx-c/include \
  docs/qualification/tensor-low-matmul-2026-09-27/simdgroup-probe/probe.c \
  -L/opt/homebrew/opt/mlx-c/lib -lmlxc -o /tmp/low-matmul-simdgroup-probe
/tmp/low-matmul-simdgroup-probe
```

[Source](probe.c), [build command/output](build.txt), [native output](run-metal.txt).
Metal access is required. Missing hardware is an unavailable probe, not a
passing numerical check. The SDK is required only for this standalone research
probe; the Rust adapter dynamically loads MLX-C through its existing ABI.
