#!/usr/bin/env python3
"""CPU emulation audit of the corrected WGSL power-of-two helper formulas.

Run from any directory with Python 3; no third-party packages or GPU are used.
The deterministic corpus/seed is the same as the earlier read-only audit.
Explicit ties/carry checks supplement that corpus in this saved reproduction.

Scope: emulate stats_down_bits and stats_up64 integer formulas, then compare
against exact binary scaling in Python f64 followed by IEEE f32 rounding.
Every original f32 value and its scaled value fit exactly in f64, including
the smallest downshifted value (2^-277). This does not execute WGSL, compile
Metal, or prove GPU behavior under reassociation/FTZ. The shader fingerprint
identifies the inspected source; Python does not parse its implementation.
"""

import hashlib
import math
from pathlib import Path
import platform
import random
import struct


def as_f32(bits):
    return struct.unpack("<f", struct.pack("<I", bits))[0]


def as_bits(value):
    return struct.unpack("<I", struct.pack("<f", value))[0]


def down_bits(bits, power):
    """Finite inputs; the production call sites use powers 1, 64 and 128."""
    assert ((bits >> 23) & 255) != 255
    assert power in (1, 64, 128)
    sign = bits & 0x80000000
    exponent = (bits >> 23) & 255
    if exponent > power:
        return bits - (power << 23)
    shift = power if exponent == 0 else power + 1 - exponent
    if shift > 24:
        return sign
    significand = (bits & 0x7FFFFF) | (0 if exponent == 0 else 0x800000)
    value = significand >> shift
    remainder = significand & ((1 << shift) - 1)
    midpoint = 1 << (shift - 1)
    round_up = remainder > midpoint or (remainder == midpoint and value & 1)
    return sign | (value + int(round_up))


def up64(bits):
    """Finite inputs with |x| < 2^-64, including signed zero/subnormals."""
    assert (bits & 0x7FFFFFFF) < 0x1F800000
    sign = bits & 0x80000000
    exponent = (bits >> 23) & 255
    if exponent:
        return bits + (64 << 23)
    mantissa = bits & 0x7FFFFF
    if not mantissa:
        return bits
    leading = mantissa.bit_length() - 1
    return (
        sign
        | ((leading + 42) << 23)
        | ((mantissa << (23 - leading)) & 0x7FFFFF)
    )


def main():
    root = Path(__file__).resolve().parents[3]
    relative = Path("crates/compute-core/shaders/tensor_stats_common.wgsl")
    source_hash = hashlib.sha256((root / relative).read_bytes()).hexdigest()
    print("CPU emulation of statistics power-of-two helper formulas")
    print(f"Python: {platform.python_version()}")
    print(f"Inspected shader: {relative}")
    print(f"SHA256: {source_hash}")

    rng = random.Random(27092026)
    inputs = [rng.randrange(0xFF000000) for _ in range(100000)]
    inputs += [
        sign | (exponent << 23) | mantissa
        for sign in (0, 0x80000000)
        for exponent in range(255)
        for mantissa in (0, 1, 2, 0x3FFFFF, 0x400000, 0x7FFFFE, 0x7FFFFF)
    ]
    inputs = [bits for bits in inputs if ((bits >> 23) & 255) != 255]
    for power in (1, 64, 128):
        for bits in inputs:
            expected = as_bits(math.ldexp(as_f32(bits), -power))
            assert down_bits(bits, power) == expected, (hex(bits), power)
    print(f"PASS: {len(inputs):,} finite encodings x downshift powers 1/64/128")

    bounded = [bits for bits in inputs if (bits & 0x7FFFFFFF) < 0x1F800000]
    for bits in bounded:
        assert up64(bits) == as_bits(math.ldexp(as_f32(bits), 64)), hex(bits)
    print(f"PASS: {len(bounded):,} encodings with |x| < 2^-64, shifted up by 64")

    ties = 0
    for sign in (0, 0x80000000):
        for power in (1, 64, 128):
            # A one-bit shift into subnormal representation: tie-to-even down,
            # tie-to-even up, and the rounded carry into the smallest normal.
            for mantissa, expected in (
                (1, 0x400000),
                (3, 0x400002),
                (0x7FFFFF, 0x800000),
            ):
                assert down_bits(sign | (power << 23) | mantissa, power) == sign | expected
                ties += 1
        for original, expected in ((0, 0), (1, 0), (3, 2), (0x7FFFFF, 0x400000)):
            assert down_bits(sign | original, 1) == sign | expected
            ties += 1
        for power in (64, 128):
            # Half of the least subnormal rounds to signed zero; the next
            # source significand above that midpoint rounds to min-subnormal.
            exponent = power - 23
            assert down_bits(sign | (exponent << 23), power) == sign
            assert down_bits(sign | (exponent << 23) | 1, power) == sign | 1
            ties += 2
    print(f"PASS: {ties} explicit ties/carry/signed-zero boundary assertions")
    print("Scope: CPU formulas only; no WGSL/Metal execution or GPU qualification.")


if __name__ == "__main__":
    main()
