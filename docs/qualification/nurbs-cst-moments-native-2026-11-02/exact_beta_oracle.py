"""Independent x-space rational integrals; no NURBS/Bernstein code imported."""
from fractions import Fraction as F
from math import comb
import json


def beta(a, b):
    # Integral x**a (1-x)**b dx, a rational and b a nonnegative integer.
    return sum((F((-1)**j * comb(b, j)) / (a+j+1) for j in range(b+1)), F(0))


def multiply(a, b):
    result = [F(0)] * (len(a)+len(b)-1)
    for i, x in enumerate(a):
        for j, y in enumerate(b):
            result[i+j] += x*y
    return result


def moments(alpha, upper, lower):
    def integral(power, x_power):
        a, b = [F(1)], [F(1)]
        for _ in range(power):
            a, b = multiply(a, upper), multiply(b, lower)
        return sum(((x-y)*beta(alpha*power+i+x_power, power)
                    for i, (x, y) in enumerate(zip(a, b))), F(0))/power
    area, mx, my = integral(1, 0), integral(1, 1), integral(2, 0)
    raw = [integral(3, 0), integral(1, 2), integral(2, 1)]
    central = [raw[0]-my*my/area, raw[1]-mx*mx/area, raw[2]-mx*my/area]
    return {'origin': list(map(str, raw)), 'centroid': list(map(str, central))}


cases = {
    'symmetric': moments(F(1, 2), [F(1, 8)], [F(-1, 8)]),
    'cambered': moments(F(1, 2), [F(1, 4)], [F(1, 8)]),
    'integer': moments(F(1), [F(1, 2)], [F(1, 4)]),
    'nonconstant_degree25': moments(F(1, 2), [F(1, 8), F(11, 32)], [F(-1, 8), F(0)]),
}
assert cases['symmetric'] == {'origin': ['1/27720', '1/63', '0'], 'centroid': ['1/27720', '8/2205', '0']}
assert cases['cambered'] == {'origin': ['1/7920', '1/126', '1/1280'], 'centroid': ['767/64880640', '4/2205', '-1/17920']}
assert cases['integer'] == {'origin': ['1/3840', '1/80', '1/640'], 'centroid': ['1/38400', '1/480', '0']}
assert cases['nonconstant_degree25'] == {'origin': ['3527/17425408', '5/168', '55/43008'], 'centroid': ['9355010923/63523279339520', '487/84105', '9889/76554240']}
print(json.dumps(cases, indent=2))
