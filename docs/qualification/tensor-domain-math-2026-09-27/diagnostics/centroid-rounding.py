"""CPU IEEE-f32 emulation of observed midpoint reconstruction cancellation.
No GPU execution; no assertion about all backend reduction orders.
"""
import struct

def f32(x):
    return struct.unpack('f', struct.pack('f', x))[0]

def next_f32(x):
    bits = struct.unpack('I', struct.pack('f', x))[0]
    return struct.unpack('f', struct.pack('I', bits + 1))[0]

a = f32(1e20)
half = f32(a * 0.5)
scaled_mean = f32(-511 / 513)
expected = a / 513
ulp = next_f32(f32(expected)) - f32(expected)
print('Scope: CPU f32 emulation, not native execution')
print(f'sparse count=513 nonzero={a!r} expected_mean_f64={expected!r}')
for fused in (False, True):
    # These particular f32 operands' exact product/add fit the f64 emulator.
    got = f32(half + half * scaled_mean) if fused else f32(half + f32(half * scaled_mean))
    print(f'fused={fused} mean={got!r} error_ULPs={(got-expected)/ulp!r} relative_error={(got-expected)/expected!r}')

values = [f32(f32((i % 17 - 8) / 8) * f32(1e18)) for i in range(513)]
mean = sum(values) / len(values)
mean_abs = sum(abs(x) for x in values) / len(values)
observed = f32(-5116930600000000.0)  # MLX residual-refinement diagnostic log.
print(f'anisotropic mean_f64={mean!r} mean_abs_f64={mean_abs!r} condition={mean_abs/abs(mean)!r}')
print(f'observed_MLX_error_to_mean_abs={abs(observed-mean)/mean_abs!r}; bound=3e-6*mean_abs={3e-6*mean_abs!r}')
print('All-positive sparse data have mean_abs == mean, retaining the original sparse check.')
