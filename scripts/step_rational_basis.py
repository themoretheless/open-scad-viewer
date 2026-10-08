"""Exact projective weight identity for independent STEP qualification."""
from fractions import Fraction
import math


def same_positive_projective_weights(a, b):
    """A single positive common scale cancels from the rational denominator.

    Compare exact binary64 values, never an epsilon or a rounded ratio.
    This proves no knot, pole, parametrization or topological identity.
    """
    if len(a) != len(b) or not a:
        return False
    if any(not math.isfinite(x) or x <= 0 for x in [*a, *b]):
        return False
    aa, bb = [Fraction(x) for x in a], [Fraction(x) for x in b]
    return all(x * bb[0] == y * aa[0] for x, y in zip(aa, bb))
