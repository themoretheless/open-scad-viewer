"""Exact positive common-weight scaling preserves a rational spline map."""
from fractions import Fraction


def positive_proportional_weights(actual, expected):
    if not actual or len(actual) != len(expected):
        return False
    a = [Fraction(x) for x in actual]
    b = [Fraction(x) for x in expected]
    return all(x > 0 and y > 0 and x*b[0] == y*a[0] for x, y in zip(a, b))


if __name__ == '__main__':
    assert positive_proportional_weights([1]*6, [2]*6)
    assert positive_proportional_weights([1, 2, 4], [2, 4, 8])
    assert not positive_proportional_weights([1, 2, 4.000000000000001], [2, 4, 8])
    assert not positive_proportional_weights([-1, -2], [1, 2])
    assert not positive_proportional_weights([1], [1, 1])
    print('Exact rational weight-scaling tests passed')
